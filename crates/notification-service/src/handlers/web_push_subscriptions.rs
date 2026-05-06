use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    #[serde(default)]
    pub user_agent: Option<String>,
}

/// Persist (or refresh) a Web Push subscription for the current user.
/// `endpoint` is treated as the unique key per user — repeated calls
/// from the same browser update keys/UA in place.
pub async fn subscribe(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.endpoint.is_empty() || req.p256dh.is_empty() || req.auth.is_empty() {
        return Err(ApiError::BadRequest("missing subscription fields".into()));
    }
    sqlx::query(
        r#"INSERT INTO push_subscriptions (user_id, endpoint, p256dh, auth, user_agent, last_used_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (user_id, endpoint) DO UPDATE
            SET p256dh = EXCLUDED.p256dh,
                auth = EXCLUDED.auth,
                user_agent = EXCLUDED.user_agent,
                last_used_at = NOW()"#,
    )
    .bind(auth.user_id)
    .bind(&req.endpoint)
    .bind(&req.p256dh)
    .bind(&req.auth)
    .bind(&req.user_agent)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "subscribed": true } })))
}

#[derive(Debug, Deserialize)]
pub struct UnsubscribeRequest {
    pub endpoint: String,
}

pub async fn unsubscribe(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UnsubscribeRequest>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id = $1 AND endpoint = $2")
        .bind(auth.user_id)
        .bind(&req.endpoint)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "unsubscribed": true } })))
}

pub async fn list_my(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, time::OffsetDateTime)>(
        r#"SELECT id, endpoint, user_agent, created_at FROM push_subscriptions
           WHERE user_id = $1 ORDER BY created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, endpoint, ua, created_at)| {
            json!({
                "id": id,
                "endpoint": endpoint,
                "user_agent": ua,
                "created_at": created_at.to_string(),
            })
        })
        .collect();
    Ok(Json(json!({ "data": data })))
}

pub async fn delete_one(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM push_subscriptions WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

/// Public endpoint — exposes the VAPID public key so the SW can call
/// `pushManager.subscribe({ applicationServerKey: ... })` without an
/// authenticated request.
pub async fn vapid_public_key(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let key: Option<String> =
        sqlx::query_scalar("SELECT value FROM site_config WHERE key = 'vapid_public_key'")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();
    let key = key
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("VAPID_PUBLIC_KEY").ok())
        .unwrap_or_default();
    Ok(Json(json!({ "data": { "public_key": key } })))
}
