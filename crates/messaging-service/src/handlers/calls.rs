use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

use super::messages::append_message_with_notification;

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
    /// Legacy / explicit callee when not starting from a conversation.
    pub callee_id: Option<i64>,
    /// Preferred when placing a call from the chat header (resolves the peer in 1:1 chats).
    pub conversation_id: Option<i64>,
    #[serde(alias = "type")]
    pub call_type: String,
}

/// POST /v1/calls — initiate a call (creates a record)
pub async fn create_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateCallRequest>,
) -> Result<Json<Value>, ApiError> {
    if !["video", "audio"].contains(&req.call_type.as_str()) {
        return Err(ApiError::BadRequest(
            "call_type must be 'video' or 'audio'".into(),
        ));
    }

    let callee_id = if let Some(cid) = req.conversation_id {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT cm.user_id FROM conversation_members cm
            WHERE cm.conversation_id = $1 AND cm.is_active = TRUE AND cm.user_id <> $2
            ORDER BY cm.user_id
            LIMIT 1
            "#,
        )
        .bind(cid)
        .bind(auth.user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest("No peer found in conversation for this call".into())
        })?
    } else if let Some(uid) = req.callee_id {
        uid
    } else {
        return Err(ApiError::BadRequest(
            "callee_id or conversation_id is required".into(),
        ));
    };

    let now = OffsetDateTime::now_utc();
    let room = format!(
        "call_{}_{}",
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
    .bind(callee_id)
    .bind(&req.call_type)
    .bind(&room)
    .fetch_one(&state.db)
    .await?;

    // Fan-out: notify the callee (and caller) via realtime hub.
    if let Err(e) = state
        .event_bus
        .publish(&DomainEvent::CallStarted {
            call_id: call.id,
            caller_id: call.caller_id,
            callee_id: call.callee_id,
            call_type: call.call_type.clone(),
        })
        .await
    {
        tracing::warn!(call_id = call.id, error = %e, "failed to publish CallStarted");
    }

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

    Ok(Json(
        json!({ "data": calls, "meta": { "has_more": has_more } }),
    ))
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

    let call = sqlx::query_as::<_, CallRow>(
        r#"SELECT id, caller_id, callee_id, call_type, provider, room_name, status,
                  started_at, ended_at, created_at
           FROM calls WHERE id = $1 AND (caller_id = $2 OR callee_id = $3)"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Call not found".into()))?;

    if call.status == req.status {
        return Ok(Json(json!({ "data": { "id": id, "status": req.status } })));
    }

    let terminal = ["ended", "missed", "declined", "busy"];
    let was_terminal = terminal.contains(&call.status.as_str());
    let becomes_terminal = terminal.contains(&req.status.as_str());

    if req.status == "answered" {
        sqlx::query("UPDATE calls SET status = $1, started_at = NOW() WHERE id = $2")
            .bind(&req.status)
            .bind(id)
            .execute(&state.db)
            .await?;

        if let Err(e) = state
            .event_bus
            .publish(&DomainEvent::CallAnswered { call_id: id })
            .await
        {
            tracing::warn!(call_id = id, error = %e, "failed to publish CallAnswered");
        }
    } else {
        sqlx::query("UPDATE calls SET status = $1, ended_at = NOW() WHERE id = $2")
            .bind(&req.status)
            .bind(id)
            .execute(&state.db)
            .await?;

        if let Err(e) = state
            .event_bus
            .publish(&DomainEvent::CallEnded { call_id: id })
            .await
        {
            tracing::warn!(call_id = id, error = %e, "failed to publish CallEnded");
        }
    }

    if becomes_terminal && !was_terminal {
        if let Some(conv_id) =
            find_direct_conversation_between(&state, call.caller_id, call.callee_id).await
        {
            let label = call_timeline_label(&req.status, &call.call_type);
            let media = json!([{
                "call_id": id,
                "status": req.status,
                "call_type": call.call_type,
            }]);
            if let Err(e) = append_message_with_notification(
                &state,
                conv_id,
                auth.user_id,
                label,
                "call".into(),
                media,
            )
            .await
            {
                tracing::warn!(call_id = id, error = %e, "failed to append call timeline message");
            }
        }
    }

    Ok(Json(json!({ "data": { "id": id, "status": req.status } })))
}

