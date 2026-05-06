use async_trait::async_trait;
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use sha2::Sha256;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

type HmacSha256 = Hmac<Sha256>;

pub struct IyzipayGateway {
    api_key: String,
    secret_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl IyzipayGateway {
    pub fn from_env() -> Self {
        let sandbox = std::env::var("IYZIPAY_SANDBOX").unwrap_or_else(|_| "true".into());
        let base_url = if sandbox == "true" {
            "https://sandbox-api.iyzipay.com".to_string()
        } else {
            "https://api.iyzipay.com".to_string()
        };
        Self {
            api_key: std::env::var("IYZIPAY_API_KEY").unwrap_or_default(),
            secret_key: std::env::var("IYZIPAY_SECRET_KEY").unwrap_or_default(),
            base_url,
            client: reqwest::Client::new(),
        }
    }

    fn generate_auth_header(&self, uri: &str, body: &str) -> String {
        let random_header = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
        let hash_str = format!(
            "{}{}{}{}",
            self.api_key, random_header, self.secret_key, body
        );
        let Ok(mut mac) = HmacSha256::new_from_slice(self.secret_key.as_bytes()) else {
            tracing::error!("Iyzipay generate_auth_header: HMAC key invalid");
            return String::new();
        };
        mac.update(hash_str.as_bytes());
        let signature = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            mac.finalize().into_bytes(),
        );
        let _ = uri;
        format!("IYZWS {}:{}", self.api_key, signature)
    }
}

#[async_trait]
impl PaymentGateway for IyzipayGateway {
    fn provider_name(&self) -> &'static str {
        "iyzipay"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let conversation_id = uuid::Uuid::new_v4().to_string();
        let basket_id = uuid::Uuid::new_v4().to_string();

        let body = serde_json::json!({
            "locale": "en",
            "conversationId": conversation_id,
            "price": params.amount.to_string(),
            "paidPrice": params.amount.to_string(),
            "currency": params.currency,
            "basketId": basket_id,
            "paymentGroup": "PRODUCT",
            "callbackUrl": params.return_url,
            "enabledInstallments": [1],
            "buyer": {
                "id": params.metadata.get("user_id").cloned().unwrap_or_else(|| "0".into()),
                "name": "User",
                "surname": "User",
                "email": params.metadata.get("email").cloned().unwrap_or_else(|| "user@example.com".into()),
                "identityNumber": "00000000000",
                "registrationAddress": "N/A",
                "ip": "127.0.0.1",
                "city": "N/A",
                "country": "N/A"
            },
            "shippingAddress": {
                "contactName": "User",
                "city": "N/A",
                "country": "N/A",
                "address": "N/A"
            },
            "billingAddress": {
                "contactName": "User",
                "city": "N/A",
                "country": "N/A",
                "address": "N/A"
            },
            "basketItems": [{
                "id": "item1",
                "name": params.description,
                "category1": "Digital",
                "itemType": "VIRTUAL",
                "price": params.amount.to_string()
            }]
        });

        let body_str = body.to_string();
        let auth = self.generate_auth_header(
            "/payment/iyzipos/checkoutform/initialize/auth/ecom",
            &body_str,
        );

        let resp = self
            .client
            .post(format!(
                "{}/payment/iyzipos/checkoutform/initialize/auth/ecom",
                self.base_url
            ))
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["status"].as_str() != Some("success") {
            return Err(PaymentError::ProviderError(
                result["errorMessage"]
                    .as_str()
                    .unwrap_or("Iyzipay error")
                    .to_string(),
            ));
        }

        Ok(PaymentSession {
            provider: "iyzipay".into(),
            session_id: result["token"].as_str().unwrap_or("").to_string(),
            redirect_url: result["checkoutFormContent"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            provider_ref: Some(conversation_id),
        })
    }

    async fn verify_payment(&self, token: &str) -> Result<PaymentStatus, PaymentError> {
        let body = serde_json::json!({ "token": token });
        let body_str = body.to_string();
        let auth =
            self.generate_auth_header("/payment/iyzipos/checkoutform/auth/ecom/detail", &body_str);

        let resp = self
            .client
            .post(format!(
                "{}/payment/iyzipos/checkoutform/auth/ecom/detail",
                self.base_url
            ))
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let status = match result["paymentStatus"].as_str() {
            Some("SUCCESS") => PaymentStatusKind::Completed,
            Some("FAILURE") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(PaymentStatus {
            provider_ref: result["paymentId"].as_str().unwrap_or("").to_string(),
            status,
            amount: result["paidPrice"].as_str().and_then(|s| s.parse().ok()),
            currency: result["currency"].as_str().map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(
        &self,
        payload: &[u8],
        _signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let token = body["token"].as_str().unwrap_or("").to_string();
        let status = match body["status"].as_str() {
            Some("success") => PaymentStatusKind::Completed,
            Some("failure") => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: "payment".to_string(),
            provider_ref: token,
            status,
            amount: None,
            currency: None,
            metadata: HashMap::new(),
        })
    }

    async fn refund(
        &self,
        payment_id: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        let body = serde_json::json!({
            "paymentTransactionId": payment_id,
            "price": amount.unwrap_or(Decimal::ZERO).to_string(),
            "currency": "TRY"
        });

        let body_str = body.to_string();
        let auth = self.generate_auth_header("/payment/refund", &body_str);

        let resp = self
            .client
            .post(format!("{}/payment/refund", self.base_url))
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["status"].as_str() != Some("success") {
            return Err(PaymentError::RefundFailed(
                result["errorMessage"]
                    .as_str()
                    .unwrap_or("Refund failed")
                    .to_string(),
            ));
        }

        Ok(RefundResult {
            provider_ref: result["paymentId"].as_str().unwrap_or("").to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: "succeeded".to_string(),
        })
    }
}
