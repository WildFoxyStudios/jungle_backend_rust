use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub email: String,
}

pub async fn subscribe(
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        "INSERT INTO newsletter_subscribers (email) VALUES ($1) ON CONFLICT (email) DO NOTHING",
    )
    .bind(&req.email)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "subscribed": true } })))
}

pub async fn unsubscribe(
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE newsletter_subscribers SET is_active = FALSE WHERE email = $1")
        .bind(&req.email)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "unsubscribed": true } })))
}
