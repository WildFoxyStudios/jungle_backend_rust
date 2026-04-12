use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct AddFundsRequest {
    pub amount: rust_decimal::Decimal,
    pub provider: String,
    pub return_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub to_user_id: i64,
    pub amount: rust_decimal::Decimal,
}

pub async fn get_balance(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let balance = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(balance, 0) FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "balance": balance } })))
}

pub async fn add_funds(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AddFundsRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.amount <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("Amount must be positive".into()));
    }

    let gw = crate::gateway::create_gateway(&req.provider)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let params = crate::gateway::PaymentParams {
        amount: req.amount,
        currency: "USD".into(),
        description: "Wallet top-up".into(),
        payment_type: "wallet_topup".into(),
        return_url: req.return_url,
        cancel_url: req.cancel_url,
        metadata: [("user_id".into(), auth.user_id.to_string())].into(),
    };

    let session = gw
        .create_session(params)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    sqlx::query(
        "INSERT INTO payment_transactions (user_id, amount, currency, provider, provider_ref, type, status) VALUES ($1, $2, 'USD', $3, $4, 'wallet_topup', 'pending')",
    )
    .bind(auth.user_id)
    .bind(req.amount)
    .bind(&req.provider)
    .bind(&session.session_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "redirect_url": session.redirect_url,
            "session_id": session.session_id
        }
    })))
}

pub async fn transfer(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TransferRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.amount <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("Amount must be positive".into()));
    }

    if req.to_user_id == auth.user_id {
        return Err(ApiError::BadRequest("Cannot transfer to yourself".into()));
    }

    let mut tx = state.db.begin().await?;

    // Check balance
    let balance = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(balance, 0) FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;

    if balance < req.amount {
        return Err(ApiError::BadRequest("Insufficient balance".into()));
    }

    // Debit sender
    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
        .bind(req.amount)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    // Credit receiver
    sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
        .bind(req.amount)
        .bind(req.to_user_id)
        .execute(&mut *tx)
        .await?;

    // Record transactions
    sqlx::query(
        "INSERT INTO payment_transactions (user_id, amount, currency, provider, type, status, metadata) VALUES ($1, $2, 'USD', 'wallet', 'wallet_transfer', 'completed', $3)",
    )
    .bind(auth.user_id)
    .bind(req.amount)
    .bind(json!({ "to_user_id": req.to_user_id }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "transferred": true, "amount": req.amount } })))
}
