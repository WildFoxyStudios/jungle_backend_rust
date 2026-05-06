use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
pub struct CreateTierRequest {
    pub name: String,
    pub price: rust_decimal::Decimal,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TierRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub price: rust_decimal::Decimal,
    pub description: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct SubscriptionRow {
    pub id: i64,
    pub creator_id: i64,
    pub subscriber_id: i64,
    pub tier_id: Option<i64>,
    pub amount: rust_decimal::Decimal,
    pub status: String,
    pub started_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub username: String,
    pub avatar: String,
}

pub async fn create_tier(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateTierRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.price <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("Price must be positive".into()));
    }

    let tier = sqlx::query_as::<_, TierRow>(
        "INSERT INTO creator_tiers (user_id, name, price, description) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(req.price)
    .bind(req.description.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": tier })))
}

pub async fn subscribe(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(creator_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if creator_id == auth.user_id {
        return Err(ApiError::BadRequest("Cannot subscribe to yourself".into()));
    }

    // Get the cheapest tier (or default)
    let tier = sqlx::query_as::<_, TierRow>(
        "SELECT * FROM creator_tiers WHERE user_id = $1 ORDER BY price ASC LIMIT 1",
    )
    .bind(creator_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Creator has no tiers".into()))?;

    // Check balance
    let balance = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(balance, 0) FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if balance < tier.price {
        return Err(ApiError::BadRequest("Insufficient balance".into()));
    }

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
        .bind(tier.price)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
        .bind(tier.price)
        .bind(creator_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO creator_subscriptions (creator_id, subscriber_id, tier_id, amount, expires_at)
        VALUES ($1, $2, $3, $4, NOW() + INTERVAL '30 days')
        ON CONFLICT (creator_id, subscriber_id)
        DO UPDATE SET tier_id = EXCLUDED.tier_id, amount = EXCLUDED.amount, expires_at = EXCLUDED.expires_at, status = 'active'
        "#,
    )
    .bind(creator_id)
    .bind(auth.user_id)
    .bind(tier.id)
    .bind(tier.price)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO payment_transactions (user_id, amount, currency, provider, type, status) VALUES ($1, $2, 'USD', 'wallet', 'creator_subscription', 'completed')",
    )
    .bind(auth.user_id)
    .bind(tier.price)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(
        json!({ "data": { "subscribed": true, "tier": tier.name, "amount": tier.price } }),
    ))
}

// ── Additional Creator Endpoints ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateTierRequest {
    pub name: Option<String>,
    pub price: Option<rust_decimal::Decimal>,
    pub description: Option<String>,
}

/// PUT /v1/payments/creator/tiers/{id}
pub async fn update_tier(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTierRequest>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        r#"UPDATE creator_tiers SET
            name = COALESCE($3, name),
            price = COALESCE($4, price),
            description = COALESCE($5, description)
        WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(req.price)
    .bind(&req.description)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Tier not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

/// DELETE /v1/payments/creator/tiers/{id}
pub async fn delete_tier(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM creator_tiers WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Tier not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

/// GET /v1/payments/creator/{user_id}/tiers — list tiers of a creator
pub async fn list_tiers(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let tiers = sqlx::query_as::<_, TierRow>(
        "SELECT * FROM creator_tiers WHERE user_id = $1 ORDER BY price ASC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": tiers })))
}

/// DELETE /v1/payments/creator/subscribe/{user_id} — unsubscribe from a creator
pub async fn unsubscribe(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(creator_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE creator_subscriptions SET status = 'cancelled' WHERE creator_id = $1 AND subscriber_id = $2 AND status = 'active'",
    )
    .bind(creator_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Active subscription not found".into()));
    }

    Ok(Json(json!({ "data": { "unsubscribed": true } })))
}

/// GET /v1/payments/creator/subscriptions — my subscriptions to creators
pub async fn my_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let subs = sqlx::query_as::<_, SubscriptionRow>(
        r#"
        SELECT cs.id, cs.creator_id, cs.subscriber_id, cs.tier_id, cs.amount, cs.status,
            cs.started_at, cs.expires_at, u.username, u.avatar
        FROM creator_subscriptions cs JOIN users u ON u.id = cs.creator_id
        WHERE cs.subscriber_id = $1 AND cs.status = 'active'
        ORDER BY cs.started_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": subs })))
}

pub async fn list_subscribers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let subs = sqlx::query_as::<_, SubscriptionRow>(
        r#"
        SELECT cs.id, cs.creator_id, cs.subscriber_id, cs.tier_id, cs.amount, cs.status,
            cs.started_at, cs.expires_at, u.username, u.avatar
        FROM creator_subscriptions cs JOIN users u ON u.id = cs.subscriber_id
        WHERE cs.creator_id = $1 AND cs.status = 'active'
        ORDER BY cs.started_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": subs })))
}
