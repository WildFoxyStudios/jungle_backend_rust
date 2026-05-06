use async_trait::async_trait;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::collections::HashMap;
use std::str::FromStr;

use crate::gateway::{
    PaymentError, PaymentGateway, PaymentParams, PaymentSession, PaymentStatus, PaymentStatusKind,
    RefundResult, WebhookEvent,
};

pub struct StripeGateway {
    client: stripe::Client,
}

impl StripeGateway {
    pub fn from_env() -> Self {
        let secret_key = std::env::var("STRIPE_SECRET_KEY").unwrap_or_default();
        Self {
            client: stripe::Client::new(secret_key),
        }
    }
}

#[async_trait]
impl PaymentGateway for StripeGateway {
    fn provider_name(&self) -> &'static str {
        "stripe"
    }

    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError> {
        let amount_cents = (params.amount * Decimal::from(100))
            .round_dp(2)
            .to_i64()
            .ok_or_else(|| PaymentError::ProviderError("amount overflow converting to cents".into()))?;

        let currency = stripe::Currency::from_str(&params.currency.to_lowercase())
            .unwrap_or(stripe::Currency::USD);

        let line_items = vec![stripe::CreateCheckoutSessionLineItems {
            quantity: Some(1),
            price_data: Some(stripe::CreateCheckoutSessionLineItemsPriceData {
                currency,
                product_data: Some(stripe::CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: params.description.clone(),
                    ..Default::default()
                }),
                unit_amount: Some(amount_cents),
                ..Default::default()
            }),
            ..Default::default()
        }];

        let mut create_session = stripe::CreateCheckoutSession::new();
        create_session.line_items = Some(line_items);
        create_session.mode = Some(stripe::CheckoutSessionMode::Payment);
        create_session.success_url = Some(&params.return_url);
        create_session.cancel_url = Some(&params.cancel_url);

        if !params.metadata.is_empty() {
            create_session.metadata = Some(params.metadata.clone());
        }

        let session = stripe::CheckoutSession::create(&self.client, create_session)
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        Ok(PaymentSession {
            provider: "stripe".into(),
            session_id: session.id.to_string(),
            redirect_url: session.url.unwrap_or_default(),
            provider_ref: session.payment_intent.map(|pi| pi.id().to_string()),
        })
    }

    async fn verify_payment(&self, session_id: &str) -> Result<PaymentStatus, PaymentError> {
        let sid = session_id
            .parse()
            .map_err(|_| PaymentError::ProviderError("invalid session id".into()))?;

        let session = stripe::CheckoutSession::retrieve(&self.client, &sid, &[])
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        let status = match session.payment_status {
            stripe::CheckoutSessionPaymentStatus::Paid => PaymentStatusKind::Completed,
            stripe::CheckoutSessionPaymentStatus::Unpaid => PaymentStatusKind::Pending,
            _ => PaymentStatusKind::Failed,
        };

        Ok(PaymentStatus {
            provider_ref: session
                .payment_intent
                .map(|pi| pi.id().to_string())
                .unwrap_or_default(),
            status,
            amount: session
                .amount_total
                .map(|a| Decimal::from(a) / Decimal::from(100)),
            currency: session.currency.map(|c| c.to_string().to_uppercase()),
        })
    }

    async fn handle_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> Result<WebhookEvent, PaymentError> {
        let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .map_err(|_| {
                PaymentError::ProviderError("STRIPE_WEBHOOK_SECRET not configured".into())
            })?;

        let payload_str = std::str::from_utf8(payload)
            .map_err(|_| PaymentError::ProviderError("invalid utf-8 payload".into()))?;

        let event = stripe::Webhook::construct_event(payload_str, signature, &webhook_secret)
            .map_err(|_| PaymentError::InvalidSignature)?;

        let (event_type, status, amount, currency, provider_ref) = match event.data.object {
            stripe::EventObject::CheckoutSession(ref session) => {
                if event.type_ == stripe::EventType::CheckoutSessionCompleted {
                    (
                        "checkout.session.completed".into(),
                        PaymentStatusKind::Completed,
                        session
                            .amount_total
                            .map(|a| Decimal::from(a) / Decimal::from(100)),
                        session.currency.map(|c| c.to_string().to_uppercase()),
                        session.id.to_string(),
                    )
                } else {
                    (
                        event.type_.to_string(),
                        PaymentStatusKind::Pending,
                        session
                            .amount_total
                            .map(|a| Decimal::from(a) / Decimal::from(100)),
                        session.currency.map(|c| c.to_string().to_uppercase()),
                        session.id.to_string(),
                    )
                }
            }
            stripe::EventObject::PaymentIntent(ref intent) => {
                let is_failed = event.type_ == stripe::EventType::PaymentIntentPaymentFailed;
                (
                    event.type_.to_string(),
                    if is_failed { PaymentStatusKind::Failed } else { PaymentStatusKind::Completed },
                    Some(Decimal::from(intent.amount) / Decimal::from(100)),
                    Some(intent.currency.to_string().to_uppercase()),
                    intent.id.to_string(),
                )
            }
            stripe::EventObject::Charge(ref charge) => (
                event.type_.to_string(),
                PaymentStatusKind::Refunded,
                Some(Decimal::from(charge.amount) / Decimal::from(100)),
                Some(charge.currency.to_string().to_uppercase()),
                charge.id.to_string(),
            ),
            _ => {
                return Err(PaymentError::ProviderError(
                    "unsupported webhook event object type".to_string()
                ));
            }
        };

        Ok(WebhookEvent {
            event_type,
            provider_ref,
            status,
            amount,
            currency,
            metadata: HashMap::new(),
        })
    }

    async fn refund(
        &self,
        payment_intent: &str,
        amount: Option<Decimal>,
    ) -> Result<RefundResult, PaymentError> {
        let pi_id: stripe::PaymentIntentId = payment_intent
            .parse()
            .map_err(|_| PaymentError::ProviderError("invalid payment intent id".into()))?;

        let mut create_refund = stripe::CreateRefund::new();
        create_refund.payment_intent = Some(pi_id);

        if let Some(amt) = amount {
            let cents = (amt * Decimal::from(100))
                .round_dp(2)
                .to_i64()
                .ok_or_else(|| PaymentError::ProviderError("refund amount overflow".into()))?;
            create_refund.amount = Some(cents);
        }

        let refund = stripe::Refund::create(&self.client, create_refund)
            .await
            .map_err(|e| PaymentError::ProviderError(e.to_string()))?;

        Ok(RefundResult {
            provider_ref: refund.id.to_string(),
            refunded_amount: Decimal::from(refund.amount) / Decimal::from(100),
            status: refund.status.unwrap_or_default(),
        })
    }
}
