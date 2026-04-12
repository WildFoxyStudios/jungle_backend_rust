use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct PayFastGateway {
    merchant_id: String,
    merchant_key: String,
    passphrase: String,
    sandbox: bool,
    client: reqwest::Client,
}

impl PayFastGateway {
    pub fn from_env() -> Self {
        let sandbox = std::env::var("PAYFAST_SANDBOX").unwrap_or_else(|_| "true".into()) == "true";
        Self {
            merchant_id: std::env::var("PAYFAST_MERCHANT_ID").unwrap_or_default(),
            merchant_key: std::env::var("PAYFAST_MERCHANT_KEY").unwrap_or_default(),
            passphrase: std::env::var("PAYFAST_PASSPHRASE").unwrap_or_default(),
            sandbox,
            client: reqwest::Client::new(),
        }
    }

    fn base_url(&self) -> &str {
        if self.sandbox {
            "https://sandbox.payfast.co.za"
        } else {
            "https://www.payfast.co.za"
        }
    }

    fn generate_signature(&self, params: &[(String, String)]) -> String {
        let param_string: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let with_passphrase = if self.passphrase.is_empty() {
            param_string
        } else {
            format!("{}&passphrase={}", param_string, urlencoding::encode(&self.passphrase))
        };

        format!("{:x}", md5::compute(with_passphrase.as_bytes()))
    }
}

#[async_trait]
impl PaymentGateway for PayFastGateway {
    fn provider_name(&self) -> &'static str {
        "payfast"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let payment_id = format!("pf_{}", uuid::Uuid::new_v4().simple());

        let mut form_params: Vec<(String, String)> = vec![
            ("merchant_id".into(), self.merchant_id.clone()),
            ("merchant_key".into(), self.merchant_key.clone()),
            ("return_url".into(), params.return_url.clone()),
            ("cancel_url".into(), params.cancel_url.clone()),
            ("notify_url".into(), params.metadata.get("webhook_url").cloned().unwrap_or_default()),
            ("m_payment_id".into(), payment_id.clone()),
            ("amount".into(), format!("{:.2}", params.amount)),
            ("item_name".into(), params.description.clone()),
        ];

        if let Some(email) = params.metadata.get("email") {
            form_params.push(("email_address".into(), email.clone()));
        }

        let signature = self.generate_signature(&form_params);
        form_params.push(("signature".into(), signature));

        // Build redirect URL with form params
        let query: String = form_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let redirect_url = format!("{}/eng/process?{}", self.base_url(), query);

        Ok(PaymentSession {
            provider: "payfast".into(),
            session_id: payment_id.clone(),
            redirect_url,
            provider_ref: Some(payment_id),
        })
    }

    async fn verify_payment(&self, pf_payment_id: &str) -> Result<PaymentStatus, PaymentError> {
        let api_base = if self.sandbox {
            "https://sandbox.payfast.co.za"
        } else {
            "https://api.payfast.co.za"
        };

        let timestamp = {
            let now = time::OffsetDateTime::now_utc();
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
                now.year(), now.month() as u8, now.day(),
                now.hour(), now.minute(), now.second()
            )
        };

        let resp = self
            .client
            .get(format!("{}/eng/query/validate", api_base))
            .header("merchant-id", &self.merchant_id)
            .header("version", "v1")
            .header("timestamp", timestamp)
            .query(&[("m_payment_id", pf_payment_id)])
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = match result["data"]["status"].as_str() {
            Some("COMPLETE") => PaymentStatusKind::Completed,
            Some("CANCELLED") => PaymentStatusKind::Cancelled,
            _ => PaymentStatusKind::Pending,
        };

        Ok(PaymentStatus {
            provider_ref: result["data"]["pf_payment_id"].as_str().unwrap_or("").to_string(),
            status,
            amount: result["data"]["amount_gross"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            currency: Some("ZAR".to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let form: HashMap<String, String> = serde_urlencoded::from_bytes(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        // Verify PayFast signature
        let mut sorted_params: Vec<(String, String)> = form
            .iter()
            .filter(|(k, _)| k.as_str() != "signature")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorted_params.sort_by(|a, b| a.0.cmp(&b.0));

        let expected_sig = self.generate_signature(&sorted_params);
        if let Some(provided_sig) = form.get("signature") {
            if *provided_sig != expected_sig {
                return Err(PaymentError::InvalidSignature);
            }
        }

        let status = match form.get("payment_status").map(|s| s.as_str()) {
            Some("COMPLETE") => PaymentStatusKind::Completed,
            Some("CANCELLED") => PaymentStatusKind::Cancelled,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: form.get("payment_status").cloned().unwrap_or_else(|| "unknown".into()),
            provider_ref: form.get("pf_payment_id").cloned().unwrap_or_default(),
            status,
            amount: form.get("amount_gross").and_then(|s| s.parse().ok()),
            currency: Some("ZAR".to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, _tx_id: &str, _amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        // PayFast refunds are initiated via their merchant dashboard, not via API
        Err(PaymentError::ProviderError(
            "PayFast refunds must be processed via the merchant dashboard".to_string(),
        ))
    }
}
