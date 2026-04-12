use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

// ── Privacy Settings ──

pub async fn get_privacy_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let settings: Value = sqlx::query_scalar(
        "SELECT COALESCE(privacy_settings, '{}') FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": settings })))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePrivacyRequest {
    pub follow_privacy: Option<String>,
    pub message_privacy: Option<String>,
    pub post_privacy: Option<String>,
    pub birth_privacy: Option<String>,
    pub online_privacy: Option<String>,
    pub profile_visibility: Option<String>,
}

pub async fn update_privacy_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdatePrivacyRequest>,
) -> Result<Json<Value>, ApiError> {
    // Build JSONB patch
    let mut patch = json!({});
    if let Some(v) = &req.follow_privacy { patch["follow_privacy"] = json!(v); }
    if let Some(v) = &req.message_privacy { patch["message_privacy"] = json!(v); }
    if let Some(v) = &req.post_privacy { patch["post_privacy"] = json!(v); }
    if let Some(v) = &req.birth_privacy { patch["birth_privacy"] = json!(v); }
    if let Some(v) = &req.online_privacy { patch["online_privacy"] = json!(v); }
    if let Some(v) = &req.profile_visibility { patch["profile_visibility"] = json!(v); }

    sqlx::query(
        "UPDATE users SET privacy_settings = COALESCE(privacy_settings, '{}')::jsonb || $1::jsonb WHERE id = $2",
    )
    .bind(&patch)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Notification Settings ──

pub async fn get_notification_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let settings: Value = sqlx::query_scalar(
        "SELECT COALESCE(notification_settings, '{}') FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": settings })))
}

pub async fn update_notification_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(settings): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        "UPDATE users SET notification_settings = $1 WHERE id = $2",
    )
    .bind(&settings)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Invite Codes ──

pub async fn get_my_invite_code(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let code = sqlx::query_as::<_, (i64, String, i32, i32, bool)>(
        "SELECT id, code, max_uses, uses, is_active FROM invite_codes WHERE user_id = $1 AND is_active = TRUE LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some((id, code, max_uses, uses, active)) = code {
        Ok(Json(json!({
            "data": { "id": id, "code": code, "max_uses": max_uses, "uses": uses, "is_active": active }
        })))
    } else {
        // Generate a new invite code
        let new_code = format!("{}", uuid::Uuid::new_v4().simple());
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO invite_codes (user_id, code) VALUES ($1, $2) RETURNING id",
        )
        .bind(auth.user_id)
        .bind(&new_code)
        .fetch_one(&state.db)
        .await?;

        Ok(Json(json!({
            "data": { "id": id, "code": new_code, "max_uses": 10, "uses": 0, "is_active": true }
        })))
    }
}
