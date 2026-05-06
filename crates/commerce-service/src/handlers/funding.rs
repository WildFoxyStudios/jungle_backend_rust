use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use sqlx::{FromRow, Row};
use time::{Duration, OffsetDateTime};
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

#[derive(Debug, FromRow)]
struct FundingCreatorRow {
    id: i64,
    uuid: uuid::Uuid,
    username: String,
    first_name: String,
    last_name: String,
    avatar: String,
    is_verified: bool,
    is_pro: i16,
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

    Ok(Json(
        json!({ "data": fundings, "meta": { "cursor": next_cursor, "has_more": has_more } }),
    ))
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

    let creator = sqlx::query_as::<_, FundingCreatorRow>(
        r#"SELECT id, uuid, username, first_name, last_name, avatar, is_verified, is_pro
           FROM users WHERE id = $1"#,
    )
    .bind(funding.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("funding creator: {e}")))?;

    let donor_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM funding_donations WHERE funding_id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let goal_f64 = funding.goal_amount.to_f64().unwrap_or(0.0);
    let raised_f64 = funding.raised_amount.to_f64().unwrap_or(0.0);
    let is_goal_reached = funding.raised_amount >= funding.goal_amount;

    let fmt = &time::format_description::well_known::Rfc3339;
    let end_date = funding.created_at + Duration::days(365);
    let end_date_s = end_date
        .format(fmt)
        .map_err(|e| ApiError::Internal(format!("date format: {e}")))?;
    let created_at_s = funding
        .created_at
        .format(fmt)
        .map_err(|e| ApiError::Internal(format!("date format: {e}")))?;

    Ok(Json(json!({
        "data": {
            "id": funding.id,
            "title": funding.title,
            "description": funding.description.unwrap_or_default(),
            "cover": funding.image,
            "goal_amount": goal_f64,
            "raised_amount": raised_f64,
            "currency": "USD",
            "end_date": end_date_s,
            "creator": {
                "id": creator.id,
                "uuid": creator.uuid,
                "username": creator.username,
                "first_name": creator.first_name,
                "last_name": creator.last_name,
                "avatar": creator.avatar,
                "is_verified": creator.is_verified,
                "is_pro": creator.is_pro,
                "is_online": false
            },
            "donor_count": donor_count,
            "is_goal_reached": is_goal_reached,
            "created_at": created_at_s
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

    let donation_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM funding_donations WHERE funding_id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    if donation_count > 0 {
        return Err(ApiError::BadRequest(
            "Cannot delete funding campaign with existing donations".into(),
        ));
    }

    sqlx::query("DELETE FROM fundings WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn donate(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<DonateRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.amount <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest(
            "Donation amount must be positive".into(),
        ));
    }

    // Verify funding exists and donor is not the owner
    let funding_owner: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM fundings WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::NotFound("Funding not found".into()))?;

    if funding_owner == Some(auth.user_id) {
        return Err(ApiError::BadRequest(
            "You cannot donate to your own funding campaign".into(),
        ));
    }

    // Verify donor has sufficient wallet balance
    let balance: Option<rust_decimal::Decimal> =
        sqlx::query_scalar("SELECT balance FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    let balance = balance.unwrap_or(rust_decimal::Decimal::ZERO);
    if balance < req.amount {
        return Err(ApiError::BadRequest(
            "Insufficient wallet balance for donation".into(),
        ));
    }

    let mut tx = state.db.begin().await?;

    // Deduct from donor wallet
    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
        .bind(req.amount)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

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

    let creator_id = funding_owner.unwrap_or(0);
    if creator_id > 0 {
        let _ = state.event_bus.publish(&DomainEvent::FundingDonation {
            funding_id: id,
            donor_id: auth.user_id,
            creator_id,
            amount: req.amount.to_string(),
        }).await;

        // Check if goal was reached
        if let Ok(Some((raised, goal))) = sqlx::query_as::<_, (rust_decimal::Decimal, rust_decimal::Decimal)>(
            "SELECT raised_amount, goal_amount FROM fundings WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await
            && raised >= goal {
                let _ = state.event_bus.publish(&DomainEvent::FundingGoalReached {
                    funding_id: id,
                    creator_id,
                    goal_amount: goal.to_string(),
                }).await;
            }
    }

    Ok(Json(
        json!({ "data": { "donated": true, "amount": req.amount } }),
    ))
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

// ── Personal Causes ─────────────────────────────────────────────

pub async fn create_personal_cause(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let title = body["title"].as_str().unwrap_or("").to_string();
    if title.is_empty() {
        return Err(ApiError::BadRequest("Title is required".into()));
    }

    let goal_amount = rust_decimal::Decimal::from_i64(body["goal_amount"].as_i64().unwrap_or(0))
        .unwrap_or(rust_decimal::Decimal::ZERO);
    let funding_type = body["funding_type"].as_str().unwrap_or("personal_cause");
    let beneficiary_name = body["beneficiary_name"].as_str().map(String::from);
    let description = body["description"].as_str().unwrap_or("");

    let row = sqlx::query(
        "INSERT INTO fundings (user_id, title, description, goal_amount, funding_type, beneficiary_name, is_transparent, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, TRUE, NOW())
         RETURNING id",
    )
    .bind(auth.user_id)
    .bind(&title)
    .bind(description)
    .bind(goal_amount)
    .bind(funding_type)
    .bind(&beneficiary_name)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "title": title,
        "funding_type": funding_type,
    })))
}

// Withdraw funds (for personal causes with "as_donated" frequency)
#[derive(Deserialize)]
pub struct WithdrawFundsRequest {
    pub amount_cents: i64,
}

pub async fn withdraw_funding(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(funding_id): Path<i64>,
    Json(body): Json<WithdrawFundsRequest>,
) -> Result<Json<()>, ApiError> {
    let funding = sqlx::query("SELECT user_id, raised_amount FROM fundings WHERE id = $1")
        .bind(funding_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("DB error".into())
        })?
        .ok_or(ApiError::NotFound("Funding not found".into()))?;

    let owner_id: i64 = funding.get("user_id");
    if owner_id != auth.user_id {
        return Err(ApiError::Forbidden(
            "Only the campaign owner can withdraw".into(),
        ));
    }

    let raised: rust_decimal::Decimal = funding.get("raised_amount");
    let amount = rust_decimal::Decimal::from_i64(body.amount_cents)
        .unwrap_or(rust_decimal::Decimal::ZERO);
    if amount > raised {
        return Err(ApiError::BadRequest(
            "Cannot withdraw more than raised amount".into(),
        ));
    }

    // Update raised amount (financial data — must propagate errors)
    sqlx::query("UPDATE fundings SET raised_amount = raised_amount - $1 WHERE id = $2")
        .bind(amount)
        .bind(funding_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update funding raised_amount");
            ApiError::Internal("Database error".into())
        })?;

    Ok(Json(()))
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

    Ok(Json(
        json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } }),
    ))
}
