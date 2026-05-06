use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

pub struct NGeniusGateway {
    api_key: String,
    outlet_ref: String,
    base_url: String,
    client: reqwest::Client,
}

impl NGeniusGateway {
    pub fn from_env() -> Self {
        let sandbox = std::env::var("NGENIUS_SANDBOX").unwrap_or_else(|_| "true".into());
        let base_url = if sandbox == "true" {
            "https://api-gateway-uat.ngenius-payments.com".to_string()
        } else {
            "https://api-gateway.ngenius-payments.com".to_string()
        };
        Self {
            api_key: std::env::var("NGENIUS_API_KEY").unwrap_or_default(),
            outlet_ref: std::env::var("NGENIUS_OUTLET_REF").unwrap_or_default(),
            base_url,
            client: reqwest::Client::new(),
        }
    }

    async fn get_access_token(&self) -> Result<String, PaymentError> {
        let resp = self
            .client
            .post(format!("{}/identity/auth/access-token", self.base_url))
            .header("Authorization", format!("Basic {}", self.api_key))
            .header("Content-Type", "application/vnd.ni-identity.v1+json")
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;
        result["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                PaymentError::ProviderError("Failed to get N-Genius access token".into())
            })
    }
}

#[async_trait]
impl PaymentGateway for NGeniusGateway {
    fn provider_name(&self) -> &'static str {
        "ngenius"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let token = self.get_access_token().await?;
        let amount_minor = (params.amount * Decimal::from(100))
            .to_string()
            .parse::<i64>()
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let body = serde_json::json!({
            "action": "SALE",
            "amount": {
                "currencyCode": params.currency,
                "value": amount_minor
            },
            "merchantAttributes": {
                "redirectUrl": params.return_url,
                "skipConfirmationPage": true
            },
            "emailAddress": params.metadata.get("email").cloned().unwrap_or_default()
        });

        let resp = self
            .client
            .post(format!(
                "{}/transactions/outlets/{}/orders",
                self.base_url, self.outlet_ref
            ))
            .bearer_auth(&token)
            .header("Content-Type", "application/vnd.ni-payment.v2+json")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if let Some(errors) = result.get("errors") {
            return Err(PaymentError::ProviderError(
                errors[0]["message"]
                    .as_str()
                    .unwrap_or("N-Genius error")
                    .to_string(),
            ));
        }

        let order_ref = result["reference"].as_str().unwrap_or("").to_string();
        let payment_url = result["_links"]["payment"]["href"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(PaymentSession {
            provider: "ngenius".into(),
            session_id: order_ref.clone(),
            redirect_url: payment_url,
            provider_ref: Some(order_ref),
        })
    }

    async fn verify_payment(&self, order_ref: &str) -> Result<PaymentStatus, PaymentError> {
        let token = self.get_access_token().await?;

        let resp = self
            .client
            .get(format!(
                "{}/transactions/outlets/{}/orders/{}",
                self.base_url, self.outlet_ref, order_ref
            ))
            .bearer_auth(&token)
            .header("Accept", "application/vnd.ni-payment.v2+json")
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        let state = result["_embedded"]["payment"][0]["state"]
            .as_str()
            .unwrap_or("");

        let status = match state {
            "CAPTURED" | "PURCHASED" => PaymentStatusKind::Completed,
            "FAILED" => PaymentStatusKind::Failed,
            "REVERSED" | "REFUNDED" | "PARTIALLY_REFUNDED" => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(PaymentStatus {
            provider_ref: order_ref.to_string(),
            status,
            amount: result["amount"]["value"]
                .as_i64()
                .map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: result["amount"]["currencyCode"]
                .as_str()
                .map(|s| s.to_string()),
        })
    }

    async fn handle_webhook(
        &self,
        payload: &[u8],
        _signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        let body: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let event_name = body["eventName"].as_str().unwrap_or("").to_string();
        let order = &body["order"];

        let status = match event_name.as_str() {
            "CAPTURED" | "PURCHASED" => PaymentStatusKind::Completed,
            "FAILED" | "DECLINED" => PaymentStatusKind::Failed,
            "REFUNDED" | "PARTIALLY_REFUNDED" => PaymentStatusKind::Refunded,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: event_name,
            provider_ref: order["reference"].as_str().unwrap_or("").to_string(),
            status,
            amount: order["amount"]["value"]
                .as_i64()
                .map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: order["amount"]["currencyCode"]
                .as_str()
                .map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn refund(
        &self,
        order_ref: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        let token = self.get_access_token().await?;

        // Get the payment reference first
        let order_resp = self
            .client
            .get(format!(
                "{}/transactions/outlets/{}/orders/{}",
                self.base_url, self.outlet_ref, order_ref
            ))
            .bearer_auth(&token)
            .header("Accept", "application/vnd.ni-payment.v2+json")
            .send()
            .await?;

        let order: serde_json::Value = order_resp.json().await?;
        let payment_ref = order["_embedded"]["payment"][0]["_id"]
            .as_str()
            .unwrap_or("");

        let refund_url = format!(
            "{}/transactions/outlets/{}/orders/{}/payments/{}/refund",
            self.base_url, self.outlet_ref, order_ref, payment_ref
        );

        let mut body = serde_json::json!({});
        if let Some(amt) = amount {
            let minor = (amt * Decimal::from(100))
                .to_string()
                .parse::<i64>()
                .unwrap_or(0);
            body["amount"] = serde_json::json!({ "value": minor });
        }

        let resp = self
            .client
            .put(&refund_url)
            .bearer_auth(&token)
            .header("Content-Type", "application/vnd.ni-payment.v2+json")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["state"].as_str() == Some("FAILED") {
            return Err(PaymentError::RefundFailed(
                "N-Genius refund failed".to_string(),
            ));
        }

        Ok(RefundResult {
            provider_ref: result["_id"].as_str().unwrap_or("").to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: result["state"].as_str().unwrap_or("pending").to_string(),
        })
    }
}
