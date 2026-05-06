use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;
use argon2::{PasswordHasher, PasswordVerifier};
use validator::Validate;

#[derive(Debug, Serialize, FromRow)]
pub struct OAuthAppRow {
    pub id: i64,
    pub user_id: i64,
    pub app_name: String,
    pub client_id: Uuid,
    pub redirect_uri: String,
    pub description: String,
    pub permissions: serde_json::Value,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateOAuthAppRequest {
    #[validate(length(min = 3, max = 100))]
    pub app_name: String,
    #[validate(url)]
    pub redirect_uri: String,
    pub description: Option<String>,
    pub permissions: Option<serde_json::Value>,
}

/// GET /v1/oauth/apps — list my OAuth apps
pub async fn list_apps(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let apps = sqlx::query_as::<_, OAuthAppRow>(
        r#"SELECT id, user_id, app_name, client_id, redirect_uri,
                  COALESCE(description, '') as description, permissions, is_active, created_at
           FROM oauth_apps WHERE user_id = $1 ORDER BY id DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": apps })))
}

/// POST /v1/oauth/apps — create new OAuth app
pub async fn create_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateOAuthAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let client_secret = Uuid::new_v4().to_string();

    // Hash the client_secret with Argon2id before storing
    let salt = argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let secret_hash = argon2::Argon2::default()
        .hash_password(client_secret.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(format!("hash error: {}", e)))?
        .to_string();

    let permissions = req.permissions.unwrap_or(json!(["read"]));

    let app = sqlx::query_as::<_, OAuthAppRow>(
        r#"INSERT INTO oauth_apps (user_id, app_name, redirect_uri, description, permissions, client_secret)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, user_id, app_name, client_id, redirect_uri,
                     COALESCE(description, '') as description, permissions, is_active, created_at"#,
    )
    .bind(auth.user_id)
    .bind(&req.app_name)
    .bind(&req.redirect_uri)
    .bind(&req.description)
    .bind(&permissions)
    .bind(&secret_hash)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "app": app,
            "client_secret": client_secret
        }
    })))
}

/// GET /v1/oauth/apps/{id}
pub async fn get_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app = sqlx::query_as::<_, OAuthAppRow>(
        r#"SELECT id, user_id, app_name, client_id, redirect_uri,
                  COALESCE(description, '') as description, permissions, is_active, created_at
           FROM oauth_apps WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("App not found".into()))?;

    Ok(Json(json!({ "data": app })))
}

/// PUT /v1/oauth/apps/{id}
pub async fn update_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateOAuthAppRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        r#"UPDATE oauth_apps SET
            app_name = COALESCE($3, app_name),
            redirect_uri = COALESCE($4, redirect_uri),
            description = COALESCE($5, description)
        WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.app_name)
    .bind(&req.redirect_uri)
    .bind(&req.description)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("App not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateOAuthAppRequest {
    pub app_name: Option<String>,
    pub redirect_uri: Option<String>,
    pub description: Option<String>,
}

