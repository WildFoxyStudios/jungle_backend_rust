use axum::{extract::State, Json};
use serde::Deserialize;
use shared::{
    auth::{encode_access_token, hash_token, AppState},
    errors::ApiError,
    models::{AuthUserResponse, User},
};
use uuid::Uuid;
use validator::Validate;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "Identifier is required"))]
    pub identifier: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
    pub platform: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let identifier = req.identifier.trim().to_lowercase();

    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE LOWER(username) = $1 OR LOWER(email) = $1 OR phone_number = $1",
    )
    .bind(&identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    if !user.is_active {
        return Err(ApiError::BadRequest("Account not verified".into()));
    }

    if user.deleted_at.is_some() {
        return Err(ApiError::BadRequest("Account has been deleted".into()));
    }

    // Social-login accounts have no local password set yet; they must log in
    // via the social provider until they call `POST /v1/auth/social/set-password`.
    let existing_hash = user.password_hash.as_deref().filter(|s| !s.is_empty()).ok_or_else(|| {
        ApiError::BadRequest(
            "This account uses social login only. Log in with the original provider or set a password first.".into(),
        )
    })?;

    let password_valid = verify_password_multi(&req.password, existing_hash)?;
    if !password_valid {
        return Err(ApiError::Unauthorized);
    }

    if existing_hash.starts_with("$2") {
        let salt = SaltString::generate(OsRng);
        if let Ok(new_hash) = Argon2::default().hash_password(req.password.as_bytes(), &salt) {
            let _ = sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
                .bind(new_hash.to_string())
                .bind(user.id)
                .execute(&state.db)
                .await;
        }
    }

    let access_token = encode_access_token(
        user.id,
        user.uuid,
        user.is_admin,
        &state.config.jwt_secret,
    )?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let platform = req.platform.as_deref().unwrap_or("web");
    let expires_at =
        time::OffsetDateTime::now_utc() + time::Duration::days(30);

    sqlx::query("INSERT INTO sessions (user_id, token_hash, platform, ip_address, expires_at) VALUES ($1, $2, $3, '0.0.0.0', $4)")
        .bind(user.id)
        .bind(&token_hash)
        .bind(platform)
        .bind(expires_at)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE users SET last_seen = NOW(), is_online = TRUE WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    let user_resp = AuthUserResponse::from(&user);

    Ok(Json(serde_json::json!({
        "data": {
            "user": user_resp,
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": 900
        }
    })))
}

fn verify_password_multi(password: &str, hash: &str) -> Result<bool, ApiError> {
    if hash.starts_with("$argon2") {
        let parsed = PasswordHash::new(hash)
            .map_err(|_| ApiError::Internal("Invalid password hash".into()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    } else if hash.starts_with("$2") {
        Ok(bcrypt::verify(password, hash).unwrap_or(false))
    } else if hash.len() == 40 {
        use sha1::Digest;
        let result = sha1::Sha1::digest(password.as_bytes());
        Ok(format!("{:x}", result) == hash)
    } else if hash.len() == 32 {
        let result = md5::compute(password.as_bytes());
        Ok(format!("{:x}", result) == hash)
    } else {
        Ok(false)
    }
}
