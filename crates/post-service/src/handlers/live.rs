use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;
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
    let _ = sqlx::query(
        r#"INSERT INTO posts (user_id, content, post_type, privacy)
           VALUES ($1, $2, 'live', 'everyone')"#,
    )
    .bind(auth.user_id)
    .bind(format!("🔴 Live: {}", &req.title))
    .execute(&state.db)
    .await;

    let _ = state.event_bus.publish(&DomainEvent::LiveStreamStarted {
        stream_id: stream.id,
        user_id: auth.user_id,
    }).await;

    Ok(Json(json!({ "data": stream })))
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
        "SELECT id FROM live_streams WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1"
    ).bind(auth.user_id).fetch_optional(&state.db).await {
        let _ = state.event_bus.publish(&DomainEvent::LiveStreamEnded {
            stream_id,
            user_id: auth.user_id,
        }).await;
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

    Ok(Json(json!({
        "data": data,
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
    sqlx::query(
        "INSERT INTO live_comments (stream_id, user_id, content) VALUES ($1, $2, $3)",
    )
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

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}
