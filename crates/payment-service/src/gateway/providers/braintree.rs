use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STD};
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::signature::verify_braintree_signature;
use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

pub struct BraintreeGateway {
    merchant_id: String,
    public_key: String,
    private_key: String,
    sandbox: bool,
    client: reqwest::Client,
}

impl BraintreeGateway {
    pub fn from_env() -> Self {
        Self {
            merchant_id: std::env::var("BRAINTREE_MERCHANT_ID").unwrap_or_default(),
            public_key: std::env::var("BRAINTREE_PUBLIC_KEY").unwrap_or_default(),
            private_key: std::env::var("BRAINTREE_PRIVATE_KEY").unwrap_or_default(),
            sandbox: std::env::var("BRAINTREE_SANDBOX").unwrap_or_else(|_| "true".into()) == "true",
            client: reqwest::Client::new(),
        }
    }

    fn base_url(&self) -> &str {
        if self.sandbox {
            "https://api.sandbox.braintreegateway.com"
        } else {
            "https://api.braintreegateway.com"
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/merchants/{}{}", self.base_url(), self.merchant_id, path)
    }
}

#[async_trait]
impl PaymentGateway for BraintreeGateway {
    fn provider_name(&self) -> &'static str {
        "braintree"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let xml_body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <transaction>
                <type>sale</type>
                <amount>{}</amount>
                <order-id>{}</order-id>
            </transaction>"#,
            params.amount,
            params
                .metadata
                .get("order_id")
                .cloned()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        );

        let resp = self
            .client
            .post(self.api_url("/transactions"))
            .basic_auth(&self.public_key, Some(&self.private_key))
            .header("Content-Type", "application/xml")
            .body(xml_body)
            .send()
            .await?;

        let text = resp
            .text()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let tx_id = extract_xml_value(&text, "id").unwrap_or_default();

        Ok(PaymentSession {
            provider: "braintree".into(),
            session_id: tx_id.clone(),
            redirect_url: params.return_url,
            provider_ref: Some(tx_id),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let url = self.api_url(&format!("/transactions/{}", reference));
        let resp = self
            .client
            .get(&url)
            .basic_auth(&self.public_key, Some(&self.private_key))
            .header("Accept", "application/xml")
            .send()
            .await?;

        let text = resp
            .text()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status_str = extract_xml_value(&text, "status").unwrap_or_default();
        let status = match status_str.as_str() {
            "settled" | "settling" | "submitted_for_settlement" => PaymentStatusKind::Completed,
            "voided" | "gateway_rejected" => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        let amount = extract_xml_value(&text, "amount").and_then(|a| a.parse::<Decimal>().ok());

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount,
            currency: extract_xml_value(&text, "currency-iso-code"),
        })
    }

    /// Verify a Braintree webhook notification.
    ///
    /// Braintree delivers webhooks as `application/x-www-form-urlencoded`
    /// bodies with two fields: `bt_signature` and `bt_payload`. We first
    /// validate the HMAC-SHA1 signature (keyed with SHA1(private_key)), then
    /// base64-decode the payload and extract event metadata from the
    /// resulting XML.
    ///
    /// `_signature` is unused because Braintree does not use an HTTP header;
    /// the signature travels inside the form body itself.
    async fn handle_webhook(
        &self,
        payload: &[u8],
        _signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        let form_pairs: HashMap<String, String> =
            serde_urlencoded::from_bytes(payload).map_err(|e| {
                PaymentError::ProviderError(format!("Braintree webhook form decode: {e}"))
            })?;

        let bt_signature = form_pairs
            .get("bt_signature")
            .map(|s| s.as_str())
            .unwrap_or("");
        let bt_payload = form_pairs
            .get("bt_payload")
            .map(|s| s.as_str())
            .unwrap_or("");

        if bt_signature.is_empty() || bt_payload.is_empty() {
            return Err(PaymentError::InvalidSignature);
        }

        verify_braintree_signature(
            &self.public_key,
            &self.private_key,
            bt_signature,
            bt_payload,
        )?;

        // Payload is base64(XML) with newlines folded every 60 chars.
        let decoded = BASE64_STD
            .decode(bt_payload.replace(['\n', '\r'], ""))
            .map_err(|e| {
                PaymentError::ProviderError(format!("Braintree bt_payload base64 decode: {e}"))
            })?;
        let text = String::from_utf8_lossy(&decoded);

        let kind = extract_xml_value(&text, "kind").unwrap_or_default();

        let status = match kind.as_str() {
            k if k.starts_with("transaction_settled")
                || k.starts_with("transaction_disbursement") =>
            {
                PaymentStatusKind::Completed
            }
            k if k.starts_with("transaction_settlement_declined")
                || k.starts_with("transaction_review_accepted") =>
            {
                PaymentStatusKind::Completed
            }
            k if k.starts_with("transaction_disbursed") => PaymentStatusKind::Completed,
            k if k.ends_with("_failed")
                || k == "transaction_settlement_declined"
                || k == "dispute_opened" =>
            {
                PaymentStatusKind::Failed
            }
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: kind,
            provider_ref: extract_xml_value(&text, "id").unwrap_or_default(),
            status,
            amount: None,
            currency: None,
            metadata: HashMap::new(),
        })
    }

    async fn refund(
        &self,
        tx_id: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        let xml_body = if let Some(amt) = amount {
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><transaction><amount>{}</amount></transaction>"#,
                amt
            )
        } else {
            String::new()
        };

        let url = self.api_url(&format!("/transactions/{}/refund", tx_id));
        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.public_key, Some(&self.private_key))
            .header("Content-Type", "application/xml")
            .body(xml_body)
            .send()
            .await?;

        let text = resp
            .text()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        Ok(RefundResult {
            provider_ref: extract_xml_value(&text, "id").unwrap_or_else(|| tx_id.to_string()),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: extract_xml_value(&text, "status").unwrap_or_else(|| "submitted".into()),
        })
    }
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        let after = start + open.len();
        if let Some(end) = xml[after..].find(&close) {
            return Some(xml[after..after + end].trim().to_string());
        }
    }
    None
}
