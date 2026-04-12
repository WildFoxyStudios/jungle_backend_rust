use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct PayProBitcoinGateway {
    api_key: String,
    client: reqwest::Client,
}

impl PayProBitcoinGateway {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("PAYPRO_API_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for PayProBitcoinGateway {
    fn provider_name(&self) -> &'static str {
        "paypro_bitcoin"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let body = serde_json::json!({
            "amount": params.amount.to_string().parse::<f64>().unwrap_or(0.0),
            "currency": params.currency,
            "description": params.description,
            "return_url": params.return_url,
            "cancel_url": params.cancel_url,
            "notify_url": params.metadata.get("webhook_url").cloned().unwrap_or_default(),
            "payment_method": "bitcoin"
        });

        let resp = self
            .client
            .post("https://api.payproglobal.com/v2/payments")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if let Some(err) = result.get("error") {
            return Err(PaymentError::ProviderError(
                err["message"].as_str().unwrap_or("PayPro error").to_string(),
            ));
        }

        Ok(PaymentSession {
            provider: "paypro_bitcoin".into(),
            session_id: result["id"].as_str().unwrap_or("").to_string(),
            redirect_url: result["checkout_url"].as_str().unwrap_or("").to_string(),
            provider_ref: Some(result["id"].as_str().unwrap_or("").to_string()),
        })
    }

    async fn verify_payment(&self, payment_id: &str) -> Result<PaymentStatus, PaymentError> {
        let resp = self
            .client
            .get(format!("https://api.payproglobal.com/v2/payments/{}", payment_id))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = match result["status"].as_str() {
            Some("completed") | Some("confirmed") => PaymentStatusKind::Completed,
            Some("pending") | Some("confirming") => PaymentStatusKind::Pending,
            Some("cancelled") | Some("expired") => PaymentStatusKind::Cancelled,
            Some("refunded") => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Failed,
        };

        Ok(PaymentStatus {
            provider_ref: payment_id.to_string(),
            status,
            amount: result["amount"].as_f64().map(|a| Decimal::try_from(a).unwrap_or(Decimal::ZERO)),
            currency: result["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status = match body["status"].as_str() {
            Some("completed") | Some("confirmed") => PaymentStatusKind::Completed,
            Some("refunded") => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: body["event"].as_str().unwrap_or("payment").to_string(),
            provider_ref: body["payment_id"].as_str().unwrap_or("").to_string(),
            status,
            amount: body["amount"].as_f64().map(|a| Decimal::try_from(a).unwrap_or(Decimal::ZERO)),
            currency: body["currency"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, _tx_id: &str, _amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        // Bitcoin payments are irreversible — refunds must be handled manually
        Err(PaymentError::ProviderError(
            "Bitcoin payments are irreversible. Refunds must be processed manually.".to_string(),
        ))
    }
}
