use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::HeaderMap,
};
use serde::Deserialize;
use shared::{
    auth::{AppState, encode_access_token, hash_token},
    errors::ApiError,
    models::{AuthUserResponse, User},
};
use std::net::SocketAddr;
use uuid::Uuid;
use validator::Validate;

use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "Identifier is required"))]
    pub identifier: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
    pub platform: Option<String>,
}

/// Extracts the caller's IP address from the forwarded headers a reverse
/// proxy would typically inject, falling back to the raw TCP peer.
///
/// Order of preference matches Vercel / Cloudflare / nginx conventions.
fn client_ip(headers: &HeaderMap, peer: &SocketAddr) -> String {
    for header in [
        "cf-connecting-ip",
        "x-real-ip",
        "x-forwarded-for",
        "forwarded",
    ] {
        if let Some(v) = headers.get(header).and_then(|v| v.to_str().ok()) {
            // `X-Forwarded-For` may contain a comma-separated chain; we
            // want the left-most (original) client IP.
            if let Some(first) = v.split(',').next().map(|s| s.trim())
                && !first.is_empty()
            {
                return first.to_string();
            }
        }
    }
    peer.ip().to_string()
}

/// Whether the site operator has enabled the unusual-login challenge.
/// Disabled by default so existing deployments keep the legacy behaviour.
async fn unusual_login_enabled(db: &sqlx::PgPool) -> bool {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT value FROM site_config WHERE category = 'general' AND key = 'unusual_login_enabled' LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    matches!(value.as_deref(), Some("true") | Some("1"))
}

/// Returns `true` if *no* existing session for this user has been observed
/// from the same IP address. When `true`, we consider the login attempt
/// "unusual" and require an additional email challenge.
async fn is_new_device(db: &sqlx::PgPool, user_id: i64, ip: &str) -> bool {
    if ip == "0.0.0.0" || ip.is_empty() {
        return false;
    }
    let seen: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND ip_address = $2")
            .bind(user_id)
            .bind(ip)
            .fetch_one(db)
            .await
            .unwrap_or(0);
    seen == 0
}

/// Issues a one-shot challenge token for an unusual-login attempt. The
/// token lives in Redis for 10 minutes and is consumed by
/// `verify_unusual_login`.
async fn issue_unusual_login_challenge(
    state: &AppState,
    user: &User,
    ip: &str,
) -> Result<(String, String), ApiError> {
    let token = Uuid::new_v4().to_string();
    let code = format!("{:06}", rand::random::<u32>() % 1_000_000);

    let payload = serde_json::json!({
        "user_id": user.id,
        "ip": ip,
        "code": &code,
    })
    .to_string();

    let mut redis = state.redis.clone();
    let _: Result<(), _> = redis::cmd("SET")
        .arg(format!("unusual_login:{}", token))
        .arg(&payload)
        .arg("EX")
        .arg(600)
        .query_async(&mut redis)
        .await;

    // Send the email with the one-time code. Silent failure is acceptable
    // here because the code is still retrievable from Redis for ops.
    let site_name = std::env::var("SITE_NAME").unwrap_or_else(|_| "Jungle".into());
    let (subject, html) =
        shared::email_templates::unusual_login_email(&code, &site_name, ip, &user.first_name);
    if let Err(e) = shared::email::send_email(&user.email, &subject, &html).await {
        tracing::error!(email = %user.email, error = %e, "Failed to send unusual-login email");
    }

    Ok((token, code))
}

