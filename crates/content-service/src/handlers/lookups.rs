use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use shared::{
    auth::AppState,
    errors::ApiError,
};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
pub struct LookupRow {
    pub id: i64,
    pub lookup_type: String,
    pub value: String,
    pub label_key: String,
    pub icon: Option<String>,
    pub sort_order: i32,
}

pub async fn list_lookups(
    State(state): State<AppState>,
    Path(lookup_type): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let cache_key = format!("lookups:{}", lookup_type);
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

    let rows = sqlx::query_as::<_, LookupRow>(
        "SELECT id, lookup_type, value, label_key, icon, sort_order
         FROM lookups
         WHERE lookup_type = $1 AND is_active = TRUE
         ORDER BY sort_order ASC, id ASC",
    )
    .bind(&lookup_type)
    .fetch_all(&state.db)
    .await?;

    let json = json!({ "data": rows });

    let _: Result<(), _> = redis::cmd("SETEX")
        .arg(&cache_key)
        .arg(3600i64)
        .arg(serde_json::to_string(&json).unwrap_or_default())
        .query_async(&mut redis)
        .await;

    Ok(Json(json))
}
