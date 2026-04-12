use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub plan_type: i16,
    pub period: String,
}

#[derive(Debug, Deserialize)]
pub struct RefundRequest {
    pub pro_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BankReceiptRequest {
    pub receipt_file: String,
    pub price: rust_decimal::Decimal,
    pub description: Option<String>,
    pub mode: Option<String>,
}

pub async fn list_plans(
    State(_state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    // Plans are configured via site_config; return hardcoded defaults
    Ok(Json(json!({
        "data": [
            { "type": 1, "name": "Star", "monthly_price": 4.99, "yearly_price": 49.99 },
            { "type": 2, "name": "Hot", "monthly_price": 9.99, "yearly_price": 99.99 },
            { "type": 3, "name": "Ultima", "monthly_price": 14.99, "yearly_price": 149.99 },
            { "type": 4, "name": "VIP", "monthly_price": 29.99, "yearly_price": 299.99 }
        ]
    })))
}

pub async fn subscribe(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<Value>, ApiError> {
    if !(1..=4).contains(&req.plan_type) {
        return Err(ApiError::BadRequest("Invalid plan type (1-4)".into()));
    }

    let valid_periods = ["weekly", "monthly", "yearly", "lifetime"];
    if !valid_periods.contains(&req.period.as_str()) {
        return Err(ApiError::BadRequest(format!("Invalid period. Use: {}", valid_periods.join(", "))));
    }

    let price = match (req.plan_type, req.period.as_str()) {
        (1, "monthly") => rust_decimal::Decimal::new(499, 2),
        (1, "yearly") => rust_decimal::Decimal::new(4999, 2),
        (2, "monthly") => rust_decimal::Decimal::new(999, 2),
        (2, "yearly") => rust_decimal::Decimal::new(9999, 2),
        (3, "monthly") => rust_decimal::Decimal::new(1499, 2),
        (3, "yearly") => rust_decimal::Decimal::new(14999, 2),
        (4, "monthly") => rust_decimal::Decimal::new(2999, 2),
        (4, "yearly") => rust_decimal::Decimal::new(29999, 2),
        _ => rust_decimal::Decimal::new(999, 2),
    };

    let duration_days: i64 = match req.period.as_str() {
        "weekly" => 7,
        "monthly" => 30,
        "yearly" => 365,
        "lifetime" => 36500,
        _ => 30,
    };

    let mut tx = state.db.begin().await?;

    // Check balance
    let balance = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(balance, 0) FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;

    if balance < price {
        return Err(ApiError::BadRequest("Insufficient balance".into()));
    }

    // Debit
    sqlx::query("UPDATE users SET balance = balance - $1, is_pro = TRUE WHERE id = $2")
        .bind(price)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    // Deactivate existing sub
    sqlx::query("UPDATE pro_subscriptions SET is_active = FALSE WHERE user_id = $1 AND is_active = TRUE")
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    // Create sub
    sqlx::query(
        "INSERT INTO pro_subscriptions (user_id, plan_type, period, amount_paid, expires_at) VALUES ($1, $2, $3, $4, NOW() + make_interval(days => $5))",
    )
    .bind(auth.user_id)
    .bind(req.plan_type)
    .bind(&req.period)
    .bind(price)
    .bind(duration_days as i32)
    .fetch_optional(&mut *tx)
    .await?;

    // Transaction record
    sqlx::query(
        "INSERT INTO payment_transactions (user_id, amount, currency, provider, type, status) VALUES ($1, $2, 'USD', 'wallet', 'pro_subscription', 'completed')",
    )
    .bind(auth.user_id)
    .bind(price)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "subscribed": true, "plan_type": req.plan_type, "period": req.period } })))
}

/// POST /v1/payments/pro/refund — User requests a pro membership refund
pub async fn request_refund(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RefundRequest>,
) -> Result<Json<Value>, ApiError> {
    // Must be a pro user
    let is_pro: i16 = sqlx::query_scalar("SELECT COALESCE(is_pro, 0) FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    if is_pro == 0 {
        return Err(ApiError::BadRequest("You are not a pro member".into()));
    }

    // Check no existing pending request
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM refund_requests WHERE user_id = $1 AND status = 0)",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if existing {
        return Err(ApiError::BadRequest("You already have a pending refund request".into()));
    }

    let valid_types = ["star", "hot", "ultima", "vip"];
    if !valid_types.contains(&req.pro_type.as_str()) {
        return Err(ApiError::BadRequest("Invalid pro_type".into()));
    }

    // Get last payment hash
    let order_hash: String = sqlx::query_scalar(
        "SELECT COALESCE(provider_ref, id::text) FROM payment_transactions WHERE user_id = $1 AND type = 'pro_subscription' ORDER BY id DESC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or_else(|| "N/A".to_string());

    sqlx::query(
        "INSERT INTO refund_requests (user_id, order_hash_id, pro_type, description, status) VALUES ($1, $2, $3, $4, 0)",
    )
    .bind(auth.user_id)
    .bind(&order_hash)
    .bind(req.pro_type.trim())
    .bind(req.description.as_deref().unwrap_or(""))
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "submitted": true, "message": "Your refund request has been submitted. You will be notified once it is reviewed." } })))
}

/// POST /v1/payments/bank-receipt — Upload a bank transfer receipt
pub async fn upload_bank_receipt(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BankReceiptRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.receipt_file.trim().is_empty() {
        return Err(ApiError::BadRequest("receipt_file is required".into()));
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO bank_receipts (user_id, description, price, mode, receipt_file) VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.price)
    .bind(req.mode.as_deref().unwrap_or("wallet"))
    .bind(req.receipt_file.trim())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id, "message": "Bank receipt uploaded. An admin will review it shortly." } })))
}

pub async fn cancel(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let affected = sqlx::query(
        "UPDATE pro_subscriptions SET is_active = FALSE WHERE user_id = $1 AND is_active = TRUE",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::BadRequest("No active subscription found".into()));
    }

    sqlx::query("UPDATE users SET is_pro = FALSE WHERE id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "cancelled": true } })))
}
