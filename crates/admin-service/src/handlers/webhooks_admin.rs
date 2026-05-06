use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use shared::auth::{AppState, AuthUser};
use shared::errors::ApiError;
use shared::permissions::Permission;
use sqlx::Row;

#[derive(Deserialize)]
pub struct CreateWebhookRequest {
    pub app_id: i64,
    pub url: String,
    pub events: Option<Vec<String>>,
}

pub async fn list_webhooks(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::ManageWebhooks, &state).await?;

    let rows = sqlx::query(
        "SELECT id, app_id, url, events, is_enabled, created_at FROM webhooks ORDER BY created_at DESC"
    )
    .fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<i64, _>("id"),
        "app_id": r.get::<i64, _>("app_id"),
        "url": r.get::<String, _>("url"),
        "events": r.get::<serde_json::Value, _>("events"),
        "is_enabled": r.get::<bool, _>("is_enabled"),
        "created_at": r.get::<String, _>("created_at"),
    })).collect();
    Ok(Json(serde_json::json!({ "data": items })))
}

pub async fn create_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateWebhookRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::ManageWebhooks, &state).await?;

    let events = serde_json::to_value(body.events.unwrap_or_default()).unwrap_or_default();
    let row = sqlx::query(
        "INSERT INTO webhooks (app_id, url, events) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(body.app_id).bind(&body.url).bind(&events)
    .fetch_one(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    Ok(Json(serde_json::json!({ "id": row.get::<i64, _>("id"), "url": body.url })))
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(webhook_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::ManageWebhooks, &state).await?;
    sqlx::query("DELETE FROM webhooks WHERE id = $1").bind(webhook_id)
        .execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(serde_json::json!({ "data": null })))
}
