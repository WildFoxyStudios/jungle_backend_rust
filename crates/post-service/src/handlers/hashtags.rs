use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::AppState,
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
    pub is_trending: bool,
    pub created_at: Option<OffsetDateTime>,
}

pub async fn trending_hashtags(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let tags = sqlx::query_as::<_, HashtagRow>(
        "SELECT id, tag, use_count, is_trending, created_at FROM hashtags WHERE is_trending = TRUE ORDER BY use_count DESC LIMIT 20",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": tags })))
}

pub async fn posts_by_hashtag(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

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
             AND ($2::bigint IS NULL OR p.id < $2)
           ORDER BY p.id DESC LIMIT $3"#,
    )
    .bind(&tag)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": data, "meta": { "has_more": has_more } })))
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
        "SELECT id, tag, use_count, is_trending, created_at FROM hashtags WHERE tag ILIKE $1 ORDER BY use_count DESC LIMIT 20",
    )
    .bind(&ilike)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": tags })))
}
