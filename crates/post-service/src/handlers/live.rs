use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    models::user::PublicUserRow,
    pagination::PaginationParams,
};
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct LiveStreamRow {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub stream_key: String,
    pub status: String,
    pub viewer_count: i32,
    pub created_at: OffsetDateTime,
}

/// POST /v1/live/start
pub async fn start_live(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<StartLiveRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Generate unique stream key
    let stream_key = uuid::Uuid::new_v4().to_string();

    let stream = sqlx::query_as::<_, LiveStreamRow>(
        r#"INSERT INTO live_streams (user_id, title, stream_key, status)
           VALUES ($1, $2, $3, 'live')
           RETURNING id, user_id, title, stream_key, status, viewer_count, created_at"#,
    )
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&stream_key)
    .fetch_one(&state.db)
    .await?;

    // Create a post for the live stream
    sqlx::query(
        r#"INSERT INTO posts (user_id, content, post_type, privacy)
           VALUES ($1, $2, 'live', 'everyone')"#,
    )
    .bind(auth.user_id)
    .bind(format!("🔴 Live: {}", &req.title))
    .execute(&state.db)
    .await?;

    let _ = state
        .event_bus
        .publish(&DomainEvent::LiveStreamStarted {
            stream_id: stream.id,
            user_id: auth.user_id,
        })
        .await;

    Ok(Json(json!({ "data": stream })))
}

/// GET /v1/live/{id} — single stream + publisher (for watch UI).
pub async fn get_live(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(stream_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let stream = sqlx::query_as::<_, LiveStreamRow>(
        r#"SELECT id, user_id, title, stream_key, status, viewer_count, created_at
           FROM live_streams WHERE id = $1"#,
    )
    .bind(stream_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Live stream not found".into()))?;

    let publishers = publishers_for_user_ids(&state.db, &[stream.user_id]).await;
    let mut val = serde_json::to_value(&stream).unwrap_or_default();
    if let Some(obj) = val.as_object_mut()
        && let Some(pub_row) = publishers.get(&stream.user_id)
    {
        obj.insert(
            "publisher".into(),
            serde_json::to_value(pub_row).unwrap_or_default(),
        );
    }
    Ok(Json(json!({ "data": val })))
}

async fn publishers_for_user_ids(db: &sqlx::PgPool, user_ids: &[i64]) -> HashMap<i64, PublicUserRow> {
    if user_ids.is_empty() {
        return HashMap::new();
    }
    let rows = sqlx::query_as::<_, PublicUserRow>(
        r#"SELECT uuid, username, first_name, last_name, avatar, cover, about, is_verified, is_pro
           FROM users WHERE id = ANY($1)"#,
    )
    .bind(user_ids)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let id_rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, username FROM users WHERE id = ANY($1)")
        .bind(user_ids)
        .fetch_all(db)
        .await
        .unwrap_or_default();

    let username_map: HashMap<String, PublicUserRow> =
        rows.into_iter().map(|r| (r.username.clone(), r)).collect();
    id_rows
        .into_iter()
        .filter_map(|(id, uname)| username_map.get(&uname).cloned().map(|p| (id, p)))
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct StartLiveRequest {
    pub title: String,
}

/// POST /v1/live/stop
pub async fn stop_live(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE live_streams SET status = 'ended', ended_at = NOW() WHERE user_id = $1 AND status = 'live'",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("No active live stream found".into()));
    }

    // Get stream id for event
    if let Ok(Some(stream_id)) = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM live_streams WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    {
        let _ = state
            .event_bus
            .publish(&DomainEvent::LiveStreamEnded {
                stream_id,
                user_id: auth.user_id,
            })
            .await;
    }

    Ok(Json(json!({ "data": { "stopped": true } })))
}

/// GET /v1/live/active — list all active live streams
pub async fn active_lives(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let streams = sqlx::query_as::<_, LiveStreamRow>(
        r#"SELECT id, user_id, title, stream_key, status, viewer_count, created_at
           FROM live_streams
           WHERE status = 'live' AND id < $1
           ORDER BY viewer_count DESC, id DESC
           LIMIT $2"#,
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = streams.len() as i64 > limit;
    let data: Vec<_> = streams.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|s| s.id.to_string());

    let uids: Vec<i64> = data.iter().map(|s| s.user_id).collect::<std::collections::HashSet<_>>().into_iter().collect();
    let publishers = publishers_for_user_ids(&state.db, &uids).await;
    let out: Vec<serde_json::Value> = data
        .iter()
        .map(|row| {
            let mut v = serde_json::to_value(row).unwrap_or_default();
            if let Some(obj) = v.as_object_mut()
                && let Some(p) = publishers.get(&row.user_id)
            {
                obj.insert("publisher".into(), serde_json::to_value(p).unwrap_or_default());
            }
            v
        })
        .collect();

    Ok(Json(json!({
        "data": out,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// POST /v1/live/{id}/comment
pub async fn live_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(stream_id): Path<i64>,
    Json(req): Json<LiveCommentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("INSERT INTO live_comments (stream_id, user_id, content) VALUES ($1, $2, $3)")
        .bind(stream_id)
        .bind(auth.user_id)
        .bind(&req.content)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "commented": true } })))
}

#[derive(Debug, Deserialize)]
pub struct LiveCommentRequest {
    pub content: String,
}

/// POST /v1/live/{id}/react
pub async fn live_react(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(stream_id): Path<i64>,
    Json(req): Json<LiveReactRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        r#"INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
           VALUES ($1, 'live', $2, $3)
           ON CONFLICT (user_id, target_type, target_id) DO UPDATE SET reaction_type = $3"#,
    )
    .bind(auth.user_id)
    .bind(stream_id)
    .bind(&req.reaction)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "reaction": req.reaction } })))
}

