use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, OptionalAuth},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct HashtagRow {
    pub id: i64,
    pub tag: String,
    pub use_count: i32,
    pub trending: bool,
    pub last_used_at: Option<OffsetDateTime>,
}

pub async fn trending_hashtags(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let tags = sqlx::query_as::<_, HashtagRow>(
        "SELECT id, tag, use_count, trending, last_used_at FROM hashtags WHERE trending = TRUE ORDER BY use_count DESC LIMIT 20",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": tags })))
}

pub async fn posts_by_hashtag(
    State(state): State<AppState>,
    auth: OptionalAuth,
    Path(tag): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let uid = auth.0.as_ref().map(|u| u.user_id);

    let rows = sqlx::query_as::<_, super::posts::PostRow>(
        r#"SELECT p.id, p.uuid, p.user_id, p.parent_id, p.content, p.post_type, p.media,
                  p.privacy, p.feeling, p.location, p.is_pinned, p.is_boosted, p.is_reel,
                  p.like_count, p.comment_count, p.share_count, p.view_count,
                  p.created_at, p.updated_at
           FROM posts p
           JOIN post_hashtags ph ON ph.post_id = p.id
           JOIN hashtags h ON h.id = ph.hashtag_id
           WHERE h.tag = $1
             AND p.deleted_at IS NULL AND p.is_approved = TRUE
             AND (p.privacy = 'everyone'
                  OR ($2::bigint IS NOT NULL AND p.user_id = $2)
                  OR (p.privacy = 'only_me' AND $2::bigint IS NOT NULL AND p.user_id = $2))
             AND ($2::bigint IS NULL OR p.user_id NOT IN (SELECT blocked_id FROM blocks WHERE blocker_id = $2))
             AND ($3::bigint IS NULL OR p.id < $3)
           ORDER BY p.id DESC LIMIT $4"#,
    )
    .bind(&tag)
    .bind(uid)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": data, "meta": { "has_more": has_more } }),
    ))
}

/// GET /v1/hashtags/{tag}/reels — only reel posts
pub async fn reels_by_hashtag(
    State(state): State<AppState>,
    auth: OptionalAuth,
    Path(tag): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let uid = auth.0.map(|u| u.user_id).unwrap_or(0);

    let rows = sqlx::query_as::<_, super::reels::ReelRow>(
        r#"SELECT p.id, p.user_id, p.content, p.media,
                  p.like_count, p.comment_count, p.share_count, p.view_count, p.comments_status,
                  u.uuid, u.username, u.first_name, u.last_name, u.avatar,
                  u.is_verified, u.is_online, u.is_pro,
                  r.reaction_type AS my_reaction,
                  EXISTS(SELECT 1 FROM follows f WHERE f.follower_id = $3 AND f.following_id = p.user_id AND f.status = 'active' AND p.user_id <> $3) AS is_following,
                  EXISTS(SELECT 1 FROM saved_posts sp WHERE sp.user_id = $3 AND sp.post_id = p.id) AS is_saved,
                  p.audio_track_id, at.title AS audio_track_title, at.artist_label AS audio_track_artist, at.source AS audio_track_source, p.remix_of_post_id, p.allow_remix,
                  p.created_at
           FROM posts p
           JOIN post_hashtags ph ON ph.post_id = p.id
           JOIN hashtags h ON h.id = ph.hashtag_id
           JOIN users u ON u.id = p.user_id
           LEFT JOIN reel_audio_tracks at ON at.id = p.audio_track_id
           LEFT JOIN reactions r
             ON r.target_type = 'post' AND r.target_id = p.id AND r.user_id = $3
           WHERE h.tag = $1
             AND p.is_reel = TRUE
             AND p.deleted_at IS NULL AND p.is_approved = TRUE
             AND p.privacy = 'everyone'
             AND p.user_id NOT IN (SELECT blocked_id FROM blocks WHERE blocker_id = $3)
             AND ($2::bigint IS NULL OR p.id < $2)
           ORDER BY p.id DESC LIMIT $4"#,
    )
    .bind(&tag)
    .bind(cursor)
    .bind(uid)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<Value> = rows
        .into_iter()
        .take(limit as usize)
        .map(super::reels::reel_to_json)
        .collect();
    let next_cursor = data.last().and_then(|d| d["id"].as_i64());
    Ok(Json(
        json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct SearchHashtagQuery {
    pub q: String,
}

pub async fn search_hashtags(
    State(state): State<AppState>,
    Query(q): Query<SearchHashtagQuery>,
) -> Result<Json<Value>, ApiError> {
    let ilike = format!("{}%", q.q);
    let tags = sqlx::query_as::<_, HashtagRow>(
        "SELECT id, tag, use_count, trending, last_used_at FROM hashtags WHERE tag ILIKE $1 ORDER BY use_count DESC LIMIT 20",
    )
    .bind(&ilike)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": tags })))
}
