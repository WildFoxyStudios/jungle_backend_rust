use axum::{extract::State, Json};
use serde::Deserialize;
use shared::{
    auth::{encode_access_token, hash_token, AppState},
    errors::ApiError,
    events::DomainEvent,
    models::AuthUserResponse,
};
use uuid::Uuid;
use validator::Validate;

use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(custom(function = "shared::validation::validate_username"))]
    pub username: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(custom(function = "shared::validation::validate_password_strength"))]
    pub password: String,
    #[validate(length(min = 1, max = 50, message = "First name is required"))]
    pub first_name: String,
    #[validate(length(max = 50))]
    pub last_name: Option<String>,
    pub gender: Option<String>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let username_lower = req.username.to_lowercase();
    let email_lower = req.email.to_lowercase();

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(username) = $1 OR LOWER(email) = $2)",
    )
    .bind(&username_lower)
    .bind(&email_lower)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if exists {
        return Err(ApiError::Conflict(
            "Username or email already taken".into(),
        ));
    }

    let salt = SaltString::generate(OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|_| ApiError::Internal("Failed to hash password".into()))?
        .to_string();

    let gender = req.gender.unwrap_or_else(|| "none".into());
    let last_name = req.last_name.unwrap_or_default();

    let user = sqlx::query_as::<_, shared::models::User>(
        "INSERT INTO users (username, email, password_hash, first_name, last_name, gender, is_active) VALUES ($1, $2, $3, $4, $5, $6, TRUE) RETURNING *",
    )
    .bind(&req.username)
    .bind(&email_lower)
    .bind(&password_hash)
    .bind(&req.first_name)
    .bind(&last_name)
    .bind(&gender)
    .fetch_one(&state.db)
    .await?;

    let access_token = encode_access_token(
        user.id,
        user.uuid,
        user.is_admin,
        &state.config.jwt_secret,
    )?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let expires_at =
        time::OffsetDateTime::now_utc() + time::Duration::days(30);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, platform, ip_address, expires_at) VALUES ($1, $2, 'web', '0.0.0.0', $3)",
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    let user_resp = AuthUserResponse::from(&user);

    let _ = state.event_bus.publish(&DomainEvent::UserCreated {
        user_id: user.id,
        username: user.username.clone(),
    }).await;

    Ok(Json(serde_json::json!({
        "data": {
            "user": user_resp,
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": 900
        }
    })))
}
