use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

pub struct PayStackGateway {
    secret_key: String,
    client: reqwest::Client,
}

impl PayStackGateway {
    pub fn from_env() -> Self {
        Self {
            secret_key: std::env::var("PAYSTACK_SECRET_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for PayStackGateway {
    fn provider_name(&self) -> &'static str {
        "paystack"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let amount_kobo = (params.amount * Decimal::from(100))
            .to_string()
            .parse::<i64>()
            .map_err(|e| PaymentError::ProviderError(format!("Amount conversion: {}", e)))?;

        let email = params
            .metadata
            .get("email")
            .cloned()
            .unwrap_or_else(|| "customer@example.com".into());

        let payload = serde_json::json!({
            "amount": amount_kobo,
            "email": email,
            "currency": params.currency.to_uppercase(),
            "callback_url": params.return_url,
            "metadata": params.metadata,
        });

        let resp = self
            .client
            .post("https://api.paystack.co/transaction/initialize")
            .bearer_auth(&self.secret_key)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        if body["status"].as_bool() != Some(true) {
            return Err(PaymentError::ProviderError(
                body["message"].as_str().unwrap_or("Unknown error").into(),
            ));
        }

        Ok(PaymentSession {
            provider: "paystack".into(),
            session_id: body["data"]["reference"].as_str().unwrap_or("").to_string(),
            redirect_url: body["data"]["authorization_url"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            provider_ref: body["data"]["access_code"].as_str().map(|s| s.to_string()),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let url = format!("https://api.paystack.co/transaction/verify/{}", reference);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.secret_key)
            .send()
            .await?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status_str = body["data"]["status"].as_str().unwrap_or("pending");
        let status = match status_str {
            "success" => PaymentStatusKind::Completed,
            "failed" => PaymentStatusKind::Failed,
            "abandoned" => PaymentStatusKind::Cancelled,
            _ => PaymentStatusKind::Pending,
        };

        let amount = body["data"]["amount"]
            .as_i64()
            .map(|a| Decimal::from(a) / Decimal::from(100));

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount,
            currency: body["data"]["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        use hmac::{Hmac, Mac};
        use sha2::Sha512;

        let mut mac = Hmac::<Sha512>::new_from_slice(self.secret_key.as_bytes())
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;
        mac.update(payload);
        let expected = hex::encode(mac.finalize().into_bytes());

        if expected != signature {
            return Err(PaymentError::InvalidSignature);
        }

        let body: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let event_type = body["event"].as_str().unwrap_or("unknown").to_string();
        let data = &body["data"];

        let status = match data["status"].as_str() {
            Some("success") => PaymentStatusKind::Completed,
            Some("failed") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref: data["reference"].as_str().unwrap_or("").to_string(),
            status,
            amount: data["amount"]
                .as_i64()
                .map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: data["currency"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(
        &self,
        tx_id: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        let mut payload = serde_json::json!({"transaction": tx_id});
        if let Some(amt) = amount {
            let kobo = (amt * Decimal::from(100))
                .to_string()
                .parse::<i64>()
                .unwrap_or(0);
            payload["amount"] = serde_json::json!(kobo);
        }

        let resp = self
            .client
            .post("https://api.paystack.co/refund")
            .bearer_auth(&self.secret_key)
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        if body["status"].as_bool() != Some(true) {
            return Err(PaymentError::RefundFailed(
                body["message"].as_str().unwrap_or("Refund failed").into(),
            ));
        }

        Ok(RefundResult {
            provider_ref: body["data"]["transaction"]["reference"]
                .as_str()
                .unwrap_or(tx_id)
                .to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: "completed".to_string(),
        })
    }
}
