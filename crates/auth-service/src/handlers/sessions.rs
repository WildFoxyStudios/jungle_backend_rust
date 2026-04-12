use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct SessionResponse {
    pub id: i64,
    pub platform: String,
    pub ip_address: Option<String>,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

pub async fn list_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sessions = sqlx::query_as::<_, SessionResponse>(
        "SELECT id, platform, ip_address, created_at, expires_at FROM sessions WHERE user_id = $1 AND expires_at > NOW() ORDER BY created_at DESC LIMIT 20",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "data": sessions })))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
        .bind(session_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Session not found".into()));
    }

    Ok(Json(serde_json::json!({
        "data": { "message": "Session revoked" }
    })))
}

pub async fn revoke_all_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "message": "All sessions revoked",
            "count": result.rows_affected()
        }
    })))
}
