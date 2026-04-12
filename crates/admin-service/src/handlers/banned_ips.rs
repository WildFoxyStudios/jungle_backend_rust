use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

pub async fn list_banned_ips(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, time::OffsetDateTime, Option<time::OffsetDateTime>)>(
        "SELECT id, ip_address, reason, created_at, expires_at FROM banned_ips ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, ip, reason, created_at, expires_at)| {
            json!({
                "id": id,
                "ip_address": ip,
                "reason": reason,
                "created_at": created_at.to_string(),
                "expires_at": expires_at.map(|t| t.to_string())
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct BanIpRequest {
    pub ip_address: String,
    pub reason: Option<String>,
    pub expires_at: Option<String>,
}

pub async fn ban_ip(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BanIpRequest>,
) -> Result<Json<Value>, ApiError> {
    let expires_at: Option<time::OffsetDateTime> = req
        .expires_at
        .as_deref()
        .and_then(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok());

    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO banned_ips (ip_address, reason, banned_by, expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (ip_address) DO UPDATE SET reason = $2, expires_at = $4
        RETURNING id"#,
    )
    .bind(&req.ip_address)
    .bind(&req.reason)
    .bind(auth.user_id)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id, "banned": true } })))
}

pub async fn unban_ip(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM banned_ips WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Banned IP not found".into()));
    }

    Ok(Json(json!({ "data": { "unbanned": true } })))
}
