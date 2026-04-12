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
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateFundingRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub description: Option<String>,
    pub goal_amount: rust_decimal::Decimal,
    pub image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DonateRequest {
    pub amount: rust_decimal::Decimal,
}

#[derive(Debug, Serialize, FromRow)]
pub struct FundingRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub user_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub goal_amount: rust_decimal::Decimal,
    pub raised_amount: rust_decimal::Decimal,
    pub image: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct DonationRow {
    pub id: i64,
    pub funding_id: i64,
    pub user_id: i64,
    pub amount: rust_decimal::Decimal,
    pub created_at: OffsetDateTime,
    pub username: String,
    pub avatar: String,
}

pub async fn list_fundings(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let fundings = sqlx::query_as::<_, FundingRow>(
        "SELECT * FROM fundings WHERE ($1::bigint IS NULL OR id < $1) ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = fundings.len() as i64 > limit;
    let fundings: Vec<_> = fundings.into_iter().take(limit as usize).collect();
    let next_cursor = fundings.last().map(|f| f.id.to_string());

    Ok(Json(json!({ "data": fundings, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn create_funding(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateFundingRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    if req.goal_amount <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("Goal amount must be positive".into()));
    }

    let funding = sqlx::query_as::<_, FundingRow>(
        "INSERT INTO fundings (user_id, title, description, goal_amount, image) VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.goal_amount)
    .bind(req.image.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": funding })))
}

pub async fn get_funding(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let funding = sqlx::query_as::<_, FundingRow>("SELECT * FROM fundings WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Funding not found".into()))?;

    let donations = sqlx::query_as::<_, DonationRow>(
        r#"
        SELECT fd.id, fd.funding_id, fd.user_id, fd.amount, fd.created_at,
            u.username, u.avatar
        FROM funding_donations fd JOIN users u ON u.id = fd.user_id
        WHERE fd.funding_id = $1
        ORDER BY fd.created_at DESC LIMIT 20
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let donor_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM funding_donations WHERE funding_id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "funding": funding,
            "recent_donations": donations,
            "donor_count": donor_count
        }
    })))
}

pub async fn delete_funding(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM fundings WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Funding not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM fundings WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn donate(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<DonateRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.amount <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("Donation amount must be positive".into()));
    }

    // Verify funding exists
    sqlx::query_scalar::<_, i64>("SELECT id FROM fundings WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Funding not found".into()))?;

    let mut tx = state.db.begin().await?;

    sqlx::query("INSERT INTO funding_donations (funding_id, user_id, amount) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(auth.user_id)
        .bind(req.amount)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE fundings SET raised_amount = raised_amount + $1 WHERE id = $2")
        .bind(req.amount)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "donated": true, "amount": req.amount } })))
}

/// GET /v1/fundings/my — list my funding campaigns
pub async fn my_fundings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let fundings = sqlx::query_as::<_, FundingRow>(
        "SELECT * FROM fundings WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": fundings })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateFundingRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub goal_amount: Option<rust_decimal::Decimal>,
    pub image: Option<String>,
}

/// PUT /v1/fundings/{id}
pub async fn update_funding(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateFundingRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM fundings WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Funding not found".into()))?;

    if owner != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    let funding = sqlx::query_as::<_, FundingRow>(
        r#"UPDATE fundings SET
               title = COALESCE($2, title),
               description = COALESCE($3, description),
               goal_amount = COALESCE($4, goal_amount),
               image = COALESCE($5, image)
           WHERE id = $1 RETURNING *"#,
    )
    .bind(id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.goal_amount)
    .bind(&req.image)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": funding })))
}

/// GET /v1/fundings/{id}/donations
pub async fn list_donations(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let donations = sqlx::query_as::<_, DonationRow>(
        r#"SELECT fd.id, fd.funding_id, fd.user_id, fd.amount, fd.created_at,
                  u.username, u.avatar
           FROM funding_donations fd JOIN users u ON u.id = fd.user_id
           WHERE fd.funding_id = $1 AND fd.id < $2
           ORDER BY fd.id DESC LIMIT $3"#,
    )
    .bind(id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = donations.len() as i64 > limit;
    let data: Vec<_> = donations.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|d| d.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}
