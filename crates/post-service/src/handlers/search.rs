use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub r#type: Option<String>,
    pub limit: Option<i64>,
}

/// GET /v1/search — global search across all entity types
pub async fn global_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 50);
    shared::search::search_all(
        &state.db,
        &params.q,
        params.r#type.as_deref(),
        None,
        limit,
    )
    .await
}

// ── Recent Searches ──

#[derive(Debug, Serialize, FromRow)]
pub struct RecentSearchRow {
    pub id: i64,
    pub search_type: String,
    pub target_id: i64,
    pub searched_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct SaveRecentSearchRequest {
    pub search_type: String,
    pub target_id: i64,
}

/// POST /v1/search/recent — save a recent search entry
pub async fn save_recent_search(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SaveRecentSearchRequest>,
) -> Result<Json<Value>, ApiError> {
    let valid_types = ["user", "page", "group", "hashtag", "post"];
    if !valid_types.contains(&req.search_type.as_str()) {
        return Err(ApiError::BadRequest("Invalid search_type".into()));
    }

    // Upsert — if same type+target already exists, just update timestamp
    sqlx::query(
        r#"
        INSERT INTO recent_searches (user_id, search_type, target_id, searched_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.search_type)
    .bind(req.target_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "saved": true } })))
}

/// GET /v1/search/recent — list recent searches
pub async fn list_recent_searches(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let recent = sqlx::query_as::<_, RecentSearchRow>(
        r#"
        SELECT id, search_type, target_id, searched_at
        FROM recent_searches
        WHERE user_id = $1
        ORDER BY searched_at DESC
        LIMIT 20
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": recent })))
}

/// DELETE /v1/search/recent — clear all recent searches
pub async fn clear_recent_searches(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM recent_searches WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "cleared": true } })))
}
