use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct PaymentListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub status: Option<String>,
    pub provider: Option<String>,
}

pub async fn list_transactions(
    State(state): State<AppState>,
    Query(q): Query<PaymentListQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let offset = (q.page.unwrap_or(1) - 1).max(0) * limit;

    let rows = sqlx::query_as::<_, (i64, i64, rust_decimal::Decimal, String, String, String, time::OffsetDateTime)>(
        r#"SELECT pt.id, pt.user_id, pt.amount, pt.currency, pt.provider, pt.status, pt.created_at
        FROM payment_transactions pt
        WHERE ($1::text IS NULL OR pt.status = $1)
          AND ($2::text IS NULL OR pt.provider = $2)
        ORDER BY pt.created_at DESC
        LIMIT $3 OFFSET $4"#,
    )
    .bind(q.status.as_deref())
    .bind(q.provider.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, user_id, amount, currency, provider, status, created_at)| {
            json!({
                "id": id,
                "user_id": user_id,
                "amount": amount.to_string(),
                "currency": currency,
                "provider": provider,
                "status": status,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn list_pending_withdrawals(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, i64, rust_decimal::Decimal, String, String, time::OffsetDateTime)>(
        r#"SELECT w.id, w.user_id, w.amount, w.method, w.status, w.created_at
        FROM withdrawal_requests w
        WHERE w.status = 'pending'
        ORDER BY w.created_at ASC"#,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, user_id, amount, method, status, created_at)| {
            json!({
                "id": id,
                "user_id": user_id,
                "amount": amount.to_string(),
                "method": method,
                "status": status,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn approve_withdrawal(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE withdrawal_requests SET status = 'approved', reviewed_at = NOW() WHERE id = $1 AND status = 'pending'",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Withdrawal not found or already processed".into()));
    }

    Ok(Json(json!({ "data": { "approved": true } })))
}

pub async fn reject_withdrawal(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Refund to user wallet
    let row = sqlx::query_as::<_, (i64, rust_decimal::Decimal)>(
        "SELECT user_id, amount FROM withdrawal_requests WHERE id = $1 AND status = 'pending'",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Withdrawal not found".into()))?;

    let (user_id, amount) = row;

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "UPDATE withdrawal_requests SET status = 'rejected', reviewed_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE users SET wallet = wallet + $1 WHERE id = $2")
        .bind(amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "rejected": true, "refunded": true } })))
}

pub async fn list_pro_plans(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, String, rust_decimal::Decimal, i32, bool)>(
        "SELECT id, plan_type, title, price, period_days, is_active FROM pro_plans ORDER BY price ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, plan_type, title, price, period_days, active)| {
            json!({
                "id": id,
                "plan_type": plan_type,
                "title": title,
                "price": price.to_string(),
                "period_days": period_days,
                "is_active": active
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct UpsertProPlanRequest {
    pub plan_type: String,
    pub title: String,
    pub price: String,
    pub period_days: Option<i32>,
}

/// GET /v1/admin/payments/stats — payment statistics overview
pub async fn payment_stats(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    // Total revenue
    let total_revenue: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM payment_transactions WHERE status = 'completed'",
    )
    .fetch_one(&state.db)
    .await?;

    // Revenue last 30 days
    let revenue_30d: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM payment_transactions WHERE status = 'completed' AND created_at >= NOW() - INTERVAL '30 days'",
    )
    .fetch_one(&state.db)
    .await?;

    // Counts by status
    let status_counts = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*) as count FROM payment_transactions GROUP BY status ORDER BY count DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let status_map: serde_json::Map<String, Value> = status_counts
        .into_iter()
        .map(|(s, c)| (s, Value::Number(c.into())))
        .collect();

    // Top providers
    let top_providers = sqlx::query_as::<_, (String, i64, rust_decimal::Decimal)>(
        r#"SELECT provider, COUNT(*) as tx_count, COALESCE(SUM(amount), 0) as total
           FROM payment_transactions WHERE status = 'completed'
           GROUP BY provider ORDER BY total DESC LIMIT 10"#,
    )
    .fetch_all(&state.db)
    .await?;

    let providers: Vec<Value> = top_providers
        .into_iter()
        .map(|(p, c, t)| json!({"provider": p, "transactions": c, "total": t}))
        .collect();

    // Pending withdrawals
    let pending_withdrawals: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM payment_transactions WHERE type = 'withdrawal' AND status = 'pending'",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "total_revenue": total_revenue,
            "revenue_30d": revenue_30d,
            "status_counts": status_map,
            "top_providers": providers,
            "pending_withdrawals": pending_withdrawals,
        }
    })))
}

pub async fn upsert_pro_plan(
    State(state): State<AppState>,
    Json(req): Json<UpsertProPlanRequest>,
) -> Result<Json<Value>, ApiError> {
    let price: rust_decimal::Decimal = req
        .price
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid price".into()))?;

    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO pro_plans (plan_type, title, price, period_days)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (plan_type) DO UPDATE SET title = $2, price = $3, period_days = $4
        RETURNING id"#,
    )
    .bind(&req.plan_type)
    .bind(&req.title)
    .bind(price)
    .bind(req.period_days.unwrap_or(30))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}
