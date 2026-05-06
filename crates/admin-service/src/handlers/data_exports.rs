use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use shared::auth::{AppState, AuthUser};
use shared::errors::ApiError;
use shared::permissions::Permission;
use sqlx::Row;

// User requests data export
pub async fn request_export(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Check if already has pending export
    let existing = sqlx::query(
        "SELECT id FROM data_export_requests WHERE user_id = $1 AND status IN ('pending', 'processing')"
    )
    .bind(auth.user_id).fetch_optional(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    if existing.is_some() {
        return Err(ApiError::BadRequest("You already have a pending export request".into()));
    }

    let row = sqlx::query(
        "INSERT INTO data_export_requests (user_id, status, requested_at) VALUES ($1, 'pending', NOW()) RETURNING id"
    )
    .bind(auth.user_id).fetch_one(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "status": "pending",
        "message": "Your data export has been requested. You will be notified when it's ready."
    })))
}

// Check export status
pub async fn get_export_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query(
        "SELECT id, status, file_url, requested_at, completed_at, expires_at
         FROM data_export_requests WHERE user_id = $1 ORDER BY requested_at DESC LIMIT 1"
    )
    .bind(auth.user_id).fetch_optional(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    if let Some(r) = row {
        Ok(Json(serde_json::json!({
            "id": r.get::<i64, _>("id"),
            "status": r.get::<String, _>("status"),
            "file_url": r.get::<Option<String>, _>("file_url"),
            "requested_at": r.get::<String, _>("requested_at"),
            "completed_at": r.get::<Option<String>, _>("completed_at"),
        })))
    } else {
        Ok(Json(serde_json::json!({ "status": "none" })))
    }
}

// Admin: list all export requests
pub async fn list_exports(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::ManageDataExports, &state).await?;

    let rows = sqlx::query(
        "SELECT e.*, u.username FROM data_export_requests e JOIN users u ON u.id = e.user_id ORDER BY e.requested_at DESC LIMIT 100"
    )
    .fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<i64, _>("id"),
        "username": r.get::<String, _>("username"),
        "status": r.get::<String, _>("status"),
        "requested_at": r.get::<String, _>("requested_at"),
    })).collect();
    Ok(Json(serde_json::json!({ "data": items })))
}

// ── Memorialization ─────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct MemorializeRequest {
    pub memorialized: bool,
}

pub async fn memorialize_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
    Json(body): Json<MemorializeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;

    if body.memorialized {
        sqlx::query("UPDATE users SET memorialized_at = NOW() WHERE id = $1")
            .bind(user_id).execute(&state.db).await
            .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    } else {
        sqlx::query("UPDATE users SET memorialized_at = NULL WHERE id = $1")
            .bind(user_id).execute(&state.db).await
            .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    }

    Ok(Json(serde_json::json!({ "user_id": user_id, "memorialized": body.memorialized })))
}

// ── Legacy Contact ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SetLegacyContactRequest { pub legacy_contact_id: i64 }

pub async fn set_legacy_contact(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<SetLegacyContactRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE users SET legacy_contact_id = $1 WHERE id = $2")
        .bind(body.legacy_contact_id).bind(auth.user_id)
        .execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(serde_json::json!({ "data": null })))
}
