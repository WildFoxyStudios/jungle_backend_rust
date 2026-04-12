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
pub struct CreateOfferRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub description: Option<String>,
    pub image: Option<String>,
    pub discount_type: Option<String>,
    pub discount_value: rust_decimal::Decimal,
    pub currency: Option<String>,
    pub expires_at: Option<String>,
    pub page_id: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct OfferRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub user_id: i64,
    pub page_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub image: String,
    pub discount_type: String,
    pub discount_value: rust_decimal::Decimal,
    pub currency: String,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

pub async fn list_offers(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let offers = sqlx::query_as::<_, OfferRow>(
        r#"
        SELECT * FROM offers
        WHERE (expires_at IS NULL OR expires_at > NOW())
          AND ($1::bigint IS NULL OR id < $1)
        ORDER BY id DESC LIMIT $2
        "#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = offers.len() as i64 > limit;
    let offers: Vec<_> = offers.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": offers, "meta": { "has_more": has_more } })))
}

pub async fn create_offer(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateOfferRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let expires_at = req.expires_at.as_deref().map(|s| {
        OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
            .map_err(|_| ApiError::BadRequest("Invalid expires_at format. Use RFC3339.".into()))
    }).transpose()?;

    let offer = sqlx::query_as::<_, OfferRow>(
        r#"
        INSERT INTO offers (user_id, page_id, title, description, image, discount_type, discount_value, currency, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(req.page_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.image.as_deref().unwrap_or(""))
    .bind(req.discount_type.as_deref().unwrap_or("percentage"))
    .bind(req.discount_value)
    .bind(req.currency.as_deref().unwrap_or("USD"))
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": offer })))
}

pub async fn get_offer(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let offer = sqlx::query_as::<_, OfferRow>("SELECT * FROM offers WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Offer not found".into()))?;

    Ok(Json(json!({ "data": offer })))
}

pub async fn delete_offer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM offers WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Offer not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM offers WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateOfferRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub discount_type: Option<String>,
    pub discount_value: Option<rust_decimal::Decimal>,
    pub expires_at: Option<String>,
}

/// PUT /v1/offers/{id}
pub async fn update_offer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateOfferRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM offers WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Offer not found".into()))?;

    if owner != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    let expires_at = req.expires_at.as_deref().map(|s| {
        OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
            .map_err(|_| ApiError::BadRequest("Invalid expires_at format".into()))
    }).transpose()?;

    let offer = sqlx::query_as::<_, OfferRow>(
        r#"UPDATE offers SET
               title = COALESCE($2, title),
               description = COALESCE($3, description),
               image = COALESCE($4, image),
               discount_type = COALESCE($5, discount_type),
               discount_value = COALESCE($6, discount_value),
               expires_at = COALESCE($7, expires_at)
           WHERE id = $1 RETURNING *"#,
    )
    .bind(id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(&req.image)
    .bind(&req.discount_type)
    .bind(req.discount_value)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": offer })))
}

// ── Nearby Offers (location-based) ───────────────────────────────

#[derive(Debug, Deserialize)]
pub struct NearbyOffersParams {
    pub lat: f64,
    pub lng: f64,
    pub radius_km: Option<f64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct NearbyOfferRow {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub image: String,
    pub discount_type: String,
    pub discount_value: rust_decimal::Decimal,
    pub currency: String,
    pub expires_at: Option<OffsetDateTime>,
    pub distance_km: f64,
}

/// GET /v1/offers/nearby — find offers near a location via page lat/lng
pub async fn nearby_offers(
    State(state): State<AppState>,
    Query(params): Query<NearbyOffersParams>,
) -> Result<Json<Value>, ApiError> {
    let radius = params.radius_km.unwrap_or(50.0);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    let offers = sqlx::query_as::<_, NearbyOfferRow>(
        r#"
        SELECT * FROM (
            SELECT o.id, o.title, o.description, o.image,
                   o.discount_type, o.discount_value, o.currency, o.expires_at,
                   (6371 * acos(LEAST(1.0,
                        cos(radians($1)) * cos(radians(u.lat)) *
                        cos(radians(u.lng) - radians($2)) +
                        sin(radians($1)) * sin(radians(u.lat))
                   ))) AS distance_km
            FROM offers o
            JOIN users u ON u.id = o.user_id
            WHERE (o.expires_at IS NULL OR o.expires_at > NOW())
              AND u.lat IS NOT NULL AND u.lng IS NOT NULL
        ) sub
        WHERE distance_km <= $3
        ORDER BY distance_km ASC
        LIMIT $4
        "#,
    )
    .bind(params.lat)
    .bind(params.lng)
    .bind(radius)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": offers })))
}

/// GET /v1/offers/my
pub async fn my_offers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let offers = sqlx::query_as::<_, OfferRow>(
        "SELECT * FROM offers WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": offers })))
}