#[derive(Debug, Deserialize)]
pub struct LiveReactRequest {
    pub reaction: String,
}

/// GET /v1/live/{id}/vod — retrieve the post-live VOD playback metadata.
///
/// Returns 404 while the stream is still live or no VOD has been
/// produced. Returns the HLS playback URL + thumbnail + duration once
/// the publisher has signalled `vod_ready_at`.
///
/// The endpoint is intentionally read-only and unauthenticated-safe
/// (it still requires AuthUser to keep parity with the rest of /v1/live).
pub async fn live_vod(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(stream_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, LiveVodRow>(
        r#"SELECT id, user_id, title, status, vod_url, vod_thumbnail,
                  vod_duration_seconds, vod_ready_at, ended_at
           FROM live_streams
           WHERE id = $1"#,
    )
    .bind(stream_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Live stream not found".into()))?;

    let vod_url = match row.vod_url {
        Some(u) if !u.is_empty() => u,
        _ => return Err(ApiError::NotFound("VOD not available yet".into())),
    };

    Ok(Json(json!({
        "data": {
            "stream_id": row.id,
            "user_id": row.user_id,
            "title": row.title,
            "status": row.status,
            "vod_url": vod_url,
            "vod_thumbnail": row.vod_thumbnail,
            "vod_duration_seconds": row.vod_duration_seconds,
            "vod_ready_at": row.vod_ready_at,
            "ended_at": row.ended_at,
        }
    })))
}

#[derive(Debug, FromRow)]
struct LiveVodRow {
    id: i64,
    user_id: i64,
    title: String,
    status: String,
    vod_url: Option<String>,
    vod_thumbnail: Option<String>,
    vod_duration_seconds: Option<i32>,
    vod_ready_at: Option<OffsetDateTime>,
    ended_at: Option<OffsetDateTime>,
}

/// PATCH /v1/live/{id}/vod — operator-side hook so the transcoder can
/// publish back the VOD URL once it finishes packaging.
///
/// Only the original streamer (or admin via gateway) can call this.
/// Field-level updates: any field passed is overwritten; missing
/// fields are left untouched. Setting `vod_url` automatically stamps
/// `vod_ready_at = NOW()` if the publisher didn't supply one.
pub async fn live_vod_publish(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(stream_id): Path<i64>,
    Json(req): Json<LiveVodPublishRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let owner: Option<i64> = sqlx::query_scalar("SELECT user_id FROM live_streams WHERE id = $1")
        .bind(stream_id)
        .fetch_optional(&state.db)
        .await?;
    let owner = owner.ok_or_else(|| ApiError::NotFound("Live stream not found".into()))?;
    if owner != auth.user_id {
        return Err(ApiError::Forbidden(
            "Cannot publish VOD for another user's stream".into(),
        ));
    }

    sqlx::query(
        r#"UPDATE live_streams
           SET vod_url               = COALESCE($2, vod_url),
               vod_thumbnail         = COALESCE($3, vod_thumbnail),
               vod_duration_seconds  = COALESCE($4, vod_duration_seconds),
               vod_ready_at          = COALESCE($5,
                   CASE WHEN $2 IS NOT NULL THEN NOW() ELSE vod_ready_at END)
           WHERE id = $1"#,
    )
    .bind(stream_id)
    .bind(req.vod_url.as_deref())
    .bind(req.vod_thumbnail.as_deref())
    .bind(req.vod_duration_seconds)
    .bind(req.vod_ready_at)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "published": true } })))
}

