use axum::{
    extract::{Path, State},
    Json,
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
    .bind(&client_secret)
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
    let perms: serde_json::Value = sqlx::query_scalar(
        "SELECT permissions FROM oauth_apps WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("App not found".into()))?;

    Ok(Json(json!({ "data": perms })))
}

/// POST /v1/oauth/authorize — authorize an OAuth app (simplified)
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

    // Generate auth code
    let code = Uuid::new_v4().to_string();

    let scope = req.scope.as_deref().unwrap_or("read");

    sqlx::query(
        r#"INSERT INTO oauth_codes (app_id, user_id, code, redirect_uri, scope, expires_at)
           VALUES ($1, $2, $3, $4, $5, NOW() + INTERVAL '10 minutes')"#,
    )
    .bind(app.0)
    .bind(auth.user_id)
    .bind(&code)
    .bind(&app.1)
    .bind(scope)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "code": code,
            "redirect_uri": app.1,
            "scope": scope,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub client_id: Uuid,
    pub scope: Option<String>,
}

/// POST /v1/oauth/token — exchange auth code for access token
pub async fn exchange_token(
    State(state): State<AppState>,
    Json(req): Json<TokenExchangeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validate code
    let code_row = sqlx::query_as::<_, (i64, i64)>(
        r#"SELECT app_id, user_id FROM oauth_codes
           WHERE code = $1 AND expires_at > NOW()"#,
    )
    .bind(&req.code)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::BadRequest("Invalid or expired code".into()))?;

    // Verify client_secret
    let valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM oauth_apps WHERE id = $1 AND client_secret = $2)",
    )
    .bind(code_row.0)
    .bind(&req.client_secret)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !valid {
        return Err(ApiError::Unauthorized);
    }

    // Generate access token
    let access_token = Uuid::new_v4().to_string();

    sqlx::query(
        r#"INSERT INTO oauth_tokens (app_id, user_id, access_token, expires_at)
           VALUES ($1, $2, $3, NOW() + INTERVAL '30 days')"#,
    )
    .bind(code_row.0)
    .bind(code_row.1)
    .bind(&access_token)
    .execute(&state.db)
    .await?;

    // Delete used code
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
