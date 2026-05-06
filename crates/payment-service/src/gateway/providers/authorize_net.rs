//! Authorize.Net payment gateway — Accept Hosted redirect flow.
//!
//! Ports the PHP `api/authorize` endpoint. Uses the Authorize.Net Merchant API
//! with:
//!   - `getHostedPaymentPageRequest` to mint a one-time token that the client
//!     exchanges for a hosted card-entry page.
//!   - `getTransactionDetailsRequest` to verify the captured transaction.
//!   - `createTransactionRequest` with `transactionType = refundTransaction`
//!     for refunds (requires the last 4 digits of the card).
//!   - Webhooks: HMAC-SHA512 signature verification per the official spec.
//!
//! Env vars:
//!   - `AUTHORIZE_NET_LOGIN_ID`      — API Login ID
//!   - `AUTHORIZE_NET_TRANSACTION_KEY` — Transaction Key
//!   - `AUTHORIZE_NET_SANDBOX`       — "true" (default) → sandbox endpoint
//!   - `AUTHORIZE_NET_WEBHOOK_SIGNATURE_KEY` — Signature key for webhook HMAC

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

pub struct AuthorizeNetGateway {
    login_id: String,
    transaction_key: String,
    webhook_signature_key: String,
    sandbox: bool,
    client: reqwest::Client,
}

impl AuthorizeNetGateway {
    pub fn from_env() -> Self {
        Self {
            login_id: std::env::var("AUTHORIZE_NET_LOGIN_ID").unwrap_or_default(),
            transaction_key: std::env::var("AUTHORIZE_NET_TRANSACTION_KEY").unwrap_or_default(),
            webhook_signature_key: std::env::var("AUTHORIZE_NET_WEBHOOK_SIGNATURE_KEY")
                .unwrap_or_default(),
            sandbox: std::env::var("AUTHORIZE_NET_SANDBOX").unwrap_or_else(|_| "true".into())
                == "true",
            client: reqwest::Client::new(),
        }
    }

    fn api_url(&self) -> &str {
        if self.sandbox {
            "https://apitest.authorize.net/xml/v1/request.api"
        } else {
            "https://api.authorize.net/xml/v1/request.api"
        }
    }

    fn hosted_page_url(&self) -> &str {
        if self.sandbox {
            "https://test.authorize.net/payment/payment"
        } else {
            "https://accept.authorize.net/payment/payment"
        }
    }

    fn auth_block(&self) -> Value {
        json!({
            "name": self.login_id,
            "transactionKey": self.transaction_key,
        })
    }

    /// Authorize.Net returns responses prefixed with a UTF-8 BOM (`\u{FEFF}`)
    /// that `serde_json` will refuse. Strip it before parsing.
    fn parse_json(raw: &str) -> Result<Value, PaymentError> {
        let trimmed = raw.trim_start_matches('\u{FEFF}').trim();
        serde_json::from_str::<Value>(trimmed)
            .map_err(|e| PaymentError::ProviderError(format!("Invalid Authorize.Net JSON: {e}")))
    }
}

