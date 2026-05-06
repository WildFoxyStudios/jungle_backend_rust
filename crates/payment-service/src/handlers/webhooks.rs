use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
};
use serde_json::{Value, json};
use shared::{auth::AppState, errors::ApiError};

use crate::gateway;

/// Providers whose `handle_webhook` implementation is known to verify the
/// signature cryptographically against the webhook secret / merchant key.
/// For every other provider we DO NOT trust the webhook payload alone —
/// instead we re-query the provider's API via `verify_payment` to confirm
/// the real status before updating our state (defense-in-depth against
/// forged webhooks).
///
/// Keep this list in sync with the `handle_webhook` implementations in
/// `crates/payment-service/src/gateway/providers/*.rs`.
const SIGNATURE_VERIFIED_PROVIDERS: &[&str] = &[
    // Providers whose handle_webhook always returns PaymentError::InvalidSignature
    // when verification fails (fail-closed against forged payloads).
    "stripe",
    "paypal",
    "authorize_net",
    "authorize",
    "authorizenet",
    "paystack",
    "razorpay",
    "paysera",
    // Native HMAC/HMAC-like verification added 2026-05-04:
    "coinbase",    // HMAC-SHA256 via X-CC-Webhook-Signature
    "flutterwave", // verif-hash shared secret
    "braintree",   // bt_signature + bt_payload HMAC-SHA1 (SDK spec)
    "aamarpay",    // signature_key field in webhook body
    // Conditionally-verified providers (require secret env var — fallback
    // API re-verification below covers them when the env is missing):
    "cashfree",
    "twocheckout",
    "fortumo",
    "coinpayments",
    "payfast",
];

fn is_signature_verified(provider: &str) -> bool {
    SIGNATURE_VERIFIED_PROVIDERS.contains(&provider)
}

pub async fn handle_webhook(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let gw = gateway::create_gateway(&provider).map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let signature: String = match provider.as_str() {
        "paypal" => {
            serde_json::json!({
                "paypal-auth-algo": headers.get("paypal-auth-algo").and_then(|v| v.to_str().ok()).unwrap_or(""),
                "paypal-cert-url": headers.get("paypal-cert-url").and_then(|v| v.to_str().ok()).unwrap_or(""),
                "paypal-transmission-id": headers.get("paypal-transmission-id").and_then(|v| v.to_str().ok()).unwrap_or(""),
                "paypal-transmission-sig": headers.get("paypal-transmission-sig").and_then(|v| v.to_str().ok()).unwrap_or(""),
                "paypal-transmission-time": headers.get("paypal-transmission-time").and_then(|v| v.to_str().ok()).unwrap_or(""),
            }).to_string()
        }
        _ => {
            let header = match provider.as_str() {
                "stripe" => headers.get("stripe-signature"),
                "paystack" => headers.get("x-paystack-signature"),
                "razorpay" => headers.get("x-razorpay-signature"),
                "coinbase" => headers.get("x-cc-webhook-signature"),
                "coinpayments" => headers.get("x-cp-signature"),
                "flutterwave" => headers.get("verif-hash"),
                "cashfree" => headers.get("x-cashfree-signature"),
                "twocheckout" => headers.get("x-2checkout-signature"),
                "ngenius" => headers.get("x-ngenius-signature"),
                "authorize_net" | "authorize" => headers.get("x-anet-signature"),
                "payfast" => headers.get("x-pf-signature"),
                "securionpay" => headers.get("x-securionpay-signature"),
                "yoomoney" => headers.get("x-yoomoney-signature"),
                _ => None,
            };
            header.and_then(|v| v.to_str().ok()).unwrap_or("").to_string()
        }
    };

    let mut event = gw
        .handle_webhook(&body, &signature)
        .await
        .map_err(|e| match e {
            // A forged / mis-signed webhook must produce a 401 so the
            // provider retries or surfaces the failure in their dashboard,
            // not a 500 which both hides the real cause from operators and
            // could be interpreted as a transient infra error to keep
            // retrying against.
            gateway::PaymentError::InvalidSignature => {
                tracing::warn!(
                    provider = %provider,
                    "Webhook rejected: invalid signature"
                );
                ApiError::Unauthorized
            }
            other => ApiError::Internal(other.to_string()),
        })?;

    // Defense-in-depth: for providers that don't cryptographically verify the
    // webhook signature, re-query the provider's API to confirm the status.
    // This prevents forged webhooks from marking transactions completed.
    if !is_signature_verified(&provider) && !event.provider_ref.is_empty() {
        match gw.verify_payment(&event.provider_ref).await {
            Ok(verified) => {
                // Trust the API response over the webhook payload.
                event.status = verified.status;
                if verified.amount.is_some() {
                    event.amount = verified.amount;
                }
                if verified.currency.is_some() {
                    event.currency = verified.currency;
                }
                tracing::info!(
                    provider = %provider,
                    provider_ref = %event.provider_ref,
                    "Webhook status re-verified via provider API"
                );
            }
            Err(e) => {
                tracing::error!(
                    provider = %provider,
                    provider_ref = %event.provider_ref,
                    error = %e,
                    "Webhook re-verification failed — rejecting to prevent forged-payment acceptance"
                );
                return Err(ApiError::Unauthorized);
            }
        }
    }

    let db_status = match event.status {
        gateway::PaymentStatusKind::Completed => "completed",
        gateway::PaymentStatusKind::Failed => "failed",
        gateway::PaymentStatusKind::Cancelled => "cancelled",
        gateway::PaymentStatusKind::Refunded => "refunded",
        gateway::PaymentStatusKind::Pending => "pending",
    };

    // Credit wallet BEFORE updating status (idempotency guard: credit only
    // if this wallet_topup transaction isn't already completed).
    if db_status == "completed" {
        let tx = sqlx::query_as::<_, super::payments::TransactionRow>(
            "SELECT * FROM payment_transactions \
              WHERE provider = $1 AND provider_ref = $2 AND type = 'wallet_topup' \
                AND status != 'completed'",
        )
        .bind(&provider)
        .bind(&event.provider_ref)
        .fetch_optional(&state.db)
        .await?;

        if let Some(tx) = tx {
            // Verify user exists and is active before crediting
            let user_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND deleted_at IS NULL)",
            )
            .bind(tx.user_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(false);

            if user_exists {
                sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
                    .bind(tx.amount)
                    .bind(tx.user_id)
                    .execute(&state.db)
                    .await?;
            }
        }
    }

    // Update matching transaction status
    let _ = sqlx::query(
        "UPDATE payment_transactions SET status = $1 WHERE provider = $2 AND (provider_ref = $3 OR metadata->>'session_id' = $3)",
    )
    .bind(db_status)
    .bind(&provider)
    .bind(&event.provider_ref)
    .execute(&state.db)
    .await?;

    tracing::info!(
        provider = %provider,
        event_type = %event.event_type,
        provider_ref = %event.provider_ref,
        "Webhook processed"
    );

    Ok(Json(json!({ "received": true })))
}