/// DELETE /v1/oauth/apps/{id}
pub async fn delete_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM oauth_apps WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("App not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

/// GET /v1/oauth/apps/{id}/permissions
pub async fn get_app_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let perms: serde_json::Value =
        sqlx::query_scalar("SELECT permissions FROM oauth_apps WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(ApiError::NotFound("App not found".into()))?;

    Ok(Json(json!({ "data": perms })))
}

/// POST /v1/oauth/authorize — authorize an OAuth app (simplified).
///
/// Accepts an opaque `state` param that the client generates. It is persisted
/// alongside the auth code and echoed on token exchange to prevent CSRF attacks
/// (OAuth 2 spec §10.12).
pub async fn authorize(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AuthorizeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify app exists
    let app = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, redirect_uri FROM oauth_apps WHERE client_id = $1 AND is_active = TRUE",
    )
    .bind(req.client_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("OAuth app not found".into()))?;

    // Validate state parameter shape if provided (prevent oversized/crafted payloads)
    if let Some(s) = req.state.as_deref()
        && (s.len() > 128
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
    {
        return Err(ApiError::BadRequest(
            "state must be ≤128 chars of [A-Za-z0-9-_.]".into(),
        ));
    }

    // Generate auth code
    let code = Uuid::new_v4().to_string();

    let scope = req.scope.as_deref().unwrap_or("read");

    sqlx::query(
        r#"INSERT INTO oauth_codes (app_id, user_id, code, redirect_uri, scope, state, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, NOW() + INTERVAL '10 minutes')"#,
    )
    .bind(app.0)
    .bind(auth.user_id)
    .bind(&code)
    .bind(&app.1)
    .bind(scope)
    .bind(&req.state)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "code": code,
            "redirect_uri": app.1,
            "scope": scope,
            "state": req.state,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub client_id: Uuid,
    pub scope: Option<String>,
    /// Opaque client-generated value bound to the auth code to prevent CSRF.
    pub state: Option<String>,
}

/// POST /v1/oauth/token — exchange auth code for access token.
///
/// The caller must present (a) the authorization code, (b) the app's
/// client_secret, and (c) the original `state` that was passed to `authorize`
/// (only if the authorize call included one). This prevents CSRF replay.
pub async fn exchange_token(
    State(state): State<AppState>,
    Json(req): Json<TokenExchangeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validate code (expires_at > NOW()). Retrieve the bound state alongside.
    let code_row: Option<(i64, i64, Option<String>)> = sqlx::query_as(
        r#"SELECT app_id, user_id, state FROM oauth_codes
           WHERE code = $1 AND expires_at > NOW()"#,
    )
    .bind(&req.code)
    .fetch_optional(&state.db)
    .await?;

    let (app_id, user_id, stored_state) =
        code_row.ok_or(ApiError::BadRequest("Invalid or expired code".into()))?;

    // CSRF check: if the code was issued with a state, the client must echo it.
    if let Some(expected) = stored_state.as_deref() {
        match req.state.as_deref() {
            Some(provided) if provided == expected => {}
            _ => {
                tracing::warn!(
                    app_id,
                    user_id,
                    "OAuth token exchange rejected — state mismatch (possible CSRF)"
                );
                return Err(ApiError::Forbidden("state mismatch".into()));
            }
        }
    }

    // Verify client_secret — fetch stored hash and verify with Argon2id
    let stored_hash: String = sqlx::query_scalar(
        "SELECT client_secret FROM oauth_apps WHERE id = $1",
    )
    .bind(app_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let parsed = argon2::PasswordHash::new(&stored_hash)
        .map_err(|_| ApiError::Unauthorized)?;
    argon2::Argon2::default()
        .verify_password(req.client_secret.as_bytes(), &parsed)
        .map_err(|_| ApiError::Unauthorized)?;

    // Generate access token
    let access_token = Uuid::new_v4().to_string();

    sqlx::query(
        r#"INSERT INTO oauth_tokens (app_id, user_id, access_token, expires_at)
           VALUES ($1, $2, $3, NOW() + INTERVAL '30 days')"#,
    )
    .bind(app_id)
    .bind(user_id)
    .bind(&access_token)
    .execute(&state.db)
    .await?;

    // Delete used code (single-use).
    sqlx::query("DELETE FROM oauth_codes WHERE code = $1")
        .bind(&req.code)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "data": {
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": 2592000,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct TokenExchangeRequest {
    pub code: String,
    pub client_secret: String,
    /// Must match the `state` sent to `authorize` (OAuth 2 §10.12 CSRF protection).
    pub state: Option<String>,
}

/// POST /v1/oauth/revoke — revoke an OAuth token
pub async fn revoke_token(
    State(state): State<AppState>,
    Json(req): Json<RevokeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM oauth_tokens WHERE access_token = $1")
        .bind(&req.token)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "revoked": true } })))
}

#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    pub token: String,
}
