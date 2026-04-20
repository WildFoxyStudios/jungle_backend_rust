use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct FortumoGateway {
    service_id: String,
    secret: String,
    client: reqwest::Client,
}

impl FortumoGateway {
    pub fn from_env() -> Self {
        Self {
            service_id: std::env::var("FORTUMO_SERVICE_ID").unwrap_or_default(),
            secret: std::env::var("FORTUMO_SECRET").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for FortumoGateway {
    fn provider_name(&self) -> &'static str {
        "fortumo"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let payment_id = format!("ft_{}", uuid::Uuid::new_v4().simple());

        // Fortumo uses a hosted payment page — build the redirect URL
        let redirect_url = format!(
            "https://pay.fortumo.com/mobile_payments.php?service_id={}&cuid={}&amount={}&currency={}&return_url={}",
            self.service_id,
            payment_id,
            params.amount,
            params.currency,
            urlencoding::encode(&params.return_url)
        );

        Ok(PaymentSession {
            provider: "fortumo".into(),
            session_id: payment_id.clone(),
            redirect_url,
            provider_ref: Some(payment_id),
        })
    }

    async fn verify_payment(&self, payment_id: &str) -> Result<PaymentStatus, PaymentError> {
        let resp = self
            .client
            .get("https://api.fortumo.com/api/v1/payments/")
            .basic_auth(&self.service_id, Some(&self.secret))
            .query(&[("cuid", payment_id)])
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let payment = &result["payments"][0];
        let status = match payment["status"].as_str() {
            Some("completed") => PaymentStatusKind::Completed,
            Some("failed") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(PaymentStatus {
            provider_ref: payment["payment_id"].as_str().unwrap_or("").to_string(),
            status,
            amount: payment["price"].as_str().and_then(|s| s.parse().ok()),
            currency: payment["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent, PaymentError> {
        let form: HashMap<String, String> = serde_urlencoded::from_bytes(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        // Verify Fortumo signature: md5(sorted params + secret)
        if !self.secret.is_empty() {
            let mut sorted_keys: Vec<&String> = form.keys()
                .filter(|k| k.as_str() != "sig")
                .collect();
            sorted_keys.sort();
            let sig_string: String = sorted_keys
                .iter()
                .map(|k| form.get(*k).cloned().unwrap_or_default())
                .collect::<Vec<_>>()
                .join("");
            let expected = format!("{:x}", md5::compute(format!("{}{}", sig_string, self.secret)));
            if let Some(provided) = form.get("sig")
                && *provided != expected
                && !signature.is_empty()
                && signature != expected
            {
                return Err(PaymentError::InvalidSignature);
            }
        }

        let status = match form.get("status").map(|s| s.as_str()) {
            Some("completed") => PaymentStatusKind::Completed,
            Some("failed") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: "payment".to_string(),
            provider_ref: form.get("payment_id").cloned().unwrap_or_default(),
            status,
            amount: form.get("price").and_then(|s| s.parse().ok()),
            currency: form.get("currency").cloned(),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, _tx_id: &str, _amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        // Fortumo carrier billing does not support automated refunds
        Err(PaymentError::ProviderError(
            "Fortumo carrier billing does not support automated refunds".to_string(),
        ))
    }
}
