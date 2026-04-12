use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
pub struct WithdrawalRequest {
    pub amount: rust_decimal::Decimal,
    pub method: String,
    pub details: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub admin_note: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct WithdrawalRow {
    pub id: i64,
    pub user_id: i64,
    pub amount: rust_decimal::Decimal,
    pub method: String,
    pub details: Value,
    pub status: String,
    pub admin_note: String,
    pub created_at: OffsetDateTime,
    pub processed_at: Option<OffsetDateTime>,
}

pub async fn request_withdrawal(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<WithdrawalRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.amount <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("Amount must be positive".into()));
    }

    let valid_methods = ["paypal", "bank_transfer", "stripe", "crypto"];
    if !valid_methods.contains(&req.method.as_str()) {
        return Err(ApiError::BadRequest(format!("Invalid method. Use: {}", valid_methods.join(", "))));
    }

    // Check balance
    let balance = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(balance, 0) FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if balance < req.amount {
        return Err(ApiError::BadRequest("Insufficient balance".into()));
    }

    let mut tx = state.db.begin().await?;

    // Hold the funds
    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
        .bind(req.amount)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    let w = sqlx::query_as::<_, WithdrawalRow>(
        "INSERT INTO withdrawal_requests (user_id, amount, method, details) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(auth.user_id)
    .bind(req.amount)
    .bind(&req.method)
    .bind(&req.details)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": w })))
}

pub async fn list_withdrawals(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let rows = sqlx::query_as::<_, WithdrawalRow>(
        "SELECT * FROM withdrawal_requests WHERE user_id = $1 AND ($2::bigint IS NULL OR id < $2) ORDER BY id DESC LIMIT $3",
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": rows, "meta": { "has_more": has_more } })))
}

pub async fn update_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let valid = ["approved", "rejected", "processing", "completed"];
    if !valid.contains(&req.status.as_str()) {
        return Err(ApiError::BadRequest(format!("Invalid status. Use: {}", valid.join(", "))));
    }

    let w = sqlx::query_as::<_, WithdrawalRow>("SELECT * FROM withdrawal_requests WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Withdrawal not found".into()))?;

    // If rejecting, refund the held amount
    if req.status == "rejected" && w.status == "pending" {
        sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
            .bind(w.amount)
            .bind(w.user_id)
            .execute(&state.db)
            .await?;
    }

    sqlx::query(
        "UPDATE withdrawal_requests SET status = $1, admin_note = COALESCE($2, admin_note), processed_at = NOW() WHERE id = $3",
    )
    .bind(&req.status)
    .bind(&req.admin_note)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "status": req.status } })))
}
