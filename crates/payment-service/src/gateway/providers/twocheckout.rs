use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct TwoCheckoutGateway {
    merchant_code: String,
    secret_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl TwoCheckoutGateway {
    pub fn from_env() -> Self {
        let sandbox = std::env::var("TWOCHECKOUT_SANDBOX").unwrap_or_else(|_| "true".into());
        let base_url = if sandbox == "true" {
            "https://sandbox.2checkout.com/checkout".to_string()
        } else {
            "https://www.2checkout.com/checkout".to_string()
        };
        Self {
            merchant_code: std::env::var("TWOCHECKOUT_MERCHANT_CODE").unwrap_or_default(),
            secret_key: std::env::var("TWOCHECKOUT_SECRET_KEY").unwrap_or_default(),
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for TwoCheckoutGateway {
    fn provider_name(&self) -> &'static str {
        "2checkout"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let order_ext_ref = format!("2co_{}", uuid::Uuid::new_v4().simple());

        // 2Checkout uses a hosted checkout URL with parameters
        let redirect_url = format!(
            "{}/purchase?merchant={}&dynamic=1&tpl=default&prod={}&price={}&qty=1&currency={}&return-url={}&return-type=redirect&order-ext-ref={}",
            self.base_url,
            self.merchant_code,
            urlencoding::encode(&params.description),
            params.amount,
            params.currency,
            urlencoding::encode(&params.return_url),
            order_ext_ref
        );

        Ok(PaymentSession {
            provider: "2checkout".into(),
            session_id: order_ext_ref.clone(),
            redirect_url,
            provider_ref: Some(order_ext_ref),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let resp = self
            .client
            .get(format!(
                "https://api.2checkout.com/rest/6.0/orders/{}/",
                reference
            ))
            .header("X-Avangate-Authentication", format!(
                "code=\"{}\" date=\"{}\" hash=\"\"",
                self.merchant_code,
                chrono_now_iso()
            ))
            .basic_auth(&self.merchant_code, Some(&self.secret_key))
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = match result["Status"].as_str() {
            Some("COMPLETE") | Some("AUTHRECEIVED") => PaymentStatusKind::Completed,
            Some("CANCELED") | Some("REVERSED") => PaymentStatusKind::Cancelled,
            Some("REFUND") => PaymentStatusKind::Refunded,
            Some("PENDING") | Some("PURCHASE_PENDING") => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        Ok(PaymentStatus {
            provider_ref: result["RefNo"].as_str().unwrap_or("").to_string(),
            status,
            amount: result["GrossPrice"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            currency: result["Currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent, PaymentError> {
        // 2Checkout IPN uses MD5(secret + IPN_PID + IPN_PNAME + IPN_DATE + DATE) signature
        if !signature.is_empty() && !self.secret_key.is_empty() {
            let to_hash = format!("{}{}", self.secret_key, String::from_utf8_lossy(payload));
            let expected = format!("{:x}", md5::compute(to_hash.as_bytes()));
            if expected != signature {
                return Err(PaymentError::InvalidSignature);
            }
        }

        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status = match body["ORDERSTATUS"].as_str() {
            Some("COMPLETE") => PaymentStatusKind::Completed,
            Some("CANCELED") | Some("REVERSED") => PaymentStatusKind::Cancelled,
            Some("REFUND") => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: body["ORDERSTATUS"].as_str().unwrap_or("unknown").to_string(),
            provider_ref: body["REFNO"].as_str().unwrap_or("").to_string(),
            status,
            amount: body["IPN_TOTALGENERAL"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            currency: body["CURRENCY"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, reference: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let body = serde_json::json!({
            "amount": amount.unwrap_or(Decimal::ZERO).to_string(),
            "comment": "Refund request",
            "reason": "Other"
        });

        let resp = self
            .client
            .post(format!(
                "https://api.2checkout.com/rest/6.0/orders/{}/refund/",
                reference
            ))
            .basic_auth(&self.merchant_code, Some(&self.secret_key))
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result.get("error_code").is_some() {
            return Err(PaymentError::RefundFailed(
                result["message"].as_str().unwrap_or("Refund failed").to_string(),
            ));
        }

        Ok(RefundResult {
            provider_ref: reference.to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: "succeeded".to_string(),
        })
    }
}

fn chrono_now_iso() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}