pub async fn login(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let ip = client_ip(&headers, &peer);
    let identifier = req.identifier.trim().to_lowercase();

    // Rate limit: 10 attempts per IP per minute
    shared::rate_limit::RateLimiter::check(
        &mut state.redis.clone(),
        &format!("rl:login:{}", ip),
        10,
        60,
    )
    .await?;

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

    // Upgrade legacy (bcrypt / SHA-1 / MD5) hashes to Argon2id transparently
    if !existing_hash.starts_with("$argon2") {
        let salt = SaltString::generate(OsRng);
        if let Ok(new_hash) = Argon2::default().hash_password(req.password.as_bytes(), &salt) {
            let _ = sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
                .bind(new_hash.to_string())
                .bind(user.id)
                .execute(&state.db)
                .await;
            tracing::info!(user_id = %user.id, "Upgraded legacy password hash to Argon2id");
        }
    }

    // Unusual-login gate: when the feature is on and the caller's IP has
    // never been associated with a session for this user, issue a
    // challenge instead of returning tokens. The frontend then drives
    // the user through `POST /v1/auth/verify-unusual-login`.
    if unusual_login_enabled(&state.db).await && is_new_device(&state.db, user.id, &ip).await {
        let (challenge_token, _code) = issue_unusual_login_challenge(&state, &user, &ip).await?;
        return Ok(Json(serde_json::json!({
            "data": {
                "requires_unusual_login_verification": true,
                "challenge_token": challenge_token,
                "email_masked": mask_email(&user.email),
            }
        })));
    }

    let access_token = encode_access_token(
        user.id,
        user.uuid,
        user.is_admin,
        user.is_moderator,
        &state.config.jwt_secret,
    )?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let platform = req.platform.as_deref().unwrap_or("web");
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(30);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, platform, ip_address, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(platform)
    .bind(&ip)
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

/// POST /v1/auth/verify-unusual-login — complete a login that was blocked
/// by the new-device heuristic. Consumes the challenge token, matches the
/// code, and returns a full `AuthResponse` on success.
#[derive(Debug, Deserialize)]
pub struct VerifyUnusualLoginRequest {
    pub challenge_token: String,
    pub code: String,
    pub platform: Option<String>,
}

pub async fn verify_unusual_login(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<VerifyUnusualLoginRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut redis = state.redis.clone();
    let key = format!("unusual_login:{}", req.challenge_token);

    let stored: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut redis)
        .await
        .ok();

    let payload = stored
        .ok_or_else(|| ApiError::BadRequest("Challenge expired. Please sign in again.".into()))?;

    let parsed: serde_json::Value = serde_json::from_str(&payload)
        .map_err(|_| ApiError::Internal("Corrupt challenge".into()))?;
    let stored_code = parsed["code"].as_str().unwrap_or("");
    let stored_user_id = parsed["user_id"].as_i64().unwrap_or(0);

    if stored_code != req.code.trim() {
        return Err(ApiError::Unauthorized);
    }

    // One-shot token — always clear it even on subsequent errors.
    let _: Result<(), _> = redis::cmd("DEL").arg(&key).query_async(&mut redis).await;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(stored_user_id)
        .fetch_one(&state.db)
        .await?;

    if !user.is_active {
        return Err(ApiError::BadRequest("Account not verified".into()));
    }

    if user.deleted_at.is_some() {
        return Err(ApiError::BadRequest("Account has been deleted".into()));
    }

    let access_token = encode_access_token(
        user.id,
        user.uuid,
        user.is_admin,
        user.is_moderator,
        &state.config.jwt_secret,
    )?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let platform = req.platform.as_deref().unwrap_or("web");
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(30);
    let ip = client_ip(&headers, &peer);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, platform, ip_address, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(platform)
    .bind(&ip)
    .bind(expires_at)
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

/// Obscures most of the email address so we can echo a hint in the
/// unusual-login response without leaking the full recipient.
fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let head: String = local.chars().take(2).collect();
            let masked_len = local.chars().count().saturating_sub(2);
            format!("{}{}@{}", head, "*".repeat(masked_len), domain)
        }
        _ => "***".into(),
    }
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
        let matches = format!("{:x}", result) == hash;
        // If verified, this legacy hash should be upgraded to Argon2.
        // Flag is consumed by the caller; the hash itself cannot be written
        // here because we don't have access to the user row.
        if matches {
            tracing::warn!("SHA-1 password hash verified — upgrade recommended");
        }
        Ok(matches)
    } else if hash.len() == 32 {
        let result = md5::compute(password.as_bytes());
        let matches = format!("{:x}", result) == hash;
        if matches {
            tracing::warn!("MD5 password hash verified — upgrade recommended");
        }
        Ok(matches)
    } else {
        Ok(false)
    }
}
