use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct FlutterwaveGateway {
    secret_key: String,
    client: reqwest::Client,
}

impl FlutterwaveGateway {
    pub fn from_env() -> Self {
        Self {
            secret_key: std::env::var("FLUTTERWAVE_SECRET_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for FlutterwaveGateway {
    fn provider_name(&self) -> &'static str {
        "flutterwave"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let tx_ref = format!("fw-{}", uuid::Uuid::new_v4());
        let email = params.metadata.get("email").cloned().unwrap_or_default();

        let payload = serde_json::json!({
            "tx_ref": tx_ref,
            "amount": params.amount.to_string(),
            "currency": params.currency.to_uppercase(),
            "redirect_url": params.return_url,
            "customer": { "email": email },
            "meta": params.metadata,
            "customizations": {
                "title": params.description,
            }
        });

        let resp = self
            .client
            .post("https://api.flutterwave.com/v3/payments")
            .bearer_auth(&self.secret_key)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        if body["status"].as_str() != Some("success") {
            return Err(PaymentError::ProviderError(
                body["message"].as_str().unwrap_or("Unknown error").into(),
            ));
        }

        Ok(PaymentSession {
            provider: "flutterwave".into(),
            session_id: tx_ref,
            redirect_url: body["data"]["link"].as_str().unwrap_or("").to_string(),
            provider_ref: None,
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let url = format!(
            "https://api.flutterwave.com/v3/transactions/{}/verify",
            reference
        );
        let resp = self.client.get(&url).bearer_auth(&self.secret_key).send().await?;
        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status = match body["data"]["status"].as_str() {
            Some("successful") => PaymentStatusKind::Completed,
            Some("failed") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount: body["data"]["amount"]
                .as_f64()
                .map(|a| Decimal::try_from(a).unwrap_or(Decimal::ZERO)),
            currency: body["data"]["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let data = &body["data"];
        let status = match data["status"].as_str() {
            Some("successful") => PaymentStatusKind::Completed,
            Some("failed") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: body["event"].as_str().unwrap_or("unknown").to_string(),
            provider_ref: data["tx_ref"].as_str().unwrap_or("").to_string(),
            status,
            amount: data["amount"].as_f64().map(|a| Decimal::try_from(a).unwrap_or(Decimal::ZERO)),
            currency: data["currency"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, tx_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let mut payload = serde_json::json!({});
        if let Some(amt) = amount {
            payload["amount"] = serde_json::json!(amt.to_string().parse::<f64>().unwrap_or(0.0));
        }

        let url = format!("https://api.flutterwave.com/v3/transactions/{}/refund", tx_id);
        let resp = self.client.post(&url).bearer_auth(&self.secret_key).json(&payload).send().await?;
        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        if body["status"].as_str() != Some("success") {
            return Err(PaymentError::RefundFailed(body["message"].as_str().unwrap_or("Refund failed").into()));
        }

        Ok(RefundResult {
            provider_ref: tx_id.to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: "completed".to_string(),
        })
    }
}
