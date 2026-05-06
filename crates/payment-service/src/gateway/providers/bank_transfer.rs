use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

/// Bank transfer gateway — generates payment instructions, admin confirms manually
/// via the admin panel (which reads from the database directly).
pub struct BankTransferGateway;

impl BankTransferGateway {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PaymentGateway for BankTransferGateway {
    fn provider_name(&self) -> &'static str {
        "bank_transfer"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let bank_name =
            std::env::var("BANK_TRANSFER_BANK_NAME").unwrap_or_else(|_| "Jungle Bank".into());
        let account_number =
            std::env::var("BANK_TRANSFER_ACCOUNT").unwrap_or_else(|_| "XXXX-XXXX-XXXX".into());
        let routing = std::env::var("BANK_TRANSFER_ROUTING").unwrap_or_default();

        let ref_code = format!(
            "BT-{}",
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("0000")
        );

        let instructions = format!(
            "Transfer {} {} to:\nBank: {}\nAccount: {}\nRouting: {}\nReference: {}\n\nInclude the reference code in your transfer description.",
            params.amount,
            params.currency.to_uppercase(),
            bank_name,
            account_number,
            routing,
            ref_code
        );

        tracing::info!(ref_code = %ref_code, amount = %params.amount, "Bank transfer session created");

        Ok(PaymentSession {
            provider: "bank_transfer".into(),
            session_id: ref_code.clone(),
            redirect_url: format!(
                "{}?instructions={}",
                params.return_url,
                urlencoding::encode(&instructions)
            ),
            provider_ref: Some(ref_code),
        })
    }

    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError> {
        tracing::info!(
            reference,
            "Bank transfer verification — requires admin confirmation"
        );
        Ok(PaymentStatus {
            provider_ref: reference.to_string(),
            status: PaymentStatusKind::Pending,
            amount: None,
            currency: None,
        })
    }

    async fn handle_webhook(
        &self,
        _payload: &[u8],
        _signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        Err(PaymentError::ProviderError(
            "Bank transfer does not support webhooks — use admin panel to confirm".into(),
        ))
    }

    async fn refund(
        &self,
        tx_id: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        tracing::warn!(tx_id, "Bank transfer refund requires manual processing");
        Ok(RefundResult {
            provider_ref: tx_id.to_string(),
            refunded_amount: amount.unwrap_or(Decimal::ZERO),
            status: "manual_processing".to_string(),
        })
    }
}
