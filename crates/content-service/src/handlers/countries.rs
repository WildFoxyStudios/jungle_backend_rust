use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::AppState,
    errors::ApiError,
};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
pub struct CountryRow {
    pub id: i64,
    pub name: String,
    pub iso_code: String,
    pub iso3_code: Option<String>,
    pub phone_code: Option<String>,
    pub flag_emoji: Option<String>,
    pub currency_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CountryFilter {
    pub q: Option<String>,
}

pub async fn list_countries(
    State(state): State<AppState>,
    Query(filter): Query<CountryFilter>,
) -> Result<Json<Value>, ApiError> {
    let cache_key = if let Some(ref q) = filter.q {
        format!("countries:search:{}", q.to_lowercase())
    } else {
        "countries:all".to_string()
    };

    let mut redis = state.redis.clone();

    if let Ok(Some(cached)) = redis::cmd("GET")
        .arg(&cache_key)
        .query_async::<Option<String>>(&mut redis)
        .await
    {
        if let Ok(value) = serde_json::from_str::<Value>(&cached) {
            return Ok(Json(value));
        }
    }

    let rows = if let Some(ref q) = filter.q {
        let pattern = format!("%{}%", q.to_lowercase());
        sqlx::query_as::<_, CountryRow>(
            "SELECT id, name, iso_code, iso3_code, phone_code, flag_emoji, currency_code
             FROM countries
             WHERE is_active = TRUE AND LOWER(name) LIKE $1
             ORDER BY sort_order ASC, name ASC",
        )
        .bind(&pattern)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, CountryRow>(
            "SELECT id, name, iso_code, iso3_code, phone_code, flag_emoji, currency_code
             FROM countries
             WHERE is_active = TRUE
             ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(&state.db)
        .await?
    };

    let json = json!({ "data": rows });

    // Cache all-countries longer (24h), search results shorter (1h)
    let ttl: i64 = if filter.q.is_some() { 3600 } else { 86400 };
    let _: Result<(), _> = redis::cmd("SETEX")
        .arg(&cache_key)
        .arg(ttl)
        .arg(serde_json::to_string(&json).unwrap_or_default())
        .query_async(&mut redis)
        .await;

    Ok(Json(json))
}
