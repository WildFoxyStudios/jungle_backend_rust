use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct CoinbaseGateway {
    api_key: String,
    client: reqwest::Client,
}

impl CoinbaseGateway {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("COINBASE_COMMERCE_API_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for CoinbaseGateway {
    fn provider_name(&self) -> &'static str {
        "coinbase"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let payload = serde_json::json!({
            "name": params.description,
            "description": params.payment_type,
            "pricing_type": "fixed_price",
            "local_price": {
                "amount": params.amount.to_string(),
                "currency": params.currency.to_uppercase(),
            },
            "redirect_url": params.return_url,
            "cancel_url": params.cancel_url,
            "metadata": params.metadata,
        });

        let resp = self
            .client
            .post("https://api.commerce.coinbase.com/charges")
            .header("X-CC-Api-Key", &self.api_key)
            .header("X-CC-Version", "2018-03-22")
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        if body["error"].is_object() {
            return Err(PaymentError::ProviderError(
                body["error"]["message"].as_str().unwrap_or("Unknown error").into(),
            ));
        }

        let data = &body["data"];
        Ok(PaymentSession {
            provider: "coinbase".into(),
            session_id: data["code"].as_str().unwrap_or("").to_string(),
            redirect_url: data["hosted_url"].as_str().unwrap_or("").to_string(),
            provider_ref: data["id"].as_str().map(|s| s.to_string()),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let url = format!("https://api.commerce.coinbase.com/charges/{}", reference);
        let resp = self
            .client
            .get(&url)
            .header("X-CC-Api-Key", &self.api_key)
            .header("X-CC-Version", "2018-03-22")
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let timeline = body["data"]["timeline"].as_array();
        let last_status = timeline
            .and_then(|t| t.last())
            .and_then(|s| s["status"].as_str())
            .unwrap_or("NEW");

        let status = match last_status {
            "COMPLETED" | "RESOLVED" => PaymentStatusKind::Completed,
            "EXPIRED" | "CANCELED" => PaymentStatusKind::Cancelled,
            "UNRESOLVED" => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount: None,
            currency: None,
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let event = &body["event"];
        let event_type = event["type"].as_str().unwrap_or("unknown").to_string();
        let data = &event["data"];

        let status = match event_type.as_str() {
            "charge:confirmed" | "charge:resolved" => PaymentStatusKind::Completed,
            "charge:failed" => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref: data["code"].as_str().unwrap_or("").to_string(),
            status,
            amount: None,
            currency: None,
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, tx_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        Err(PaymentError::RefundFailed(format!(
            "Coinbase Commerce does not support refunds via API for charge {}. Amount: {:?}",
            tx_id, amount
        )))
    }
}
