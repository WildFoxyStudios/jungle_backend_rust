use axum::{Json, extract::State};
use serde::Deserialize;
use shared::{
    auth::{AppState, hash_token},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token_hash = hash_token(&req.refresh_token);

    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "message": "Logged out successfully" }
    })))
}
