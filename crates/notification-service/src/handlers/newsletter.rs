use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub email: String,
}

/// Basic email validation to reject malformed addresses before DB insert.
fn is_valid_email(email: &str) -> bool {
    email.len() >= 5
        && email.len() <= 254
        && email.contains('@')
        && email.contains('.')
        && !email.contains(char::is_whitespace)
        && !email.contains('<')
        && !email.contains('>')
        && !email.contains("..")
        && email.chars().all(|c| c.is_ascii_graphic() || c == '@')
}

pub async fn subscribe(
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<Value>, ApiError> {
    let email = req.email.trim().to_lowercase();
    if !is_valid_email(&email) {
        return Err(ApiError::BadRequest("Invalid email address".into()));
    }

    sqlx::query(
        "INSERT INTO newsletter_subscribers (email) VALUES ($1) ON CONFLICT (email) DO NOTHING",
    )
    .bind(&email)
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
