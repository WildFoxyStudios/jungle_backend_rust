use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct AamarPayGateway {
    store_id: String,
    signature_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AamarPayGateway {
    pub fn from_env() -> Self {
        let sandbox = std::env::var("AAMARPAY_SANDBOX").unwrap_or_else(|_| "true".into());
        let base_url = if sandbox == "true" {
            "https://sandbox.aamarpay.com".to_string()
        } else {
            "https://secure.aamarpay.com".to_string()
        };
        Self {
            store_id: std::env::var("AAMARPAY_STORE_ID").unwrap_or_default(),
            signature_key: std::env::var("AAMARPAY_SIGNATURE_KEY").unwrap_or_default(),
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for AamarPayGateway {
    fn provider_name(&self) -> &'static str {
        "aamarpay"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let tran_id = format!("ap_{}", uuid::Uuid::new_v4().simple());

        let body = serde_json::json!({
            "store_id": self.store_id,
            "signature_key": self.signature_key,
            "tran_id": tran_id,
            "amount": params.amount.to_string(),
            "currency": params.currency,
            "desc": params.description,
            "cus_name": params.metadata.get("name").cloned().unwrap_or_else(|| "Customer".into()),
            "cus_email": params.metadata.get("email").cloned().unwrap_or_else(|| "user@example.com".into()),
            "cus_phone": params.metadata.get("phone").cloned().unwrap_or_else(|| "01700000000".into()),
            "success_url": params.return_url,
            "fail_url": params.cancel_url,
            "cancel_url": params.cancel_url,
            "type": "json"
        });

        let resp = self
            .client
            .post(format!("{}/jsonpost.php", self.base_url))
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["result"].as_str() != Some("true") {
            return Err(PaymentError::ProviderError(
                result["error_message"].as_str().unwrap_or("aamarPay error").to_string(),
            ));
        }

        let payment_url = result["payment_url"].as_str().unwrap_or("").to_string();

        Ok(PaymentSession {
            provider: "aamarpay".into(),
            session_id: tran_id.clone(),
            redirect_url: payment_url,
            provider_ref: Some(tran_id),
        })
    }

    async fn verify_payment(&self, tran_id: &str) -> Result<PaymentStatus, PaymentError> {
        let resp = self
            .client
            .get(format!(
                "{}/api/v1/trxcheck/request.php?request_id={}&store_id={}&signature_key={}&type=json",
                self.base_url, tran_id, self.store_id, self.signature_key
            ))
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = match result["pay_status"].as_str() {
            Some("Successful") => PaymentStatusKind::Completed,
            Some("Failed") => PaymentStatusKind::Failed,
            Some("Cancel") => PaymentStatusKind::Cancelled,
            _ => PaymentStatusKind::Pending,
        };

        Ok(PaymentStatus {
            provider_ref: result["pg_txnid"].as_str().unwrap_or("").to_string(),
            status,
            amount: result["amount"].as_str().and_then(|s| s.parse().ok()),
            currency: result["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status = match body["pay_status"].as_str() {
            Some("Successful") => PaymentStatusKind::Completed,
            Some("Failed") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: "payment".to_string(),
            provider_ref: body["mer_txnid"].as_str().unwrap_or("").to_string(),
            status,
            amount: body["amount"].as_str().and_then(|s| s.parse().ok()),
            currency: body["currency"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, _tx_id: &str, _amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        // aamarPay does not provide a direct refund API; refunds are processed manually via dashboard
        Err(PaymentError::ProviderError(
            "aamarPay refunds must be processed via the merchant dashboard".to_string(),
        ))
    }
}