#[async_trait]
impl PaymentGateway for AuthorizeNetGateway {
    fn provider_name(&self) -> &'static str {
        "authorize_net"
    }

    /// Create a hosted payment page token. The caller redirects the user to
    /// `hosted_page_url()` with the returned token as a form-POST field `token`.
    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let ref_id =
            params.metadata.get("order_id").cloned().unwrap_or_else(|| {
                uuid::Uuid::new_v4().to_string().replace('-', "")[..20].to_string()
            });

        let body = json!({
            "getHostedPaymentPageRequest": {
                "merchantAuthentication": self.auth_block(),
                "refId": ref_id,
                "transactionRequest": {
                    "transactionType": "authCaptureTransaction",
                    "amount": params.amount.to_string(),
                    "currencyCode": params.currency,
                    "order": {
                        "description": params.description,
                    }
                },
                "hostedPaymentSettings": {
                    "setting": [
                        {
                            "settingName": "hostedPaymentReturnOptions",
                            "settingValue": serde_json::to_string(&json!({
                                "showReceipt": true,
                                "url": params.return_url,
                                "urlText": "Continue",
                                "cancelUrl": params.cancel_url,
                                "cancelUrlText": "Cancel"
                            })).unwrap_or_default()
                        },
                        {
                            "settingName": "hostedPaymentButtonOptions",
                            "settingValue": r#"{"text":"Pay"}"#
                        },
                        {
                            "settingName": "hostedPaymentOrderOptions",
                            "settingValue": serde_json::to_string(&json!({
                                "show": true,
                                "merchantName": params.description,
                            })).unwrap_or_default()
                        }
                    ]
                }
            }
        });

        let resp = self.client.post(self.api_url()).json(&body).send().await?;
        let text = resp
            .text()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;
        let parsed = Self::parse_json(&text)?;

        let result_code = parsed["messages"]["resultCode"]
            .as_str()
            .unwrap_or_default();
        if result_code != "Ok" {
            let msg = parsed["messages"]["message"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|m| m["text"].as_str())
                .unwrap_or("Authorize.Net error")
                .to_string();
            return Err(PaymentError::ProviderError(msg));
        }

        let token = parsed["token"].as_str().unwrap_or("").to_string();
        if token.is_empty() {
            return Err(PaymentError::ProviderError(
                "Authorize.Net returned an empty token".into(),
            ));
        }

        Ok(PaymentSession {
            provider: "authorize_net".into(),
            session_id: ref_id.clone(),
            redirect_url: self.hosted_page_url().to_string(),
            provider_ref: Some(token),
        })
    }

    /// Verify a completed transaction by its `transId` (returned to the merchant
    /// via the `relay_response_url` or polled through the API).
    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let body = json!({
            "getTransactionDetailsRequest": {
                "merchantAuthentication": self.auth_block(),
                "transId": reference,
            }
        });

        let resp = self.client.post(self.api_url()).json(&body).send().await?;
        let text = resp
            .text()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;
        let parsed = Self::parse_json(&text)?;

        let result_code = parsed["messages"]["resultCode"]
            .as_str()
            .unwrap_or_default();
        if result_code != "Ok" {
            let error_code = parsed["messages"]["message"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|m| m["code"].as_str())
                .unwrap_or_default();
            if error_code == "E00040" {
                return Err(PaymentError::NotFound(format!(
                    "Transaction not found: {}",
                    reference
                )));
            }
            let msg = parsed["messages"]["message"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|m| m["text"].as_str())
                .unwrap_or("Authorize.Net error")
                .to_string();
            return Err(PaymentError::ProviderError(msg));
        }

        let tx = &parsed["transaction"];
        let status_str = tx["transactionStatus"].as_str().unwrap_or_default();

        let status = match status_str {
            "authorizedPendingCapture" | "capturedPendingSettlement" => PaymentStatusKind::Pending,
            "settledSuccessfully" => PaymentStatusKind::Completed,
            "refundPendingSettlement" | "refundSettledSuccessfully" => PaymentStatusKind::Refunded,
            "voided" | "declined" | "communicationError" | "settlementError" | "failedReview" => {
                PaymentStatusKind::Failed
            }
            _ => PaymentStatusKind::Pending,
        };

        let amount = tx["authAmount"]
            .as_str()
            .or_else(|| tx["settleAmount"].as_str())
            .and_then(|s| s.parse::<Decimal>().ok());

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount,
            currency: None,
        })
    }

    /// Verify an Authorize.Net webhook payload using HMAC-SHA512 over the raw
    /// body, compared against the `X-ANET-Signature` header value which arrives
    /// formatted as `sha512=<UPPERCASE_HEX>`.
    async fn handle_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        if !self.webhook_signature_key.is_empty() {
            crate::gateway::signature::verify_hmac_sha512_hex(
                self.webhook_signature_key.as_bytes(),
                payload,
                signature,
            )?;
        }

        let body = Self::parse_json(&String::from_utf8_lossy(payload))?;

        let event_type = body["eventType"].as_str().unwrap_or("").to_string();
        let provider_ref = body["payload"]["id"].as_str().unwrap_or("").to_string();
        let amount = body["payload"]["authAmount"]
            .as_f64()
            .and_then(|n| Decimal::try_from(n).ok());

        let status = match event_type.as_str() {
            s if s.contains("authcapture.created") || s.contains("capture.created") => {
                PaymentStatusKind::Completed
            }
            s if s.contains("void.created") || s.contains("decline") => PaymentStatusKind::Failed,
            s if s.contains("refund.created") => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref,
            status,
            amount,
            currency: None,
            metadata: HashMap::new(),
        })
    }

    /// Refund a settled transaction. The last 4 digits of the card must be
    /// supplied in `metadata` (key `card_last_four`) — Authorize.Net requires
    /// them for unreferenced refunds to succeed.
    async fn refund(
        &self,
        tx_id: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        // Look up the card last-four via transaction-details lookup.
        let lookup = json!({
            "getTransactionDetailsRequest": {
                "merchantAuthentication": self.auth_block(),
                "transId": tx_id,
            }
        });
        let lookup_resp = self
            .client
            .post(self.api_url())
            .json(&lookup)
            .send()
            .await?;
        let lookup_text = lookup_resp
            .text()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;
        let lookup_json = Self::parse_json(&lookup_text)?;

        let tx = &lookup_json["transaction"];
        let last_four = tx["payment"]["creditCard"]["cardNumber"]
            .as_str()
            .map(|s| s.trim_start_matches('X').to_string())
            .filter(|s| s.len() == 4)
            .ok_or_else(|| {
                PaymentError::RefundFailed(
                    "Could not determine card last-four from original transaction".into(),
                )
            })?;

        let refund_amount = amount
            .or_else(|| {
                tx["settleAmount"]
                    .as_str()
                    .and_then(|s| s.parse::<Decimal>().ok())
            })
            .ok_or_else(|| PaymentError::RefundFailed("Cannot determine refund amount".into()))?;

        let body = json!({
            "createTransactionRequest": {
                "merchantAuthentication": self.auth_block(),
                "transactionRequest": {
                    "transactionType": "refundTransaction",
                    "amount": refund_amount.to_string(),
                    "payment": {
                        "creditCard": {
                            "cardNumber": last_four,
                            "expirationDate": "XXXX"
                        }
                    },
                    "refTransId": tx_id,
                }
            }
        });

        let resp = self.client.post(self.api_url()).json(&body).send().await?;
        let text = resp
            .text()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;
        let parsed = Self::parse_json(&text)?;

        let result_code = parsed["messages"]["resultCode"]
            .as_str()
            .unwrap_or_default();
        if result_code != "Ok" {
            let msg = parsed["messages"]["message"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|m| m["text"].as_str())
                .unwrap_or("Refund failed")
                .to_string();
            return Err(PaymentError::RefundFailed(msg));
        }

        let trans_response = &parsed["transactionResponse"];
        let response_code = trans_response["responseCode"].as_str().unwrap_or_default();
        if response_code != "1" {
            let msg = trans_response["errors"]["error"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|e| e["errorText"].as_str())
                .unwrap_or("Refund declined")
                .to_string();
            return Err(PaymentError::RefundFailed(msg));
        }

        let refund_tx_id = trans_response["transId"]
            .as_str()
            .unwrap_or(tx_id)
            .to_string();

        Ok(RefundResult {
            provider_ref: refund_tx_id,
            refunded_amount: refund_amount,
            status: "refundPendingSettlement".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_strips_bom() {
        let raw = "\u{FEFF}{\"messages\":{\"resultCode\":\"Ok\"}}";
        let parsed = AuthorizeNetGateway::parse_json(raw).unwrap();
        assert_eq!(parsed["messages"]["resultCode"], "Ok");
    }

    #[tokio::test]
    async fn webhook_with_empty_signature_key_skips_verification() {
        // Safety net: when the merchant hasn't configured a signature key we
        // accept the payload as-is (useful for local dev).
        let gw = AuthorizeNetGateway {
            login_id: String::new(),
            transaction_key: String::new(),
            webhook_signature_key: String::new(),
            sandbox: true,
            client: reqwest::Client::new(),
        };
        let payload = br#"{"eventType":"net.authorize.payment.authcapture.created","payload":{"id":"60000000001","authAmount":10.50}}"#;
        let event = gw.handle_webhook(payload, "").await.unwrap();
        assert_eq!(
            event.event_type,
            "net.authorize.payment.authcapture.created"
        );
        assert_eq!(event.provider_ref, "60000000001");
        assert_eq!(event.status, PaymentStatusKind::Completed);
    }
}
