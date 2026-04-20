use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct CashfreeGateway {
    app_id: String,
    secret_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl CashfreeGateway {
    pub fn from_env() -> Self {
        let sandbox = std::env::var("CASHFREE_SANDBOX").unwrap_or_else(|_| "true".into());
        let base_url = if sandbox == "true" {
            "https://sandbox.cashfree.com/pg".to_string()
        } else {
            "https://api.cashfree.com/pg".to_string()
        };
        Self {
            app_id: std::env::var("CASHFREE_APP_ID").unwrap_or_default(),
            secret_key: std::env::var("CASHFREE_SECRET_KEY").unwrap_or_default(),
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for CashfreeGateway {
    fn provider_name(&self) -> &'static str {
        "cashfree"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let order_id = format!("cf_{}", uuid::Uuid::new_v4().simple());

        let body = serde_json::json!({
            "order_id": order_id,
            "order_amount": params.amount.to_string().parse::<f64>().unwrap_or(0.0),
            "order_currency": params.currency,
            "order_note": params.description,
            "customer_details": {
                "customer_id": params.metadata.get("user_id").cloned().unwrap_or_else(|| "0".into()),
                "customer_email": params.metadata.get("email").cloned().unwrap_or_else(|| "user@example.com".into()),
                "customer_phone": params.metadata.get("phone").cloned().unwrap_or_else(|| "9999999999".into()),
            },
            "order_meta": {
                "return_url": format!("{}?order_id={}", params.return_url, order_id),
                "notify_url": params.metadata.get("webhook_url").cloned().unwrap_or_default(),
            }
        });

        let resp = self
            .client
            .post(format!("{}/orders", self.base_url))
            .header("x-client-id", &self.app_id)
            .header("x-client-secret", &self.secret_key)
            .header("x-api-version", "2023-08-01")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if let Some(msg) = result.get("message")
            && result.get("payment_session_id").is_none()
        {
            return Err(PaymentError::ProviderError(
                msg.as_str().unwrap_or("Cashfree error").to_string(),
            ));
        }

        let session_id = result["payment_session_id"].as_str().unwrap_or("").to_string();
        let payment_link = result["payment_link"].as_str().unwrap_or("").to_string();

        Ok(PaymentSession {
            provider: "cashfree".into(),
            session_id: session_id.clone(),
            redirect_url: payment_link,
            provider_ref: Some(order_id),
        })
    }

    async fn verify_payment(&self, order_id: &str) -> Result<PaymentStatus, PaymentError> {
        let resp = self
            .client
            .get(format!("{}/orders/{}", self.base_url, order_id))
            .header("x-client-id", &self.app_id)
            .header("x-client-secret", &self.secret_key)
            .header("x-api-version", "2023-08-01")
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = match result["order_status"].as_str() {
            Some("PAID") => PaymentStatusKind::Completed,
            Some("EXPIRED") | Some("CANCELLED") => PaymentStatusKind::Cancelled,
            Some("ACTIVE") => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        Ok(PaymentStatus {
            provider_ref: result["cf_order_id"].as_str().unwrap_or("").to_string(),
            status,
            amount: result["order_amount"].as_f64().map(|a| Decimal::try_from(a).unwrap_or(Decimal::ZERO)),
            currency: result["order_currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent, PaymentError> {
        // Cashfree sends webhook with x-webhook-signature header (HMAC-SHA256)
        if !signature.is_empty() && !self.secret_key.is_empty() {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;

            let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
                .map_err(|_| PaymentError::InvalidSignature)?;
            mac.update(payload);
            let expected = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                mac.finalize().into_bytes(),
            );
            if expected != signature {
                return Err(PaymentError::InvalidSignature);
            }
        }

        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let data = &body["data"];
        let order = &data["order"];
        let payment = &data["payment"];

        let status = match body["type"].as_str() {
            Some("PAYMENT_SUCCESS_WEBHOOK") => PaymentStatusKind::Completed,
            Some("PAYMENT_FAILED_WEBHOOK") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: body["type"].as_str().unwrap_or("unknown").to_string(),
            provider_ref: order["order_id"].as_str().unwrap_or("").to_string(),
            status,
            amount: payment["payment_amount"].as_f64().map(|a| Decimal::try_from(a).unwrap_or(Decimal::ZERO)),
            currency: payment["payment_currency"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, order_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let refund_id = format!("refund_{}", uuid::Uuid::new_v4().simple());
        let body = serde_json::json!({
            "refund_amount": amount.unwrap_or(Decimal::ZERO).to_string().parse::<f64>().unwrap_or(0.0),
            "refund_id": refund_id,
            "refund_note": "Refund request"
        });

        let resp = self
            .client
            .post(format!("{}/orders/{}/refunds", self.base_url, order_id))
            .header("x-client-id", &self.app_id)
            .header("x-client-secret", &self.secret_key)
            .header("x-api-version", "2023-08-01")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["refund_status"].as_str() == Some("CANCELLED") {
            return Err(PaymentError::RefundFailed("Refund cancelled".to_string()));
        }

        Ok(RefundResult {
            provider_ref: result["cf_refund_id"].as_str().unwrap_or("").to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: result["refund_status"].as_str().unwrap_or("pending").to_string(),
        })
    }
}
