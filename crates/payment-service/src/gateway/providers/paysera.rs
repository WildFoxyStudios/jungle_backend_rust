use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct PayseraGateway {
    project_id: String,
    sign_password: String,
    client: reqwest::Client,
}

impl PayseraGateway {
    pub fn from_env() -> Self {
        Self {
            project_id: std::env::var("PAYSERA_PROJECT_ID").unwrap_or_default(),
            sign_password: std::env::var("PAYSERA_SIGN_PASSWORD").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }

    fn sign_data(&self, data: &str) -> String {
        let to_sign = format!("{}{}", data, self.sign_password);
        format!("{:x}", md5::compute(to_sign.as_bytes()))
    }
}

#[async_trait]
impl PaymentGateway for PayseraGateway {
    fn provider_name(&self) -> &'static str {
        "paysera"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let order_id = format!("ps_{}", uuid::Uuid::new_v4().simple());
        let amount_cents = (params.amount * Decimal::from(100)).to_string().replace(".0", "");

        let mut pay_params: HashMap<String, String> = HashMap::new();
        pay_params.insert("projectid".into(), self.project_id.clone());
        pay_params.insert("orderid".into(), order_id.clone());
        pay_params.insert("amount".into(), amount_cents);
        pay_params.insert("currency".into(), params.currency.clone());
        pay_params.insert("accepturl".into(), params.return_url.clone());
        pay_params.insert("cancelurl".into(), params.cancel_url.clone());
        pay_params.insert("callbackurl".into(), params.metadata.get("webhook_url").cloned().unwrap_or_default());
        pay_params.insert("test".into(), std::env::var("PAYSERA_TEST").unwrap_or_else(|_| "1".into()));

        // Base64 encode parameters
        let query_string = serde_urlencoded::to_string(&pay_params)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;
        let data = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            query_string.as_bytes(),
        );
        let sign = self.sign_data(&data);

        let redirect_url = format!(
            "https://www.paysera.com/pay/?data={}&sign={}",
            data, sign
        );

        Ok(PaymentSession {
            provider: "paysera".into(),
            session_id: order_id.clone(),
            redirect_url,
            provider_ref: Some(order_id),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        // Paysera Macro API: check payment status via GET request
        let resp = self
            .client
            .get("https://www.paysera.com/pay/publicapi/paymentStatus")
            .query(&[
                ("projectid", self.project_id.as_str()),
                ("orderid", reference),
            ])
            .send()
            .await?;

        let body = resp.text().await.unwrap_or_default();
        // Paysera returns simple key=value pairs
        let status = if body.contains("status=1") {
            PaymentStatusKind::Completed
        } else if body.contains("status=2") || body.contains("status=0") {
            PaymentStatusKind::Pending
        } else {
            PaymentStatusKind::Failed
        };

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount: None,
            currency: None,
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let form: HashMap<String, String> = serde_urlencoded::from_bytes(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let data = form.get("data").ok_or_else(|| PaymentError::ProviderError("Missing data".into()))?;
        let ss1 = form.get("ss1").ok_or_else(|| PaymentError::ProviderError("Missing ss1".into()))?;

        // Verify signature
        let expected_sign = self.sign_data(data);
        if *ss1 != expected_sign {
            return Err(PaymentError::InvalidSignature);
        }

        // Decode base64 data
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            data,
        )
        .map_err(|_| PaymentError::ProviderError("Invalid base64 data".into()))?;

        let params: HashMap<String, String> = serde_urlencoded::from_bytes(&decoded)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status_code = params.get("status").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
        let status = match status_code {
            1 => PaymentStatusKind::Completed,
            2 => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        Ok(WebhookEvent {
            event_type: "callback".to_string(),
            provider_ref: params.get("orderid").cloned().unwrap_or_default(),
            status,
            amount: params.get("amount").and_then(|s| {
                s.parse::<i64>().ok().map(|cents| Decimal::from(cents) / Decimal::from(100))
            }),
            currency: params.get("currency").cloned(),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, _tx_id: &str, _amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        Err(PaymentError::ProviderError(
            "Paysera refunds must be processed via the merchant dashboard".to_string(),
        ))
    }
}
