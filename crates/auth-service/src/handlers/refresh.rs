use axum::{Json, extract::State};
use serde::Deserialize;
use shared::{
    auth::{AppState, encode_access_token, hash_token},
    errors::ApiError,
    models::User,
};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
struct SessionRow {
    id: i64,
    user_id: i64,
    platform: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token_hash = hash_token(&req.refresh_token);

    let session = sqlx::query_as::<_, SessionRow>(
        "SELECT id, user_id, platform FROM sessions WHERE token_hash = $1 AND expires_at > NOW()",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL AND is_active = TRUE",
    )
    .bind(session.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let access_token = encode_access_token(
        user.id,
        user.uuid,
        user.is_admin,
        user.is_moderator,
        &state.config.jwt_secret,
    )?;

    let new_refresh = Uuid::new_v4().to_string();
    let new_hash = hash_token(&new_refresh);
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(30);

    let mut tx = state.db.begin().await?;

    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session.id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, platform, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(user.id)
    .bind(&new_hash)
    .bind(&session.platform)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "data": {
            "access_token": access_token,
            "refresh_token": new_refresh,
            "expires_in": 900
        }
    })))
}
