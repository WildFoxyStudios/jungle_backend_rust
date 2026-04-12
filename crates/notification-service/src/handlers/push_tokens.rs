use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct RegisterTokenRequest {
    pub token: String,
    pub platform: Option<String>,
    pub device_id: Option<String>,
}

pub async fn register_push_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegisterTokenRequest>,
) -> Result<Json<Value>, ApiError> {
    let platform = req.platform.as_deref().unwrap_or("fcm");

    sqlx::query(
        r#"INSERT INTO push_tokens (user_id, token, platform, device_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, token) DO UPDATE SET platform = $3, device_id = $4"#,
    )
    .bind(auth.user_id)
    .bind(&req.token)
    .bind(platform)
    .bind(&req.device_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "registered": true } })))
}

pub async fn unregister_push_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(token): Path<String>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM push_tokens WHERE user_id = $1 AND token = $2")
        .bind(auth.user_id)
        .bind(&token)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "unregistered": true } })))
}

pub async fn list_my_tokens(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, time::OffsetDateTime)>(
        "SELECT id, token, platform, device_id, created_at FROM push_tokens WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, token, platform, device_id, created_at)| {
            json!({
                "id": id, "token": token, "platform": platform,
                "device_id": device_id, "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}
