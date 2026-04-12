use axum::{extract::State, Json};
use hmac::Mac;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Digest;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct VerifyCodeRequest {
    pub code: String,
}

pub async fn setup_2fa(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    // Generate TOTP secret
    let secret = generate_totp_secret();
    let issuer = "WoWonder";

    // Store pending secret (not yet enabled)
    sqlx::query("UPDATE users SET two_factor_secret = $1 WHERE id = $2")
        .bind(&secret)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let username = sqlx::query_scalar::<_, String>(
        "SELECT username FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    let otpauth_url = format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&digits=6&period=30",
        issuer, username, secret, issuer
    );

    Ok(Json(json!({
        "data": {
            "secret": secret,
            "otpauth_url": otpauth_url,
            "qr_data": otpauth_url
        }
    })))
}

pub async fn enable_2fa(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<VerifyCodeRequest>,
) -> Result<Json<Value>, ApiError> {
    let secret = sqlx::query_scalar::<_, Option<String>>(
        "SELECT two_factor_secret FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("2FA not set up yet".into()))?;

    if !verify_totp(&secret, &req.code) {
        return Err(ApiError::BadRequest("Invalid verification code".into()));
    }

    // Generate backup codes
    let backup_codes: Vec<String> = (0..10)
        .map(|_| format!("{:08}", rand::random::<u32>() % 100_000_000))
        .collect();

    let codes_json = serde_json::to_value(&backup_codes)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    sqlx::query("UPDATE users SET two_factor_enabled = TRUE WHERE id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    // Store hashed backup codes
    for code in &backup_codes {
        let hash = format!("{:x}", sha2::Sha256::digest(code.as_bytes()));
        sqlx::query(
            "INSERT INTO backup_codes (user_id, code_hash) VALUES ($1, $2)",
        )
        .bind(auth.user_id)
        .bind(&hash)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(json!({
        "data": {
            "enabled": true,
            "backup_codes": codes_json
        }
    })))
}

pub async fn verify_2fa(
    State(state): State<AppState>,
    Json(req): Json<Verify2faRequest>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64, Option<String>, bool)>(
        "SELECT id, two_factor_secret, two_factor_enabled FROM users WHERE id = $1",
    )
    .bind(req.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    let (user_id, secret, enabled) = row;
    if !enabled {
        return Err(ApiError::BadRequest("2FA is not enabled".into()));
    }

    let secret = secret.ok_or_else(|| ApiError::Internal("2FA secret missing".into()))?;

    // Try TOTP code first
    if verify_totp(&secret, &req.code) {
        return Ok(Json(json!({ "data": { "verified": true } })));
    }

    // Try backup code
    let code_hash = format!("{:x}", sha2::Sha256::digest(req.code.as_bytes()));
    let deleted = sqlx::query(
        "DELETE FROM backup_codes WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL",
    )
    .bind(user_id)
    .bind(&code_hash)
    .execute(&state.db)
    .await?
    .rows_affected();

    if deleted > 0 {
        return Ok(Json(json!({ "data": { "verified": true, "backup_code_used": true } })));
    }

    Err(ApiError::BadRequest("Invalid verification code".into()))
}

pub async fn disable_2fa(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<VerifyCodeRequest>,
) -> Result<Json<Value>, ApiError> {
    let secret = sqlx::query_scalar::<_, Option<String>>(
        "SELECT two_factor_secret FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("2FA not enabled".into()))?;

    if !verify_totp(&secret, &req.code) {
        return Err(ApiError::BadRequest("Invalid verification code".into()));
    }

    sqlx::query(
        "UPDATE users SET two_factor_enabled = FALSE, two_factor_secret = NULL WHERE id = $1",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    sqlx::query("DELETE FROM backup_codes WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "disabled": true } })))
}

pub async fn get_backup_codes(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM backup_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "remaining_codes": count } })))
}

pub async fn regenerate_backup_codes(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let enabled = sqlx::query_scalar::<_, bool>(
        "SELECT two_factor_enabled FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !enabled {
        return Err(ApiError::BadRequest("2FA is not enabled".into()));
    }

    // Delete old codes
    sqlx::query("DELETE FROM backup_codes WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let backup_codes: Vec<String> = (0..10)
        .map(|_| format!("{:08}", rand::random::<u32>() % 100_000_000))
        .collect();

    for code in &backup_codes {
        let hash = format!("{:x}", sha2::Sha256::digest(code.as_bytes()));
        sqlx::query("INSERT INTO backup_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(auth.user_id)
            .bind(&hash)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(json!({ "data": { "backup_codes": backup_codes } })))
}

// ── DTOs ──

#[derive(Debug, Deserialize)]
pub struct Verify2faRequest {
    pub user_id: i64,
    pub code: String,
}

// ── TOTP helpers ──

fn generate_totp_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 20];
    rand::rng().fill_bytes(&mut bytes);
    base32_encode(&bytes)
}

fn verify_totp(secret: &str, code: &str) -> bool {
    let Ok(secret_bytes) = base32_decode(secret) else {
        return false;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check current period and ±1 for clock skew
    for offset in [-1i64, 0, 1] {
        let period = ((now as i64 / 30) + offset) as u64;
        let expected = generate_totp_code(&secret_bytes, period);
        if expected == code {
            return true;
        }
    }
    false
}

fn generate_totp_code(secret: &[u8], counter: u64) -> String {
    use hmac::Hmac;
    use sha1::Sha1;

    let counter_bytes = counter.to_be_bytes();
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).unwrap();
    mac.update(&counter_bytes);
    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0xf) as usize;
    let code = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);

    format!("{:06}", code % 1_000_000)
}

fn base32_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut buffer = 0u64;
    let mut bits = 0;
    for &byte in data {
        buffer = (buffer << 8) | byte as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            result.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        buffer <<= 5 - bits;
        result.push(ALPHABET[(buffer & 0x1f) as usize] as char);
    }
    result
}

fn base32_decode(encoded: &str) -> Result<Vec<u8>, ()> {
    let mut buffer = 0u64;
    let mut bits = 0;
    let mut result = Vec::new();
    for c in encoded.chars() {
        let val = match c {
            'A'..='Z' => c as u64 - 'A' as u64,
            '2'..='7' => c as u64 - '2' as u64 + 26,
            '=' => continue,
            _ => return Err(()),
        };
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
        }
    }
    Ok(result)
}
