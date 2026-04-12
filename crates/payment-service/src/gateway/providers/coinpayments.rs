use async_trait::async_trait;
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use sha2::Sha512;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

type HmacSha512 = Hmac<Sha512>;

pub struct CoinPaymentsGateway {
    merchant_id: String,
    public_key: String,
    private_key: String,
    ipn_secret: String,
    client: reqwest::Client,
}

impl CoinPaymentsGateway {
    pub fn from_env() -> Self {
        Self {
            merchant_id: std::env::var("COINPAYMENTS_MERCHANT_ID").unwrap_or_default(),
            public_key: std::env::var("COINPAYMENTS_PUBLIC_KEY").unwrap_or_default(),
            private_key: std::env::var("COINPAYMENTS_PRIVATE_KEY").unwrap_or_default(),
            ipn_secret: std::env::var("COINPAYMENTS_IPN_SECRET").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }

    async fn api_call(&self, cmd: &str, params: &HashMap<String, String>) -> Result<serde_json::Value, PaymentError> {
        let mut form: HashMap<String, String> = params.clone();
        form.insert("version".into(), "1".into());
        form.insert("key".into(), self.public_key.clone());
        form.insert("cmd".into(), cmd.into());
        form.insert("format".into(), "json".into());

        let form_body: String = serde_urlencoded::to_string(&form)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let mut mac = HmacSha512::new_from_slice(self.private_key.as_bytes())
            .map_err(|_| PaymentError::ProviderError("Invalid HMAC key".into()))?;
        mac.update(form_body.as_bytes());
        let hmac_sig = hex::encode(mac.finalize().into_bytes());

        let resp = self
            .client
            .post("https://www.coinpayments.net/api.php")
            .header("HMAC", hmac_sig)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["error"].as_str() != Some("ok") {
            return Err(PaymentError::ProviderError(
                result["error"].as_str().unwrap_or("CoinPayments error").to_string(),
            ));
        }

        Ok(result["result"].clone())
    }
}

#[async_trait]
impl PaymentGateway for CoinPaymentsGateway {
    fn provider_name(&self) -> &'static str {
        "coinpayments"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let mut api_params = HashMap::new();
        api_params.insert("amount".into(), params.amount.to_string());
        api_params.insert("currency1".into(), params.currency.clone());
        api_params.insert("currency2".into(), params.metadata.get("crypto").cloned().unwrap_or_else(|| "BTC".into()));
        api_params.insert("buyer_email".into(), params.metadata.get("email").cloned().unwrap_or_default());
        api_params.insert("item_name".into(), params.description.clone());
        api_params.insert("ipn_url".into(), params.metadata.get("webhook_url").cloned().unwrap_or_default());

        let result = self.api_call("create_transaction", &api_params).await?;

        Ok(PaymentSession {
            provider: "coinpayments".into(),
            session_id: result["txn_id"].as_str().unwrap_or("").to_string(),
            redirect_url: result["checkout_url"].as_str().unwrap_or("").to_string(),
            provider_ref: Some(result["txn_id"].as_str().unwrap_or("").to_string()),
        })
    }

    async fn verify_payment(&self, txn_id: &str) -> Result<PaymentStatus, PaymentError> {
        let mut api_params = HashMap::new();
        api_params.insert("txid".into(), txn_id.to_string());

        let result = self.api_call("get_tx_info", &api_params).await?;

        let status_code = result["status"].as_i64().unwrap_or(-1);
        let status = if status_code >= 100 {
            PaymentStatusKind::Completed
        } else if status_code >= 0 {
            PaymentStatusKind::Pending
        } else {
            PaymentStatusKind::Failed
        };

        Ok(PaymentStatus {
            provider_ref: txn_id.to_string(),
            status,
            amount: result["amountf"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            currency: result["coin"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent, PaymentError> {
        // CoinPayments IPN uses HMAC-SHA512 with ipn_secret
        if !self.ipn_secret.is_empty() {
            let mut mac = HmacSha512::new_from_slice(self.ipn_secret.as_bytes())
                .map_err(|_| PaymentError::InvalidSignature)?;
            mac.update(payload);
            let expected = hex::encode(mac.finalize().into_bytes());
            if expected != signature {
                return Err(PaymentError::InvalidSignature);
            }
        }

        let form: HashMap<String, String> = serde_urlencoded::from_bytes(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        // Verify the IPN merchant matches our configured merchant_id
        if !self.merchant_id.is_empty() {
            if let Some(ipn_merchant) = form.get("merchant") {
                if *ipn_merchant != self.merchant_id {
                    return Err(PaymentError::InvalidSignature);
                }
            }
        }

        let status_code: i64 = form.get("status").and_then(|s| s.parse().ok()).unwrap_or(-1);
        let status = if status_code >= 100 {
            PaymentStatusKind::Completed
        } else if status_code >= 0 {
            PaymentStatusKind::Pending
        } else {
            PaymentStatusKind::Failed
        };

        Ok(WebhookEvent {
            event_type: form.get("ipn_type").cloned().unwrap_or_else(|| "payment".into()),
            provider_ref: form.get("txn_id").cloned().unwrap_or_default(),
            status,
            amount: form.get("amount1").and_then(|s| s.parse().ok()),
            currency: form.get("currency1").cloned(),
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, _tx_id: &str, _amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        // CoinPayments does not support automated refunds via API; crypto transactions are irreversible
        Err(PaymentError::ProviderError(
            "CoinPayments: crypto refunds must be processed manually".to_string(),
        ))
    }
}
