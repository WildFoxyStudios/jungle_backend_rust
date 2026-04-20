pub mod providers;

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Trait ───────────────────────────────────────────────────────────────────

#[async_trait]
pub trait PaymentGateway: Send + Sync {
    fn provider_name(&self) -> &'static str;

    async fn create_session(
        &self,
        params: PaymentParams,
    ) -> Result<PaymentSession, PaymentError>;

    async fn verify_payment(
        &self,
        reference: &str,
    ) -> Result<PaymentStatus, PaymentError>;

    async fn handle_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> Result<WebhookEvent, PaymentError>;

    async fn refund(
        &self,
        tx_id: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError>;
}

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentParams {
    pub amount: Decimal,
    pub currency: String,
    pub description: String,
    pub payment_type: String,
    pub return_url: String,
    pub cancel_url: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSession {
    pub provider: String,
    pub session_id: String,
    pub redirect_url: String,
    pub provider_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatusKind {
    Pending,
    Completed,
    Failed,
    Cancelled,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatus {
    pub provider_ref: String,
    pub status: PaymentStatusKind,
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event_type: String,
    pub provider_ref: String,
    pub status: PaymentStatusKind,
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResult {
    pub provider_ref: String,
    pub refunded_amount: Decimal,
    pub status: String,
}

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Invalid webhook signature")]
    InvalidSignature,
    #[error("Payment not found: {0}")]
    NotFound(String),
    #[error("Refund failed: {0}")]
    RefundFailed(String),
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
}

// ─── Factory ─────────────────────────────────────────────────────────────────

pub fn create_gateway(provider: &str) -> Result<Box<dyn PaymentGateway>, PaymentError> {
    match provider {
        "stripe" => Ok(Box::new(providers::stripe::StripeGateway::from_env())),
        "paypal" => Ok(Box::new(providers::paypal::PayPalGateway::from_env())),
        "authorize_net" | "authorize" | "authorizenet" => {
            Ok(Box::new(providers::authorize_net::AuthorizeNetGateway::from_env()))
        }
        "paystack" => Ok(Box::new(providers::paystack::PayStackGateway::from_env())),
        "flutterwave" => Ok(Box::new(providers::flutterwave::FlutterwaveGateway::from_env())),
        "razorpay" => Ok(Box::new(providers::razorpay::RazorpayGateway::from_env())),
        "coinbase" => Ok(Box::new(providers::coinbase::CoinbaseGateway::from_env())),
        "braintree" => Ok(Box::new(providers::braintree::BraintreeGateway::from_env())),
        "bank_transfer" => Ok(Box::new(providers::bank_transfer::BankTransferGateway::new())),
        "iyzipay" => Ok(Box::new(providers::iyzipay::IyzipayGateway::from_env())),
        "cashfree" => Ok(Box::new(providers::cashfree::CashfreeGateway::from_env())),
        "yoomoney" => Ok(Box::new(providers::yoomoney::YooMoneyGateway::from_env())),
        "aamarpay" => Ok(Box::new(providers::aamarpay::AamarPayGateway::from_env())),
        "fortumo" => Ok(Box::new(providers::fortumo::FortumoGateway::from_env())),
        "2checkout" => Ok(Box::new(providers::twocheckout::TwoCheckoutGateway::from_env())),
        "coinpayments" => Ok(Box::new(providers::coinpayments::CoinPaymentsGateway::from_env())),
        "payfast" => Ok(Box::new(providers::payfast::PayFastGateway::from_env())),
        "paysera" => Ok(Box::new(providers::paysera::PayseraGateway::from_env())),
        "securionpay" => Ok(Box::new(providers::securionpay::SecurionPayGateway::from_env())),
        "ngenius" => Ok(Box::new(providers::ngenius::NGeniusGateway::from_env())),
        "paypro_bitcoin" => Ok(Box::new(providers::paypro::PayProBitcoinGateway::from_env())),
        _ => Err(PaymentError::UnsupportedProvider(provider.into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_gateway_known_providers() {
        let known = [
            "stripe", "paypal", "paystack", "flutterwave", "razorpay",
            "coinbase", "braintree", "bank_transfer", "iyzipay",
        ];
        for name in known {
            assert!(create_gateway(name).is_ok(), "Failed to create gateway: {}", name);
        }
    }

    #[test]
    fn test_create_gateway_unknown_provider() {
        let result = create_gateway("nonexistent");
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            PaymentError::UnsupportedProvider(name) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected UnsupportedProvider error"),
        }
    }

    #[test]
    fn test_provider_names() {
        assert_eq!(create_gateway("stripe").unwrap().provider_name(), "stripe");
        assert_eq!(create_gateway("paypal").unwrap().provider_name(), "paypal");
        assert_eq!(create_gateway("paystack").unwrap().provider_name(), "paystack");
        assert_eq!(create_gateway("bank_transfer").unwrap().provider_name(), "bank_transfer");
    }

    #[tokio::test]
    async fn test_bank_transfer_creates_session() {
        let gw = providers::bank_transfer::BankTransferGateway::new();
        let params = PaymentParams {
            amount: rust_decimal::Decimal::new(1000, 2),
            currency: "USD".into(),
            description: "Test payment".into(),
            payment_type: "pro_subscription".into(),
            return_url: "https://example.com/return".into(),
            cancel_url: "https://example.com/cancel".into(),
            metadata: std::collections::HashMap::new(),
        };
        let session = gw.create_session(params).await.unwrap();
        assert_eq!(session.provider, "bank_transfer");
        assert!(session.session_id.starts_with("BT-"));
    }

    #[tokio::test]
    async fn test_bank_transfer_verify_returns_pending() {
        let gw = providers::bank_transfer::BankTransferGateway::new();
        let status = gw.verify_payment("BT-test123").await.unwrap();
        assert_eq!(status.status, PaymentStatusKind::Pending);
    }
}
