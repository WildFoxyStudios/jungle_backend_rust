use argon2::{PasswordHasher, PasswordVerifier};
use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Digest;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<Json<Value>, ApiError> {
    // Always respond success to prevent email enumeration
    let user = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM users WHERE email = $1 AND deleted_at IS NULL",
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await?;

    if let Some(user_id) = user {
        let token = Uuid::new_v4().to_string();
        let token_hash = format!("{:x}", sha2::Sha256::digest(token.as_bytes()));

        sqlx::query(
            "UPDATE users SET email_code = $1 WHERE id = $2",
        )
        .bind(&token_hash)
        .bind(user_id)
        .execute(&state.db)
        .await?;

        // Store token in Redis with 1h expiry
        let mut redis = state.redis.clone();
        let _: Result<(), _> = redis::cmd("SET")
            .arg(format!("pwd_reset:{}", token_hash))
            .arg(user_id)
            .arg("EX")
            .arg(3600)
            .query_async(&mut redis)
            .await;

        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:3000".into());
        let reset_link = format!("{}/reset-password?token={}", frontend_url, token);
        let site_name = std::env::var("SITE_NAME").unwrap_or_else(|_| "Jungle".into());
        let (subject, html_body) =
            shared::email_templates::password_reset_email(&reset_link, &site_name);

        if let Err(e) = shared::email::send_email(&req.email, &subject, &html_body).await {
            tracing::error!(email = %req.email, error = %e, "Failed to send password reset email");
        }
    }

    Ok(Json(json!({ "data": { "message": "If the email exists, a reset link has been sent" } })))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.password.len() < 6 {
        return Err(ApiError::BadRequest("Password must be at least 6 characters".into()));
    }

    let token_hash = format!("{:x}", sha2::Sha256::digest(req.token.as_bytes()));

    // Check Redis for token
    let mut redis = state.redis.clone();
    let user_id: Option<i64> = redis::cmd("GET")
        .arg(format!("pwd_reset:{}", token_hash))
        .query_async(&mut redis)
        .await
        .ok();

    let user_id = user_id.ok_or_else(|| ApiError::BadRequest("Invalid or expired token".into()))?;

    // Hash new password
    let salt = argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let hash = argon2::Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .to_string();

    sqlx::query("UPDATE users SET password = $1, email_code = NULL WHERE id = $2")
        .bind(&hash)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    // Delete token
    let _: Result<(), _> = redis::cmd("DEL")
        .arg(format!("pwd_reset:{}", token_hash))
        .query_async(&mut redis)
        .await;

    // Revoke all sessions
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "message": "Password reset successfully" } })))
}

/// PUT /v1/auth/password — Change password while authenticated (PHP: update_user_password.php)
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.new_password.len() < 8 {
        return Err(ApiError::BadRequest("New password must be at least 8 characters".into()));
    }

    let hash: String = sqlx::query_scalar(
        "SELECT password_hash FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    // Verify current password
    let parsed = argon2::PasswordHash::new(&hash)
        .map_err(|_| ApiError::BadRequest("Invalid current password".into()))?;
    argon2::Argon2::default()
        .verify_password(req.current_password.as_bytes(), &parsed)
        .map_err(|_| ApiError::BadRequest("Current password is incorrect".into()))?;

    // Hash new password
    let salt = argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let new_hash = argon2::Argon2::default()
        .hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .to_string();

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&new_hash)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    // Revoke all other sessions (keep current session valid)
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "changed": true, "message": "Password changed successfully" } })))
}
