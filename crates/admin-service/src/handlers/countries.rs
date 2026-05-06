use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct CountryAdminRow {
    pub id: i64,
    pub name: String,
    pub iso_code: String,
    pub iso3_code: Option<String>,
    pub phone_code: Option<String>,
    pub flag_emoji: Option<String>,
    pub currency_code: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct CountryFilter {
    pub q: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCountryRequest {
    pub name: Option<String>,
    pub phone_code: Option<String>,
    pub flag_emoji: Option<String>,
    pub currency_code: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

pub async fn list_countries(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(filter): Query<CountryFilter>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = if let Some(ref q) = filter.q {
        let pattern = format!("%{}%", q.to_lowercase());
        sqlx::query_as::<_, CountryAdminRow>(
            "SELECT id, name, iso_code, iso3_code, phone_code, flag_emoji, currency_code, is_active, sort_order
             FROM countries
             WHERE LOWER(name) LIKE $1
             ORDER BY sort_order ASC, name ASC",
        )
        .bind(&pattern)
        .fetch_all(&state.db)
        .await?
    } else if filter.active.unwrap_or(false) {
        sqlx::query_as::<_, CountryAdminRow>(
            "SELECT id, name, iso_code, iso3_code, phone_code, flag_emoji, currency_code, is_active, sort_order
             FROM countries
             WHERE is_active = TRUE
             ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, CountryAdminRow>(
            "SELECT id, name, iso_code, iso3_code, phone_code, flag_emoji, currency_code, is_active, sort_order
             FROM countries
             ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(json!({ "data": rows })))
}

pub async fn update_country(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCountryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, CountryAdminRow>(
        "UPDATE countries SET
            name = COALESCE($2, name),
            phone_code = COALESCE($3, phone_code),
            flag_emoji = COALESCE($4, flag_emoji),
            currency_code = COALESCE($5, currency_code),
            is_active = COALESCE($6, is_active),
            sort_order = COALESCE($7, sort_order)
         WHERE id = $1
         RETURNING id, name, iso_code, iso3_code, phone_code, flag_emoji, currency_code, is_active, sort_order",
    )
    .bind(id)
    .bind(req.name.as_deref().map(|s| s.trim()))
    .bind(req.phone_code.as_deref().map(|s| s.trim()))
    .bind(req.flag_emoji.as_deref().map(|s| s.trim()))
    .bind(req.currency_code.as_deref().map(|s| s.trim()))
    .bind(req.is_active)
    .bind(req.sort_order)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Country not found".into()))?;

    invalidate_countries_cache(&state).await;

    Ok(Json(json!({ "data": row })))
}

async fn invalidate_countries_cache(state: &AppState) {
    let mut redis = state.redis.clone();
    // Delete the common cache keys; search variants will expire naturally (short TTL)
    let _: Result<(), _> = redis::cmd("DEL")
        .arg("countries:all")
        .query_async(&mut redis)
        .await;
}