#[derive(Debug, Deserialize)]
pub struct LiveVodPublishRequest {
    pub vod_url: Option<String>,
    pub vod_thumbnail: Option<String>,
    pub vod_duration_seconds: Option<i32>,
    pub vod_ready_at: Option<OffsetDateTime>,
}

/// GET /v1/live/friends — list live streams from friends
pub async fn live_friends(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let streams = sqlx::query_as::<_, LiveStreamRow>(
        r#"SELECT ls.id, ls.user_id, ls.title, ls.stream_key, ls.status, ls.viewer_count, ls.created_at
           FROM live_streams ls
           WHERE ls.status = 'live'
             AND ls.user_id IN (
                 SELECT following_id FROM follows WHERE follower_id = $1 AND status = 'active'
             )
             AND ls.id < $2
           ORDER BY ls.created_at DESC
           LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = streams.len() as i64 > limit;
    let data: Vec<_> = streams.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|s| s.id.to_string());

    let uids: Vec<i64> = data.iter().map(|s| s.user_id).collect::<std::collections::HashSet<_>>().into_iter().collect();
    let publishers = publishers_for_user_ids(&state.db, &uids).await;
    let out: Vec<serde_json::Value> = data
        .iter()
        .map(|row| {
            let mut v = serde_json::to_value(row).unwrap_or_default();
            if let Some(obj) = v.as_object_mut()
                && let Some(p) = publishers.get(&row.user_id)
            {
                obj.insert("publisher".into(), serde_json::to_value(p).unwrap_or_default());
            }
            v
        })
        .collect();

    Ok(Json(json!({
        "data": out,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ── Millicast Live ────────────────────────────────────────────────

/// POST /v1/live/millicast/publish-token — Mint Millicast publish credentials
/// (JWT + WebSocket URL) so the user can broadcast through `@millicast/sdk`.
#[derive(Debug, Deserialize)]
pub struct MillicastPublishRequest {
    pub stream_name: String,
}

pub async fn millicast_publish_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<MillicastPublishRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let stream = req.stream_name.trim();
    if stream.is_empty() {
        return Err(ApiError::BadRequest("stream_name is required".into()));
    }

    let cfg = load_millicast_config(&state).await?;
    let publish_token = cfg
        .publish_token
        .ok_or_else(|| ApiError::BadRequest("Millicast publish_token is not configured".into()))?;

    let http = reqwest::Client::new();
    let res = crate::millicast::publish_token(&http, &publish_token, stream)
        .await
        .map_err(ApiError::Internal)?;

    Ok(Json(json!({
        "data": {
            "provider": "millicast",
            "stream_name": stream,
            "account_id": cfg.account_id,
            "user_id": auth.user_id,
            "jwt": res.jwt,
            "urls": res.urls,
        }
    })))
}

/// POST /v1/live/millicast/subscribe-token — Mint Millicast subscribe credentials
/// for a viewer.
#[derive(Debug, Deserialize)]
pub struct MillicastSubscribeRequest {
    pub stream_name: String,
}

pub async fn millicast_subscribe_token(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<MillicastSubscribeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let stream = req.stream_name.trim();
    if stream.is_empty() {
        return Err(ApiError::BadRequest("stream_name is required".into()));
    }

    let cfg = load_millicast_config(&state).await?;
    let token = cfg
        .subscribe_token
        .as_ref()
        .or(cfg.publish_token.as_ref())
        .ok_or_else(|| ApiError::BadRequest("Millicast subscribe_token is not configured".into()))?
        .clone();

    let http = reqwest::Client::new();
    let res = crate::millicast::subscribe_token(&http, &token, stream, cfg.account_id.as_deref())
        .await
        .map_err(ApiError::Internal)?;

    Ok(Json(json!({
        "data": {
            "provider": "millicast",
            "stream_name": stream,
            "account_id": cfg.account_id,
            "jwt": res.jwt,
            "urls": res.urls,
        }
    })))
}

struct MillicastConfig {
    publish_token: Option<String>,
    subscribe_token: Option<String>,
    account_id: Option<String>,
}

async fn load_millicast_config(state: &AppState) -> Result<MillicastConfig, ApiError> {
    let publish_token = read_millicast_value(state, "publish_token").await?;
    let subscribe_token = read_millicast_value(state, "subscribe_token").await?;
    let account_id = read_millicast_value(state, "account_id").await?;
    Ok(MillicastConfig {
        publish_token,
        subscribe_token,
        account_id,
    })
}

async fn read_millicast_value(state: &AppState, key: &str) -> Result<Option<String>, ApiError> {
    let v = sqlx::query_scalar::<_, Option<String>>(
        "SELECT value FROM site_config WHERE category = 'millicast' AND key = $1",
    )
    .bind(key)
    .fetch_optional(&state.db)
    .await?;
    Ok(v.flatten().filter(|v| !v.trim().is_empty()))
}

// ═══════════════════════════════════════════════════════════════
// Phase 16: Co-hosts
// ═══════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct InviteCohostRequest {
    pub user_id: i64,
}

pub async fn invite_cohost(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(live_id): Path<i64>,
    Json(body): Json<InviteCohostRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "INSERT INTO live_cohosts (live_id, user_id, role) VALUES ($1, $2, 'cohost') ON CONFLICT DO NOTHING"
    )
    .bind(live_id).bind(body.user_id)
    .execute(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    Ok(Json(serde_json::json!({ "data": { "live_id": live_id, "cohost_id": body.user_id, "status": "invited" } })))
}

pub async fn accept_cohost(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(live_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query(
        "UPDATE live_cohosts SET accepted_at = NOW() WHERE live_id = $1 AND user_id = $2 AND accepted_at IS NULL"
    )
    .bind(live_id).bind(auth.user_id)
    .execute(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    if rows.rows_affected() == 0 {
        return Err(ApiError::NotFound("No pending co-host invitation found".into()));
    }
    Ok(Json(serde_json::json!({ "data": null })))
}

pub async fn remove_cohost(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((live_id, user_id)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM live_cohosts WHERE live_id = $1 AND user_id = $2")
        .bind(live_id).bind(user_id)
        .execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(serde_json::json!({ "data": null })))
}

// ═══════════════════════════════════════════════════════════════
// Phase 16: Live Polls
// ═══════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct CreateLivePollRequest {
    pub question: String,
    pub options: Vec<String>,
}

pub async fn create_live_poll(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(live_id): Path<i64>,
    Json(body): Json<CreateLivePollRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.options.len() < 2 {
        return Err(ApiError::BadRequest("At least 2 options required".into()));
    }
    if body.question.trim().is_empty() {
        return Err(ApiError::BadRequest("Question is required".into()));
    }

    let options_json = serde_json::to_value(&body.options).unwrap_or_default();
    let row = sqlx::query(
        "INSERT INTO live_polls (live_id, question, options, created_by) VALUES ($1, $2, $3, $4) RETURNING id"
    )
    .bind(live_id).bind(body.question.trim()).bind(&options_json).bind(auth.user_id)
    .fetch_one(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let id: i64 = row.get("id");
    Ok(Json(serde_json::json!({ "data": { "id": id, "question": body.question, "options": body.options } })))
}

#[derive(Deserialize)]
pub struct VotePollRequest {
    pub option_index: i32,
}

pub async fn vote_live_poll(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_live_id, poll_id)): Path<(i64, i64)>,
    Json(body): Json<VotePollRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "INSERT INTO live_poll_votes (poll_id, user_id, option_index) VALUES ($1, $2, $3) ON CONFLICT (poll_id, user_id) DO UPDATE SET option_index = $3"
    )
    .bind(poll_id).bind(auth.user_id).bind(body.option_index)
    .execute(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    // Return current results
    let results = sqlx::query(
        "SELECT option_index, COUNT(*) as cnt FROM live_poll_votes WHERE poll_id = $1 GROUP BY option_index ORDER BY option_index"
    )
    .bind(poll_id).fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let votes: Vec<serde_json::Value> = results.iter().map(|r| serde_json::json!({
        "option_index": r.get::<i32, _>("option_index"),
        "count": r.get::<i64, _>("cnt"),
    })).collect();

    Ok(Json(serde_json::json!({ "data": { "poll_id": poll_id, "results": votes } })))
}

pub async fn get_live_polls(
    State(state): State<AppState>,
    Path(live_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let polls = sqlx::query(
        "SELECT id, question, options, is_active, created_at FROM live_polls WHERE live_id = $1 ORDER BY created_at DESC"
    )
    .bind(live_id).fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let mut result = Vec::new();
    for poll in &polls {
        let poll_id: i64 = poll.get("id");
        let votes = sqlx::query(
            "SELECT option_index, COUNT(*) as cnt FROM live_poll_votes WHERE poll_id = $1 GROUP BY option_index"
        )
        .bind(poll_id).fetch_all(&state.db).await.unwrap_or_default();

        let results: Vec<serde_json::Value> = votes.iter().map(|r| serde_json::json!({
            "option_index": r.get::<i32, _>("option_index"),
            "count": r.get::<i64, _>("cnt"),
        })).collect();

        result.push(serde_json::json!({
            "id": poll_id,
            "question": poll.get::<String, _>("question"),
            "options": poll.get::<serde_json::Value, _>("options"),
            "is_active": poll.get::<bool, _>("is_active"),
            "results": results,
        }));
    }

    Ok(Json(serde_json::json!({ "data": result })))
}

// ═══════════════════════════════════════════════════════════════
// Phase 16: VOD listing
// ═══════════════════════════════════════════════════════════════

pub async fn list_vods(
    State(state): State<AppState>,
    Query(params): Query<shared::pagination::PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit.unwrap_or(20).min(50);
    let cursor: i64 = params.cursor.as_deref().and_then(|c| c.parse().ok()).unwrap_or(0);

    let rows = sqlx::query(
        "SELECT ls.id, ls.user_id, ls.title, ls.vod_url, ls.vod_thumbnail,
                ls.vod_duration_seconds, ls.vod_ready_at, ls.ended_at,
                u.username, u.first_name, u.last_name, u.avatar
         FROM live_streams ls
         JOIN users u ON u.id = ls.user_id
         WHERE ls.vod_url IS NOT NULL AND ls.id > $1
         ORDER BY ls.vod_ready_at DESC LIMIT $2"
    )
    .bind(cursor).bind(limit)
    .fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let vods: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<i64, _>("id"),
        "user_id": r.get::<i64, _>("user_id"),
        "title": r.get::<String, _>("title"),
        "vod_url": r.get::<Option<String>, _>("vod_url"),
        "vod_thumbnail": r.get::<Option<String>, _>("vod_thumbnail"),
        "vod_duration_seconds": r.get::<Option<i32>, _>("vod_duration_seconds"),
        "vod_ready_at": r.get::<Option<String>, _>("vod_ready_at"),
        "ended_at": r.get::<Option<String>, _>("ended_at"),
        "username": r.get::<String, _>("username"),
        "first_name": r.get::<String, _>("first_name"),
        "last_name": r.get::<String, _>("last_name"),
        "avatar": r.get::<Option<String>, _>("avatar"),
    })).collect();

    let next_cursor = vods.last().map(|v| v["id"].as_i64().unwrap_or(0));
    Ok(Json(serde_json::json!({ "data": { "items": vods, "next_cursor": next_cursor } })))
}
