use async_trait::async_trait;
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use sha2::Sha256;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

type HmacSha256 = Hmac<Sha256>;

const STRIPE_SIGNATURE_TOLERANCE_SECS: i64 = 300;

fn verify_stripe_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
) -> Result<(), PaymentError> {
    let mut timestamp: Option<&str> = None;
    let mut signatures: Vec<&str> = Vec::new();

    for part in sig_header.split(',') {
        let part = part.trim();
        if let Some(ts) = part.strip_prefix("t=") {
            timestamp = Some(ts);
        } else if let Some(sig) = part.strip_prefix("v1=") {
            signatures.push(sig);
        }
    }

    let ts = timestamp.ok_or(PaymentError::InvalidSignature)?;
    if signatures.is_empty() {
        return Err(PaymentError::InvalidSignature);
    }

    let ts_i64: i64 = ts
        .parse()
        .map_err(|_| PaymentError::InvalidSignature)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    if (now - ts_i64).abs() > STRIPE_SIGNATURE_TOLERANCE_SECS {
        return Err(PaymentError::InvalidSignature);
    }

    let signed_payload = format!("{}.", ts);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| PaymentError::InvalidSignature)?;
    mac.update(signed_payload.as_bytes());
    mac.update(payload);
    let expected = hex::encode(mac.finalize().into_bytes());

    if signatures.iter().any(|s| *s == expected) {
        Ok(())
    } else {
        Err(PaymentError::InvalidSignature)
    }
}

pub struct StripeGateway {
    secret_key: String,
    client: reqwest::Client,
}

impl StripeGateway {
    pub fn from_env() -> Self {
        Self {
            secret_key: std::env::var("STRIPE_SECRET_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentGateway for StripeGateway {
    fn provider_name(&self) -> &'static str {
        "stripe"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        // Convert amount to cents
        let amount_cents = (params.amount * Decimal::from(100))
            .to_string()
            .parse::<i64>()
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let mut form = HashMap::new();
        form.insert("mode", "payment".to_string());
        form.insert("currency", params.currency.to_lowercase());
        form.insert("success_url", params.return_url.clone());
        form.insert("cancel_url", params.cancel_url.clone());
        form.insert("line_items[0][price_data][currency]", params.currency.to_lowercase());
        form.insert("line_items[0][price_data][unit_amount]", amount_cents.to_string());
        form.insert("line_items[0][price_data][product_data][name]", params.description.clone());
        form.insert("line_items[0][quantity]", "1".to_string());

        let meta_keys: Vec<(String, String)> = params
            .metadata
            .iter()
            .map(|(k, v)| (format!("metadata[{}]", k), v.clone()))
            .collect();

        for (k, v) in &meta_keys {
            form.insert(k.as_str(), v.clone());
        }

        let resp = self
            .client
            .post("https://api.stripe.com/v1/checkout/sessions")
            .basic_auth(&self.secret_key, None::<&str>)
            .form(&form)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        if let Some(err) = body.get("error") {
            return Err(PaymentError::ProviderError(
                err["message"].as_str().unwrap_or("Stripe error").to_string(),
            ));
        }

        Ok(PaymentSession {
            provider: "stripe".into(),
            session_id: body["id"].as_str().unwrap_or("").to_string(),
            redirect_url: body["url"].as_str().unwrap_or("").to_string(),
            provider_ref: Some(body["payment_intent"].as_str().unwrap_or("").to_string()),
        })
    }

    async fn verify_payment(&self, session_id: &str) -> Result<PaymentStatus, PaymentError> {
        let url = format!("https://api.stripe.com/v1/checkout/sessions/{}", session_id);
        let resp = self
            .client
            .get(&url)
            .basic_auth(&self.secret_key, None::<&str>)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        let status = match body["payment_status"].as_str() {
            Some("paid") => PaymentStatusKind::Completed,
            Some("unpaid") => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        Ok(PaymentStatus {
            provider_ref: body["payment_intent"].as_str().unwrap_or("").to_string(),
            status,
            amount: body["amount_total"].as_i64().map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: body["currency"].as_str().map(|s| s.to_uppercase()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent, PaymentError> {
        let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default();
        if !webhook_secret.is_empty() {
            verify_stripe_signature(payload, signature, &webhook_secret)?;
        }

        let body: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let event_type = body["type"].as_str().unwrap_or("").to_string();
        let data = &body["data"]["object"];

        let status = match event_type.as_str() {
            "checkout.session.completed" => PaymentStatusKind::Completed,
            "payment_intent.payment_failed" => PaymentStatusKind::Failed,
            "charge.refunded" => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref: data["payment_intent"].as_str().unwrap_or("").to_string(),
            status,
            amount: data["amount_total"].as_i64().map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: data["currency"].as_str().map(|s| s.to_uppercase()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, payment_intent: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let mut form = HashMap::new();
        form.insert("payment_intent", payment_intent.to_string());
        if let Some(amt) = amount {
            let cents = (amt * Decimal::from(100)).to_string();
            form.insert("amount", cents);
        }

        let resp = self
            .client
            .post("https://api.stripe.com/v1/refunds")
            .basic_auth(&self.secret_key, None::<&str>)
            .form(&form)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        if let Some(err) = body.get("error") {
            let code = err["code"].as_str().unwrap_or("");
            let msg  = err["message"].as_str().unwrap_or("Refund failed").to_string();
            return Err(if code == "resource_missing" {
                PaymentError::NotFound(format!("Payment not found: {}", payment_intent))
            } else {
                PaymentError::RefundFailed(msg)
            });
        }

        let refunded = body["amount"]
            .as_i64()
            .map(|a| Decimal::from(a) / Decimal::from(100))
            .unwrap_or(Decimal::ZERO);

        Ok(RefundResult {
            provider_ref: body["id"].as_str().unwrap_or("").to_string(),
            refunded_amount: refunded,
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
        })
    }
}
