use axum::{Json, extract::State};
use serde::Deserialize;
use shared::{
    auth::{AppState, encode_access_token, hash_token},
    errors::ApiError,
    events::DomainEvent,
    models::AuthUserResponse,
};
use uuid::Uuid;
use validator::Validate;

use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};

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
    /// E.164 phone number without the leading `+` is also accepted. When
    /// provided and the site enforces phone verification, a verification
    /// code is sent before the account is considered fully active.
    #[validate(length(min = 5, max = 20))]
    pub phone_number: Option<String>,
    /// Pre-registration invitation code (matches `invitation_links.code`).
    /// Required when `registration_mode=invite_only`; rejected if the code
    /// is inactive, expired, or has exhausted its `max_uses` quota.
    #[validate(length(min = 3, max = 300))]
    pub invite_code: Option<String>,
}

/// Reads the current registration gating rule from `site_config`. When the
/// key is missing we default to `open` to keep self-hosted deployments
/// functional out of the box.
///
/// Allowed values: `open`, `invite_only`, `approval_required`, `closed`
/// (see `crates/admin-service/src/handlers/config_catalog.rs`).
async fn registration_mode(db: &sqlx::PgPool) -> String {
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM site_config \
         WHERE category = 'general' AND key = 'registration_mode' \
         LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "open".into())
}

/// Reads verification requirements from `site_config`.
/// Defaults match `/v1/auth/register-config`:
/// - email verification: enabled
/// - phone verification: disabled
async fn registration_verification_requirements(db: &sqlx::PgPool) -> (bool, bool) {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM site_config \
         WHERE category = 'general' \
           AND key IN ('require_email_verification','require_phone_verification')",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut require_email_verification = true;
    let mut require_phone_verification = false;

    for (key, value) in rows {
        match key.as_str() {
            "require_email_verification" => {
                require_email_verification = value == "true" || value == "1";
            }
            "require_phone_verification" => {
                require_phone_verification = value == "true" || value == "1";
            }
            _ => {}
        }
    }

    (require_email_verification, require_phone_verification)
}

/// Consumes an invitation code atomically: increments `uses`, auto-disables
/// when `uses >= max_uses`, and returns the row id of the inviter so we can
/// optionally credit them with a referral bonus later.
async fn consume_invite_code(db: &sqlx::PgPool, code: &str) -> Result<i64, ApiError> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Err(ApiError::BadRequest("Invitation code is required".into()));
    }

    // `RETURNING` + `FOR UPDATE` semantics: the UPDATE is atomic under
    // serializable-snapshot isolation, which is enough for this flow.
    let row: Option<(i64, i64)> = sqlx::query_as(
        r#"
        UPDATE invitation_links
           SET uses = uses + 1,
               is_active = CASE
                   WHEN uses + 1 >= max_uses THEN FALSE
                   ELSE is_active
               END
         WHERE code = $1
           AND is_active = TRUE
           AND (expires_at IS NULL OR expires_at > NOW())
           AND uses < max_uses
         RETURNING id, user_id
        "#,
    )
    .bind(trimmed)
    .fetch_optional(db)
    .await?;

    match row {
        Some((_id, user_id)) => Ok(user_id),
        None => Err(ApiError::BadRequest(
            "Invitation code is invalid, expired, or has been fully used".into(),
        )),
    }
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let (require_email_verification, require_phone_verification) =
        registration_verification_requirements(&state.db).await;

    if require_phone_verification && req.phone_number.is_none() {
        return Err(ApiError::BadRequest(
            "Phone number is required when phone verification is enabled".into(),
        ));
    }

    // Enforce invite-only mode before doing any heavier work.
    let mode = registration_mode(&state.db).await;
    let invite_required = mode == "invite_only";

    let inviter_id: Option<i64> = match (&req.invite_code, invite_required) {
        (Some(code), _) => Some(consume_invite_code(&state.db, code).await?),
        (None, true) => {
            return Err(ApiError::BadRequest(
                "Registration is invite-only — please provide an invitation code".into(),
            ));
        }
        (None, false) => None,
    };

    let username_lower = req.username.to_lowercase();
    let email_lower = req.email.to_lowercase();

    // Atomic uniqueness check across every identifier we store on the row.
    let exists: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM users
             WHERE LOWER(username) = $1
                OR LOWER(email) = $2
                OR ($3::text IS NOT NULL AND phone_number = $3)
        )"#,
    )
    .bind(&username_lower)
    .bind(&email_lower)
    .bind(req.phone_number.as_deref())
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if exists {
        return Err(ApiError::Conflict(
            "Username, email or phone number already taken".into(),
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
        r#"INSERT INTO users
              (username, email, password_hash, first_name, last_name, gender, phone_number, is_active, email_verified, phone_verified)
           VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE, $8, $9)
           RETURNING *"#,
    )
    .bind(&req.username)
    .bind(&email_lower)
    .bind(&password_hash)
    .bind(&req.first_name)
    .bind(&last_name)
    .bind(&gender)
    .bind(req.phone_number.as_deref())
    .bind(!require_email_verification)
    .bind(!require_phone_verification)
    .fetch_one(&state.db)
    .await?;

    // Credit the invitation with the new user id — useful for affiliate
    // programmes downstream. Best-effort: a failure here doesn't abort
    // the registration.
    if let Some(code) = req.invite_code.as_deref() {
        let _ = sqlx::query(
            "UPDATE invitation_links SET used_by = $1 WHERE code = $2 AND used_by IS NULL",
        )
        .bind(user.id)
        .bind(code.trim())
        .execute(&state.db)
        .await;
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
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(30);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, platform, ip_address, expires_at) VALUES ($1, $2, 'web', '0.0.0.0', $3)",
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    let user_resp = AuthUserResponse::from(&user);

    let _ = state
        .event_bus
        .publish(&DomainEvent::UserCreated {
            user_id: user.id,
            username: user.username.clone(),
        })
        .await;

    // Surface whether the client should redirect to the phone/email
    // verification screen. The frontend consults these flags to pick the
    // next route after a successful register.
    let needs_phone_verify = require_phone_verification && req.phone_number.is_some();
    let needs_email_verify = require_email_verification && !user.email_verified;

    Ok(Json(serde_json::json!({
        "data": {
            "user": user_resp,
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": 900,
            "needs_phone_verification": needs_phone_verify,
            "needs_email_verification": needs_email_verify,
            "inviter_id": inviter_id,
        }
    })))
}
