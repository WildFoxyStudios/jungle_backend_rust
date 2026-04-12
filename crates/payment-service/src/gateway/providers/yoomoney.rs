use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct YooMoneyGateway {
    shop_id: String,
    secret_key: String,
    client: reqwest::Client,
}

impl YooMoneyGateway {
    pub fn from_env() -> Self {
        Self {
            shop_id: std::env::var("YOOMONEY_SHOP_ID").unwrap_or_default(),
            secret_key: std::env::var("YOOMONEY_SECRET_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for YooMoneyGateway {
    fn provider_name(&self) -> &'static str {
        "yoomoney"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let idempotence_key = uuid::Uuid::new_v4().to_string();

        let body = serde_json::json!({
            "amount": {
                "value": params.amount.to_string(),
                "currency": params.currency
            },
            "confirmation": {
                "type": "redirect",
                "return_url": params.return_url
            },
            "capture": true,
            "description": params.description,
            "metadata": params.metadata
        });

        let resp = self
            .client
            .post("https://api.yookassa.ru/v3/payments")
            .basic_auth(&self.shop_id, Some(&self.secret_key))
            .header("Idempotence-Key", &idempotence_key)
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result.get("type").filter(|t| t.as_str() == Some("error")).is_some() {
            return Err(PaymentError::ProviderError(
                result["description"].as_str().unwrap_or("YooMoney error").to_string(),
            ));
        }

        Ok(PaymentSession {
            provider: "yoomoney".into(),
            session_id: result["id"].as_str().unwrap_or("").to_string(),
            redirect_url: result["confirmation"]["confirmation_url"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            provider_ref: Some(result["id"].as_str().unwrap_or("").to_string()),
        })
    }

    async fn verify_payment(&self, payment_id: &str) -> Result<PaymentStatus, PaymentError> {
        let resp = self
            .client
            .get(format!("https://api.yookassa.ru/v3/payments/{}", payment_id))
            .basic_auth(&self.shop_id, Some(&self.secret_key))
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = match result["status"].as_str() {
            Some("succeeded") => PaymentStatusKind::Completed,
            Some("canceled") => PaymentStatusKind::Cancelled,
            Some("waiting_for_capture") | Some("pending") => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        Ok(PaymentStatus {
            provider_ref: result["id"].as_str().unwrap_or("").to_string(),
            status,
            amount: result["amount"]["value"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            currency: result["amount"]["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let event_type = body["event"].as_str().unwrap_or("").to_string();
        let object = &body["object"];

        let status = match event_type.as_str() {
            "payment.succeeded" => PaymentStatusKind::Completed,
            "payment.canceled" => PaymentStatusKind::Cancelled,
            "refund.succeeded" => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref: object["id"].as_str().unwrap_or("").to_string(),
            status,
            amount: object["amount"]["value"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            currency: object["amount"]["currency"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, payment_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let idempotence_key = uuid::Uuid::new_v4().to_string();

        let mut body = serde_json::json!({
            "payment_id": payment_id
        });

        if let Some(amt) = amount {
            body["amount"] = serde_json::json!({
                "value": amt.to_string(),
                "currency": "RUB"
            });
        }

        let resp = self
            .client
            .post("https://api.yookassa.ru/v3/refunds")
            .basic_auth(&self.shop_id, Some(&self.secret_key))
            .header("Idempotence-Key", &idempotence_key)
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["status"].as_str() == Some("canceled") {
            return Err(PaymentError::RefundFailed("Refund canceled".to_string()));
        }

        Ok(RefundResult {
            provider_ref: result["id"].as_str().unwrap_or("").to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: result["status"].as_str().unwrap_or("pending").to_string(),
        })
    }
}
