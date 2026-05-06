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
use time::OffsetDateTime;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct LookupAdminRow {
    pub id: i64,
    pub lookup_type: String,
    pub value: String,
    pub label_key: String,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct LookupFilter {
    pub r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLookupRequest {
    pub lookup_type: String,
    pub value: String,
    pub label_key: String,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLookupRequest {
    pub lookup_type: Option<String>,
    pub value: Option<String>,
    pub label_key: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

pub async fn list_lookups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(filter): Query<LookupFilter>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = if let Some(ref tp) = filter.r#type {
        sqlx::query_as::<_, LookupAdminRow>(
            "SELECT id, lookup_type, value, label_key, icon, sort_order, is_active, created_at
             FROM lookups
             WHERE lookup_type = $1
             ORDER BY sort_order ASC, id ASC",
        )
        .bind(tp)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, LookupAdminRow>(
            "SELECT id, lookup_type, value, label_key, icon, sort_order, is_active, created_at
             FROM lookups
             ORDER BY lookup_type ASC, sort_order ASC, id ASC",
        )
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(json!({ "data": rows })))
}

pub async fn create_lookup(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateLookupRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, LookupAdminRow>(
        "INSERT INTO lookups (lookup_type, value, label_key, icon, sort_order)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, lookup_type, value, label_key, icon, sort_order, is_active, created_at",
    )
    .bind(req.lookup_type.trim())
    .bind(req.value.trim())
    .bind(req.label_key.trim())
    .bind(req.icon.as_deref().map(|s| s.trim()))
    .bind(req.sort_order.unwrap_or(0))
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e.as_database_error().and_then(|d| d.code()) {
        Some(code) if code == "23505" => {
            ApiError::Conflict("A lookup with this type and value already exists".into())
        }
        _ => ApiError::from(e),
    })?;

    invalidate_lookups_cache(&state, &req.lookup_type).await;

    Ok(Json(json!({ "data": row })))
}

pub async fn update_lookup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateLookupRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, LookupAdminRow>(
        "UPDATE lookups SET
            lookup_type = COALESCE($2, lookup_type),
            value = COALESCE($3, value),
            label_key = COALESCE($4, label_key),
            icon = COALESCE($5, icon),
            sort_order = COALESCE($6, sort_order),
            is_active = COALESCE($7, is_active)
         WHERE id = $1
         RETURNING id, lookup_type, value, label_key, icon, sort_order, is_active, created_at",
    )
    .bind(id)
    .bind(req.lookup_type.as_deref().map(|s| s.trim()))
    .bind(req.value.as_deref().map(|s| s.trim()))
    .bind(req.label_key.as_deref().map(|s| s.trim()))
    .bind(req.icon.as_deref().map(|s| s.trim()))
    .bind(req.sort_order)
    .bind(req.is_active)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Lookup not found".into()))?;

    invalidate_lookups_cache(&state, &row.lookup_type).await;

    Ok(Json(json!({ "data": row })))
}

pub async fn delete_lookup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, LookupAdminRow>(
        "SELECT id, lookup_type, value, label_key, icon, sort_order, is_active, created_at FROM lookups WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Lookup not found".into()))?;

    sqlx::query("DELETE FROM lookups WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    invalidate_lookups_cache(&state, &row.lookup_type).await;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

async fn invalidate_lookups_cache(state: &AppState, lookup_type: &str) {
    let cache_key = format!("lookups:{}", lookup_type);
    let mut redis = state.redis.clone();
    let _: Result<(), _> = redis::cmd("DEL")
        .arg(&cache_key)
        .query_async(&mut redis)
        .await;
}
