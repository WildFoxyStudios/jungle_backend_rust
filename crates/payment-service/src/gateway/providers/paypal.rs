use async_trait::async_trait;
use rust_decimal::Decimal;
use serde_json::json;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

pub struct PayPalGateway {
    client_id: String,
    secret: String,
    base_url: String,
    client: reqwest::Client,
}

impl PayPalGateway {
    pub fn from_env() -> Self {
        let sandbox = std::env::var("PAYPAL_SANDBOX")
            .unwrap_or_else(|_| "true".into())
            .parse::<bool>()
            .unwrap_or(true);

        let base_url = if sandbox {
            "https://api-m.sandbox.paypal.com"
        } else {
            "https://api-m.paypal.com"
        };

        Self {
            client_id: std::env::var("PAYPAL_CLIENT_ID").unwrap_or_default(),
            secret: std::env::var("PAYPAL_SECRET").unwrap_or_default(),
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn get_access_token(&self) -> Result<String, PaymentError> {
        let resp = self
            .client
            .post(format!("{}/v1/oauth2/token", self.base_url))
            .basic_auth(&self.client_id, Some(&self.secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        body["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PaymentError::ProviderError("Failed to get PayPal token".into()))
    }
}

#[async_trait]
impl PaymentGateway for PayPalGateway {
    fn provider_name(&self) -> &'static str {
        "paypal"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let token = self.get_access_token().await?;

        let order = json!({
            "intent": "CAPTURE",
            "purchase_units": [{
                "amount": {
                    "currency_code": params.currency,
                    "value": params.amount.to_string()
                },
                "description": params.description
            }],
            "application_context": {
                "return_url": params.return_url,
                "cancel_url": params.cancel_url
            }
        });

        let resp = self
            .client
            .post(format!("{}/v2/checkout/orders", self.base_url))
            .bearer_auth(&token)
            .json(&order)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        let redirect_url = body["links"]
            .as_array()
            .and_then(|links| links.iter().find(|l| l["rel"].as_str() == Some("approve")))
            .and_then(|l| l["href"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(PaymentSession {
            provider: "paypal".into(),
            session_id: body["id"].as_str().unwrap_or("").to_string(),
            redirect_url,
            provider_ref: Some(body["id"].as_str().unwrap_or("").to_string()),
        })
    }

    async fn verify_payment(&self, order_id: &str) -> Result<PaymentStatus, PaymentError> {
        let token = self.get_access_token().await?;

        // Capture the order
        let resp = self
            .client
            .post(format!(
                "{}/v2/checkout/orders/{}/capture",
                self.base_url, order_id
            ))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        let status = match body["status"].as_str() {
            Some("COMPLETED") => PaymentStatusKind::Completed,
            Some("APPROVED") => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        let capture = &body["purchase_units"][0]["payments"]["captures"][0];

        Ok(PaymentStatus {
            provider_ref: order_id.to_string(),
            status,
            amount: capture["amount"]["value"]
                .as_str()
                .and_then(|s| s.parse::<Decimal>().ok()),
            currency: capture["amount"]["currency_code"]
                .as_str()
                .map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        // PayPal webhook verification via API call (mandatory).
        let webhook_id = std::env::var("PAYPAL_WEBHOOK_ID")
            .map_err(|_| PaymentError::ProviderError("PAYPAL_WEBHOOK_ID not configured".into()))?;
        if webhook_id.is_empty() {
            return Err(PaymentError::ProviderError("PAYPAL_WEBHOOK_ID is empty".into()));
        }
        if signature.is_empty() {
            return Err(PaymentError::InvalidSignature);
        }

        let verify_headers: serde_json::Value = serde_json::from_str(signature)
            .map_err(|_| PaymentError::InvalidSignature)?;
        let token = self.get_access_token().await?;
        let verify_body = serde_json::json!({
            "auth_algo": verify_headers.get("paypal-auth-algo").and_then(|v| v.as_str()).unwrap_or(""),
            "cert_url": verify_headers.get("paypal-cert-url").and_then(|v| v.as_str()).unwrap_or(""),
            "transmission_id": verify_headers.get("paypal-transmission-id").and_then(|v| v.as_str()).unwrap_or(""),
            "transmission_sig": verify_headers.get("paypal-transmission-sig").and_then(|v| v.as_str()).unwrap_or(""),
            "transmission_time": verify_headers.get("paypal-transmission-time").and_then(|v| v.as_str()).unwrap_or(""),
            "webhook_id": &webhook_id,
            "webhook_event": serde_json::from_slice::<serde_json::Value>(payload)
                .unwrap_or(serde_json::Value::Null),
        });

        let verify_resp = self
            .client
            .post(format!("{}/v1/notifications/verify-webhook-signature", self.base_url))
            .bearer_auth(&token)
            .json(&verify_body)
            .send()
            .await?;

        let verify_body: serde_json::Value = verify_resp.json().await?;
        let verification_status = verify_body["verification_status"].as_str().unwrap_or("");
        if verification_status != "SUCCESS" {
            return Err(PaymentError::InvalidSignature);
        }

        let body: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let event_type = body["event_type"].as_str().unwrap_or("").to_string();
        let resource = &body["resource"];

        let status = match event_type.as_str() {
            "PAYMENT.CAPTURE.COMPLETED" => PaymentStatusKind::Completed,
            "PAYMENT.CAPTURE.DENIED" => PaymentStatusKind::Failed,
            "PAYMENT.CAPTURE.REFUNDED" => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref: resource["id"].as_str().unwrap_or("").to_string(),
            status,
            amount: resource["amount"]["value"]
                .as_str()
                .and_then(|s| s.parse::<Decimal>().ok()),
            currency: resource["amount"]["currency_code"]
                .as_str()
                .map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(
        &self,
        capture_id: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        let token = self.get_access_token().await?;

        let body = if let Some(amt) = amount {
            json!({ "amount": { "value": amt.to_string(), "currency_code": "USD" } })
        } else {
            json!({})
        };

        let resp = self
            .client
            .post(format!(
                "{}/v2/payments/captures/{}/refund",
                self.base_url, capture_id
            ))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await?;

        let resp_body: serde_json::Value = resp.json().await?;

        Ok(RefundResult {
            provider_ref: resp_body["id"].as_str().unwrap_or("").to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: resp_body["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
        })
    }
}
