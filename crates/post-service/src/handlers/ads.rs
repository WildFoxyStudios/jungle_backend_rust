use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct CreateAdRequest {
    pub post_id: i64,
    pub audience: Option<String>,
    pub budget: String,
    pub bid_type: Option<String>,
}

/// POST /v1/ads — create a user ad
pub async fn create_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateAdRequest>,
) -> Result<Json<Value>, ApiError> {
    let budget: rust_decimal::Decimal = req
        .budget
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid budget".into()))?;

    if budget <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("Budget must be positive".into()));
    }

    // Check user has enough wallet balance
    let wallet = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(wallet, 0) FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if wallet < budget {
        return Err(ApiError::BadRequest("Insufficient wallet balance".into()));
    }

    let mut tx = state.db.begin().await?;

    let ad_id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO user_ads (user_id, post_id, audience, budget, bid_type, status)
        VALUES ($1, $2, $3, $4, $5, 'active')
        RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(req.post_id)
    .bind(req.audience.as_deref().unwrap_or("all"))
    .bind(budget)
    .bind(req.bid_type.as_deref().unwrap_or("views"))
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE users SET wallet = wallet - $1 WHERE id = $2")
        .bind(budget)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "id": ad_id } })))
}

/// GET /v1/ads/my — list my ads
pub async fn my_ads(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, i64, String, rust_decimal::Decimal, i64, i64, String, time::OffsetDateTime)>(
        r#"SELECT id, post_id, audience, budget, impressions, clicks, status, created_at
        FROM user_ads WHERE user_id = $1
        ORDER BY created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, post_id, audience, budget, impressions, clicks, status, created_at)| {
            json!({
                "id": id,
                "post_id": post_id,
                "audience": audience,
                "budget": budget.to_string(),
                "impressions": impressions,
                "clicks": clicks,
                "status": status,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

/// GET /v1/ads/{id}/stats
pub async fn ad_stats(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64, rust_decimal::Decimal, i64, i64, String)>(
        "SELECT post_id, budget, impressions, clicks, status FROM user_ads WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Ad not found".into()))?;

    let (post_id, budget, impressions, clicks, status) = row;
    let ctr = if impressions > 0 {
        clicks as f64 / impressions as f64 * 100.0
    } else {
        0.0
    };

    Ok(Json(json!({
        "data": {
            "post_id": post_id,
            "budget_remaining": budget.to_string(),
            "impressions": impressions,
            "clicks": clicks,
            "ctr": format!("{:.2}%", ctr),
            "status": status
        }
    })))
}

/// DELETE /v1/ads/{id} — cancel ad, refund remaining budget
pub async fn cancel_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let budget = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT budget FROM user_ads WHERE id = $1 AND user_id = $2 AND status = 'active'",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Active ad not found".into()))?;

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE user_ads SET status = 'cancelled' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    if budget > rust_decimal::Decimal::ZERO {
        sqlx::query("UPDATE users SET wallet = wallet + $1 WHERE id = $2")
            .bind(budget)
            .bind(auth.user_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Json(json!({ "data": { "cancelled": true, "refunded": budget.to_string() } })))
}

/// POST /v1/ads/{id}/view — Record an ad impression/view (PHP: ads.php `rads-v`)
pub async fn record_ad_view(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(ad_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        "UPDATE user_ads SET views = views + 1, budget = CASE WHEN bid_type = 'views' THEN GREATEST(budget - 0.001, 0) ELSE budget END WHERE id = $1 AND status = 'active'",
    )
    .bind(ad_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "recorded": true } })))
}

/// GET /v1/ads/estimated-audience?gender=all&country=US — Estimated audience size (PHP: ads.php `get_estimated_users`)
pub async fn get_estimated_audience(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<EstimatedAudienceParams>,
) -> Result<Json<Value>, ApiError> {
    let _ = auth; // logged in required

    let mut conditions = vec!["deleted_at IS NULL".to_string(), "is_active = TRUE".to_string()];

    if let Some(ref gender) = params.gender
        && gender != "all"
        && !gender.is_empty()
    {
        conditions.push(format!("gender = '{}'", gender.replace('\'', "")));
    }

    if let Some(ref country) = params.country
        && !country.is_empty()
    {
        conditions.push(format!(
            "country_id IN (SELECT id FROM categories WHERE slug = '{}' AND type = 'country' LIMIT 1)",
            country.replace('\'', "")
        ));
    }

    if let Some(min_age) = params.min_age {
        conditions.push(format!(
            "EXTRACT(YEAR FROM AGE(birthday)) >= {}",
            min_age
        ));
    }
    if let Some(max_age) = params.max_age {
        conditions.push(format!(
            "EXTRACT(YEAR FROM AGE(birthday)) <= {}",
            max_age
        ));
    }

    let where_clause = conditions.join(" AND ");
    let count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM users WHERE {}",
        where_clause
    ))
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(json!({ "data": { "estimated_users": count } })))
}

#[derive(Debug, serde::Deserialize)]
pub struct EstimatedAudienceParams {
    pub gender: Option<String>,
    pub country: Option<String>,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
}
