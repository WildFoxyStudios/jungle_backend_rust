use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

use crate::gateway;

pub async fn handle_webhook(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let gw = gateway::create_gateway(&provider)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let signature = headers
        .get("stripe-signature")
        .or_else(|| headers.get("x-paypal-signature"))
        .or_else(|| headers.get("x-paystack-signature"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let event = gw
        .handle_webhook(&body, signature)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let db_status = match event.status {
        gateway::PaymentStatusKind::Completed => "completed",
        gateway::PaymentStatusKind::Failed => "failed",
        gateway::PaymentStatusKind::Cancelled => "cancelled",
        gateway::PaymentStatusKind::Refunded => "refunded",
        gateway::PaymentStatusKind::Pending => "pending",
    };

    // Update matching transaction
    let result = sqlx::query(
        "UPDATE payment_transactions SET status = $1 WHERE provider = $2 AND (provider_ref = $3 OR metadata->>'session_id' = $3)",
    )
    .bind(db_status)
    .bind(&provider)
    .bind(&event.provider_ref)
    .execute(&state.db)
    .await?;

    // If completed and wallet_topup, credit user
    if db_status == "completed" && result.rows_affected() > 0 {
        let tx = sqlx::query_as::<_, super::payments::TransactionRow>(
            "SELECT * FROM payment_transactions WHERE provider = $1 AND provider_ref = $2 AND type = 'wallet_topup'",
        )
        .bind(&provider)
        .bind(&event.provider_ref)
        .fetch_optional(&state.db)
        .await?;

        if let Some(tx) = tx {
            sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
                .bind(tx.amount)
                .bind(tx.user_id)
                .execute(&state.db)
                .await?;
        }
    }

    tracing::info!(
        provider = %provider,
        event_type = %event.event_type,
        provider_ref = %event.provider_ref,
        "Webhook processed"
    );

    Ok(Json(json!({ "received": true })))
}
