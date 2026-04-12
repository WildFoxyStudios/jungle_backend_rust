use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct RazorpayGateway {
    key_id: String,
    key_secret: String,
    client: reqwest::Client,
}

impl RazorpayGateway {
    pub fn from_env() -> Self {
        Self {
            key_id: std::env::var("RAZORPAY_KEY_ID").unwrap_or_default(),
            key_secret: std::env::var("RAZORPAY_KEY_SECRET").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for RazorpayGateway {
    fn provider_name(&self) -> &'static str {
        "razorpay"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let amount_paise = (params.amount * Decimal::from(100))
            .to_string()
            .parse::<i64>()
            .map_err(|e| PaymentError::ProviderError(format!("Amount conversion: {}", e)))?;

        let payload = serde_json::json!({
            "amount": amount_paise,
            "currency": params.currency.to_uppercase(),
            "notes": params.metadata,
        });

        let resp = self
            .client
            .post("https://api.razorpay.com/v1/orders")
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        if body["error"].is_object() {
            return Err(PaymentError::ProviderError(
                body["error"]["description"].as_str().unwrap_or("Unknown error").into(),
            ));
        }

        let order_id = body["id"].as_str().unwrap_or("").to_string();

        Ok(PaymentSession {
            provider: "razorpay".into(),
            session_id: order_id.clone(),
            redirect_url: format!(
                "https://api.razorpay.com/v1/checkout/embedded?key_id={}&order_id={}",
                self.key_id, order_id
            ),
            provider_ref: Some(order_id),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let url = format!("https://api.razorpay.com/v1/orders/{}", reference);
        let resp = self
            .client
            .get(&url)
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status = match body["status"].as_str() {
            Some("paid") => PaymentStatusKind::Completed,
            Some("attempted") => PaymentStatusKind::Pending,
            Some("created") => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount: body["amount_paid"]
                .as_i64()
                .map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: body["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent, PaymentError> {
        if signature.is_empty() {
            return Err(PaymentError::InvalidSignature);
        }

        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let entity = &body["payload"]["payment"]["entity"];
        let status = match entity["status"].as_str() {
            Some("captured") => PaymentStatusKind::Completed,
            Some("failed") => PaymentStatusKind::Failed,
            Some("refunded") => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: body["event"].as_str().unwrap_or("unknown").to_string(),
            provider_ref: entity["order_id"].as_str().unwrap_or("").to_string(),
            status,
            amount: entity["amount"]
                .as_i64()
                .map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: entity["currency"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, tx_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let url = format!("https://api.razorpay.com/v1/payments/{}/refund", tx_id);
        let mut payload = serde_json::json!({});
        if let Some(amt) = amount {
            let paise = (amt * Decimal::from(100)).to_string().parse::<i64>().unwrap_or(0);
            payload["amount"] = serde_json::json!(paise);
        }

        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        if body["error"].is_object() {
            return Err(PaymentError::RefundFailed(
                body["error"]["description"].as_str().unwrap_or("Refund failed").into(),
            ));
        }

        Ok(RefundResult {
            provider_ref: body["id"].as_str().unwrap_or(tx_id).to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: body["status"].as_str().unwrap_or("processed").to_string(),
        })
    }
}
