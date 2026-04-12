use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct SecurionPayGateway {
    secret_key: String,
    client: reqwest::Client,
}

impl SecurionPayGateway {
    pub fn from_env() -> Self {
        Self {
            secret_key: std::env::var("SECURIONPAY_SECRET_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for SecurionPayGateway {
    fn provider_name(&self) -> &'static str {
        "securionpay"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let amount_cents = (params.amount * Decimal::from(100))
            .to_string()
            .parse::<i64>()
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let body = serde_json::json!({
            "amount": amount_cents,
            "currency": params.currency.to_lowercase(),
            "description": params.description,
            "metadata": params.metadata
        });

        let resp = self
            .client
            .post("https://api.securionpay.com/charges")
            .basic_auth(&self.secret_key, None::<&str>)
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if let Some(err) = result.get("error") {
            return Err(PaymentError::ProviderError(
                err["message"].as_str().unwrap_or("SecurionPay error").to_string(),
            ));
        }

        Ok(PaymentSession {
            provider: "securionpay".into(),
            session_id: result["id"].as_str().unwrap_or("").to_string(),
            redirect_url: params.return_url,
            provider_ref: Some(result["id"].as_str().unwrap_or("").to_string()),
        })
    }

    async fn verify_payment(&self, charge_id: &str) -> Result<PaymentStatus, PaymentError> {
        let resp = self
            .client
            .get(format!("https://api.securionpay.com/charges/{}", charge_id))
            .basic_auth(&self.secret_key, None::<&str>)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = if result["captured"].as_bool() == Some(true) {
            PaymentStatusKind::Completed
        } else if result["refunded"].as_bool() == Some(true) {
            PaymentStatusKind::Refunded
        } else if result["status"].as_str() == Some("failed") {
            PaymentStatusKind::Failed
        } else {
            PaymentStatusKind::Pending
        };

        Ok(PaymentStatus {
            provider_ref: result["id"].as_str().unwrap_or("").to_string(),
            status,
            amount: result["amount"].as_i64().map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: result["currency"].as_str().map(|s| s.to_uppercase()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let event_type = body["type"].as_str().unwrap_or("").to_string();
        let data = &body["data"];

        let status = match event_type.as_str() {
            "CHARGE_SUCCEEDED" => PaymentStatusKind::Completed,
            "CHARGE_FAILED" => PaymentStatusKind::Failed,
            "CHARGE_REFUNDED" => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref: data["id"].as_str().unwrap_or("").to_string(),
            status,
            amount: data["amount"].as_i64().map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: data["currency"].as_str().map(|s| s.to_uppercase()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, charge_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let mut body = serde_json::json!({
            "chargeId": charge_id
        });

        if let Some(amt) = amount {
            let cents = (amt * Decimal::from(100)).to_string().parse::<i64>().unwrap_or(0);
            body["amount"] = serde_json::json!(cents);
        }

        let resp = self
            .client
            .post("https://api.securionpay.com/refunds")
            .basic_auth(&self.secret_key, None::<&str>)
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if let Some(err) = result.get("error") {
            return Err(PaymentError::RefundFailed(
                err["message"].as_str().unwrap_or("Refund failed").to_string(),
            ));
        }

        let refunded = result["amount"]
            .as_i64()
            .map(|a| Decimal::from(a) / Decimal::from(100))
            .unwrap_or(Decimal::ZERO);

        Ok(RefundResult {
            provider_ref: result["id"].as_str().unwrap_or("").to_string(),
            refunded_amount: refunded,
            status: "succeeded".to_string(),
        })
    }
}
