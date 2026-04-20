use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct CallRow {
    pub id: i64,
    pub caller_id: i64,
    pub callee_id: i64,
    pub call_type: String,
    pub provider: Option<String>,
    pub room_name: String,
    pub status: String,
    pub started_at: Option<OffsetDateTime>,
    pub ended_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateCallRequest {
    pub callee_id: i64,
    pub call_type: String,
}

/// POST /v1/calls — initiate a call (creates a record)
pub async fn create_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateCallRequest>,
) -> Result<Json<Value>, ApiError> {
    if !["video", "audio"].contains(&req.call_type.as_str()) {
        return Err(ApiError::BadRequest("call_type must be 'video' or 'audio'".into()));
    }

    let now = OffsetDateTime::now_utc();
    let room = format!("call_{}_{}",
        auth.user_id,
        now.unix_timestamp_nanos() / 1_000_000,
    );

    let call = sqlx::query_as::<_, CallRow>(
        r#"
        INSERT INTO calls (caller_id, callee_id, call_type, room_name)
        VALUES ($1, $2, $3, $4)
        RETURNING id, caller_id, callee_id, call_type, provider, room_name,
                  status, started_at, ended_at, created_at
        "#,
    )
    .bind(auth.user_id)
    .bind(req.callee_id)
    .bind(&req.call_type)
    .bind(&room)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": call })))
}

/// GET /v1/calls — list my calls (caller or callee)
pub async fn list_calls(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let calls = sqlx::query_as::<_, CallRow>(
        r#"
        SELECT id, caller_id, callee_id, call_type, provider, room_name,
               status, started_at, ended_at, created_at
        FROM calls
        WHERE (caller_id = $1 OR callee_id = $1)
          AND ($2::bigint IS NULL OR id < $2)
        ORDER BY id DESC
        LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = calls.len() as i64 > limit;
    let calls: Vec<_> = calls.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": calls, "meta": { "has_more": has_more } })))
}

/// GET /v1/calls/{id} — get a single call
pub async fn get_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let call = sqlx::query_as::<_, CallRow>(
        r#"
        SELECT id, caller_id, callee_id, call_type, provider, room_name,
               status, started_at, ended_at, created_at
        FROM calls WHERE id = $1 AND (caller_id = $2 OR callee_id = $2)
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Call not found".into()))?;

    Ok(Json(json!({ "data": call })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCallStatusRequest {
    pub status: String,
}

/// PUT /v1/calls/{id}/status — update call status (answered, ended, declined, missed)
pub async fn update_call_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCallStatusRequest>,
) -> Result<Json<Value>, ApiError> {
    let valid = ["answered", "ended", "declined", "missed", "busy"];
    if !valid.contains(&req.status.as_str()) {
        return Err(ApiError::BadRequest("Invalid call status".into()));
    }

    // Verify the user is part of this call
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM calls WHERE id = $1 AND (caller_id = $2 OR callee_id = $2))",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !exists {
        return Err(ApiError::NotFound("Call not found".into()));
    }

    if req.status == "answered" {
        sqlx::query("UPDATE calls SET status = $1, started_at = NOW() WHERE id = $2")
            .bind(&req.status)
            .bind(id)
            .execute(&state.db)
            .await?;
    } else {
        sqlx::query("UPDATE calls SET status = $1, ended_at = NOW() WHERE id = $2")
            .bind(&req.status)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(json!({ "data": { "id": id, "status": req.status } })))
}

/// POST /v1/calls/agora-token — Generate Agora RTC token for video/audio call (PHP: agora.php)
/// Generates a privilege-based token compatible with Agora SDK
#[derive(Debug, serde::Deserialize)]
pub struct AgoraTokenRequest {
    pub channel_name: String,
    pub call_id: Option<i64>,
}

pub async fn generate_agora_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AgoraTokenRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.channel_name.trim().is_empty() {
        return Err(ApiError::BadRequest("channel_name is required".into()));
    }

    // Fetch Agora config from site_config
    let app_id: Option<String> = sqlx::query_scalar(
        "SELECT value FROM site_config WHERE category = 'agora' AND key = 'app_id'",
    )
    .fetch_optional(&state.db)
    .await?
    .flatten();

    let app_certificate: Option<String> = sqlx::query_scalar(
        "SELECT value FROM site_config WHERE category = 'agora' AND key = 'app_certificate'",
    )
    .fetch_optional(&state.db)
    .await?
    .flatten();

    let app_id = app_id.as_deref().unwrap_or("").trim().to_string();
    let app_cert = app_certificate.as_deref().unwrap_or("").trim().to_string();

    if app_id.is_empty() {
        return Err(ApiError::BadRequest("Agora is not configured".into()));
    }

    // If a call_id is provided, verify the caller is the caller/callee and update the room_name
    if let Some(cid) = req.call_id {
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM calls WHERE id = $1 AND (caller_id = $2 OR callee_id = $2))",
        )
        .bind(cid)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if !valid {
            return Err(ApiError::Forbidden("Call not found or access denied".into()));
        }

        sqlx::query("UPDATE calls SET room_name = $1 WHERE id = $2")
            .bind(req.channel_name.trim())
            .bind(cid)
            .execute(&state.db)
            .await?;
    }

    // Token lifetime: 10 hours (36000 s). Client should request a new one on expiry.
    const EXPIRE_SECS: u32 = 10 * 3600;
    let expire_ts = (time::OffsetDateTime::now_utc().unix_timestamp() + EXPIRE_SECS as i64) as u32;

    // Agora uses u32 UIDs. We clamp to avoid overflow on big user IDs.
    let uid: u32 = (auth.user_id as u64 & 0xFFFF_FFFF) as u32;

    // If an App Certificate is configured, produce a real HMAC-signed RTC token v006.
    // Without a certificate, Agora accepts the raw app_id as a token (only for testing
    // projects flagged as "App ID only" in the Agora console).
    let token = if app_cert.is_empty() {
        tracing::warn!(
            "Agora App Certificate not configured — returning App-ID-only token (insecure; test mode only)"
        );
        app_id.clone()
    } else {
        crate::agora_token::build(
            &app_id,
            &app_cert,
            req.channel_name.trim(),
            uid,
            crate::agora_token::Role::Publisher,
            EXPIRE_SECS,
        )
        .map_err(|e| ApiError::Internal(format!("Agora token generation failed: {}", e)))?
    };

    Ok(Json(json!({
        "data": {
            "token": token,
            "app_id": app_id,
            "channel": req.channel_name,
            "uid": uid,
            "expire_ts": expire_ts
        }
    })))
}