fn call_timeline_label(status: &str, call_type: &str) -> String {
    let video = call_type == "video";
    match status {
        "missed" if video => "Missed video call".into(),
        "missed" => "Missed audio call".into(),
        "declined" => "Call declined".into(),
        "busy" => "Line busy".into(),
        "ended" if video => "Video call ended".into(),
        "ended" => "Audio call ended".into(),
        _ => "Call ended".into(),
    }
}

async fn find_direct_conversation_between(
    state: &AppState,
    user_a: i64,
    user_b: i64,
) -> Option<i64> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT c.id FROM conversations c
        WHERE c.type = 'direct'
          AND EXISTS(
            SELECT 1 FROM conversation_members cm
            WHERE cm.conversation_id = c.id AND cm.user_id = $1 AND cm.is_active = TRUE
          )
          AND EXISTS(
            SELECT 1 FROM conversation_members cm
            WHERE cm.conversation_id = c.id AND cm.user_id = $2 AND cm.is_active = TRUE
          )
        LIMIT 1
        "#,
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
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
            return Err(ApiError::Forbidden(
                "Call not found or access denied".into(),
            ));
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

/// POST /v1/calls/twilio-token — Generate a Twilio Video Access Token.
///
/// Used when admins set the active video provider to Twilio (PHP parity:
/// `system → live` with `video_provider = twilio`). Mirrors the Agora
/// endpoint shape so frontends can swap providers transparently.
#[derive(Debug, serde::Deserialize)]
pub struct TwilioTokenRequest {
    pub room: String,
    pub call_id: Option<i64>,
}

pub async fn generate_twilio_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TwilioTokenRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.room.trim().is_empty() {
        return Err(ApiError::BadRequest("room is required".into()));
    }

    let cfg = load_twilio_config(&state).await?;

    if let Some(cid) = req.call_id {
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM calls WHERE id = $1 AND (caller_id = $2 OR callee_id = $2))",
        )
        .bind(cid)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if !valid {
            return Err(ApiError::Forbidden(
                "Call not found or access denied".into(),
            ));
        }

        sqlx::query("UPDATE calls SET room_name = $1, provider = 'twilio' WHERE id = $2")
            .bind(req.room.trim())
            .bind(cid)
            .execute(&state.db)
            .await?;
    }

    const TTL_SECS: u64 = 4 * 3600;
    let identity = auth.user_id.to_string();

    let (token, exp) = crate::twilio_token::build(
        &cfg.account_sid,
        &cfg.api_key_sid,
        &cfg.api_key_secret,
        &identity,
        req.room.trim(),
        TTL_SECS,
    )
    .map_err(ApiError::Internal)?;

    Ok(Json(json!({
        "data": {
            "provider": "twilio",
            "token": token,
            "identity": identity,
            "room": req.room,
            "expire_ts": exp,
        }
    })))
}

struct TwilioConfig {
    account_sid: String,
    api_key_sid: String,
    api_key_secret: String,
}

async fn load_twilio_config(state: &AppState) -> Result<TwilioConfig, ApiError> {
    let read = |k: &'static str| {
        let pool = state.db.clone();
        async move {
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT value FROM site_config WHERE category = 'twilio' AND key = $1",
            )
            .bind(k)
            .fetch_optional(&pool)
            .await
            .map(|opt| opt.flatten().unwrap_or_default())
        }
    };

    let account_sid = read("account_sid").await?.trim().to_string();
    let api_key_sid = read("api_key_sid").await?.trim().to_string();
    let api_key_secret = read("api_key_secret").await?.trim().to_string();

    if account_sid.is_empty() || api_key_sid.is_empty() || api_key_secret.is_empty() {
        return Err(ApiError::BadRequest(
            "Twilio Video is not configured".into(),
        ));
    }

    Ok(TwilioConfig {
        account_sid,
        api_key_sid,
        api_key_secret,
    })
}
