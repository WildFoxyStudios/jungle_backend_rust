use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus,
    PaymentStatusKind, RefundResult, WebhookEvent,
};

pub struct BraintreeGateway {
    merchant_id: String,
    public_key: String,
    private_key: String,
    sandbox: bool,
    client: reqwest::Client,
}

impl BraintreeGateway {
    pub fn from_env() -> Self {
        Self {
            merchant_id: std::env::var("BRAINTREE_MERCHANT_ID").unwrap_or_default(),
            public_key: std::env::var("BRAINTREE_PUBLIC_KEY").unwrap_or_default(),
            private_key: std::env::var("BRAINTREE_PRIVATE_KEY").unwrap_or_default(),
            sandbox: std::env::var("BRAINTREE_SANDBOX").unwrap_or_else(|_| "true".into()) == "true",
            client: reqwest::Client::new(),
        }
    }

    fn base_url(&self) -> &str {
        if self.sandbox {
            "https://api.sandbox.braintreegateway.com"
        } else {
            "https://api.braintreegateway.com"
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/merchants/{}{}", self.base_url(), self.merchant_id, path)
    }
}

#[async_trait]
impl PaymentGateway for BraintreeGateway {
    fn provider_name(&self) -> &'static str {
        "braintree"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let xml_body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <transaction>
                <type>sale</type>
                <amount>{}</amount>
                <order-id>{}</order-id>
            </transaction>"#,
            params.amount,
            params.metadata.get("order_id").cloned().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        );

        let resp = self
            .client
            .post(self.api_url("/transactions"))
            .basic_auth(&self.public_key, Some(&self.private_key))
            .header("Content-Type", "application/xml")
            .body(xml_body)
            .send()
            .await?;

        let text = resp.text().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let tx_id = extract_xml_value(&text, "id").unwrap_or_default();

        Ok(PaymentSession {
            provider: "braintree".into(),
            session_id: tx_id.clone(),
            redirect_url: params.return_url,
            provider_ref: Some(tx_id),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        let url = self.api_url(&format!("/transactions/{}", reference));
        let resp = self
            .client
            .get(&url)
            .basic_auth(&self.public_key, Some(&self.private_key))
            .header("Accept", "application/xml")
            .send()
            .await?;

        let text = resp.text().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status_str = extract_xml_value(&text, "status").unwrap_or_default();
        let status = match status_str.as_str() {
            "settled" | "settling" | "submitted_for_settlement" => PaymentStatusKind::Completed,
            "voided" | "gateway_rejected" => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        let amount = extract_xml_value(&text, "amount")
            .and_then(|a| a.parse::<Decimal>().ok());

        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status,
            amount,
            currency: extract_xml_value(&text, "currency-iso-code"),
        })
    }

    async fn handle_webhook(&self, payload: &[u8], _signature: &str) -> Result<WebhookEvent, PaymentError> {
        let text = String::from_utf8_lossy(payload);
        let kind = extract_xml_value(&text, "kind").unwrap_or_default();

        let status = match kind.as_str() {
            "settled" | "disbursed" => PaymentStatusKind::Completed,
            "failed" | "gateway_rejected" => PaymentStatusKind::Failed,
            _ => PaymentStatusKind::Pending,
        };

        Ok(WebhookEvent {
            event_type: kind,
            provider_ref: extract_xml_value(&text, "id").unwrap_or_default(),
            status,
            amount: None,
            currency: None,
            metadata: HashMap::new(),
        })
    }

    async fn refund(&self, tx_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError> {
        let xml_body = if let Some(amt) = amount {
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><transaction><amount>{}</amount></transaction>"#,
                amt
            )
        } else {
            String::new()
        };

        let url = self.api_url(&format!("/transactions/{}/refund", tx_id));
        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.public_key, Some(&self.private_key))
            .header("Content-Type", "application/xml")
            .body(xml_body)
            .send()
            .await?;

        let text = resp.text().await.map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        Ok(RefundResult {
            provider_ref: extract_xml_value(&text, "id").unwrap_or_else(|| tx_id.to_string()),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: extract_xml_value(&text, "status").unwrap_or_else(|| "submitted".into()),
        })
    }
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        let after = start + open.len();
        if let Some(end) = xml[after..].find(&close) {
            return Some(xml[after..after + end].trim().to_string());
        }
    }
    None
}