/// POST /v1/calls/viewer-token — Generate an Agora RTC **subscriber-only** token.
///
/// Use this for live-stream viewers and listen-only participants: the resulting
/// token can join the channel and receive audio/video, but **cannot publish**.
/// This is the proper way to scale Agora-based live broadcasts, as only the host
/// publishes (1 publisher → many subscribers).
#[derive(Debug, serde::Deserialize)]
pub struct ViewerTokenRequest {
    pub channel_name: String,
}

pub async fn generate_viewer_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ViewerTokenRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.channel_name.trim().is_empty() {
        return Err(ApiError::BadRequest("channel_name is required".into()));
    }

    let app_id: Option<String> = sqlx::query_scalar(
        "SELECT value FROM site_config WHERE category = 'agora' AND key = 'app_id'",
    )
    .fetch_optional(&state.db)
    .await?
    .flatten();

    let app_certificate: Option<String> = sqlx::query_scalar(
        "SELECT value FROM site_config WHERE category = 'agora' AND key = 'app_certificate'",
    )
    .fetch_optional(&state.db)
    .await?
    .flatten();

    let app_id = app_id.as_deref().unwrap_or("").trim().to_string();
    let app_cert = app_certificate.as_deref().unwrap_or("").trim().to_string();

    if app_id.is_empty() {
        return Err(ApiError::BadRequest("Agora is not configured".into()));
    }

    // Viewer tokens are short-lived (1 hour) — viewers can reconnect cheaply.
    const EXPIRE_SECS: u32 = 3600;
    let expire_ts = (time::OffsetDateTime::now_utc().unix_timestamp() + EXPIRE_SECS as i64) as u32;

    let uid: u32 = (auth.user_id as u64 & 0xFFFF_FFFF) as u32;

    let token = if app_cert.is_empty() {
        tracing::warn!("Agora App Certificate not configured — returning App-ID-only viewer token");
        app_id.clone()
    } else {
        crate::agora_token::build(
            &app_id,
            &app_cert,
            req.channel_name.trim(),
            uid,
            crate::agora_token::Role::Subscriber,
            EXPIRE_SECS,
        )
        .map_err(|e| ApiError::Internal(format!("Agora viewer token generation failed: {}", e)))?
    };

    Ok(Json(json!({
        "data": {
            "token": token,
            "app_id": app_id,
            "channel": req.channel_name,
            "uid": uid,
            "expire_ts": expire_ts,
            "role": "subscriber"
        }
    })))
}
