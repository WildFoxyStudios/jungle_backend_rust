use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};

#[derive(Debug, Deserialize)]
pub struct ReelsFeedQuery {
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /v1/reels — trending/random reels feed
pub async fn get_reels_feed(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ReelsFeedQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 50);

    let rows = sqlx::query_as::<_, (i64, Option<String>, Option<Value>, i64, i64, i64, String, String, time::OffsetDateTime)>(
        r#"
        SELECT p.id, p.content, p.media, p.like_count, p.comment_count, p.view_count,
               u.username, u.avatar, p.created_at
        FROM posts p
        JOIN users u ON u.id = p.user_id
        WHERE p.is_reel = TRUE
          AND p.deleted_at IS NULL
          AND p.is_approved = TRUE
          AND p.privacy = 'everyone'
          AND p.user_id NOT IN (
              SELECT blocked_id FROM blocks WHERE blocker_id = $1
              UNION
              SELECT blocker_id FROM blocks WHERE blocked_id = $1
          )
          AND ($2::bigint IS NULL OR p.id < $2)
        ORDER BY (p.like_count + p.comment_count + p.view_count) DESC, p.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(q.cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<Value> = rows
        .into_iter()
        .take(limit as usize)
        .map(|(id, content, media, likes, comments, views, username, avatar, created_at)| {
            json!({
                "id": id,
                "content": content,
                "media": media,
                "like_count": likes,
                "comment_count": comments,
                "view_count": views,
                "author": { "username": username, "avatar": avatar },
                "created_at": created_at.to_string()
            })
        })
        .collect();

    let next_cursor = data.last().and_then(|d| d["id"].as_i64());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// POST /v1/reels/{id}/view — increment view count
pub async fn view_reel(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE posts SET view_count = COALESCE(view_count, 0) + 1 WHERE id = $1 AND is_reel = TRUE")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "viewed": true } })))
}

#[derive(Debug, Deserialize)]
pub struct CreateReelRequest {
    pub content: Option<String>,
    pub media: Option<Value>,
}

/// POST /v1/reels — create a reel (a post with is_reel = true)
pub async fn create_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateReelRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let row = sqlx::query_as::<_, (i64,)>(
        r#"INSERT INTO posts (user_id, content, media, is_reel, post_type, privacy, is_approved)
           VALUES ($1, $2, $3, TRUE, 'reel', 'everyone', TRUE)
           RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(&req.content)
    .bind(&req.media)
    .fetch_one(&state.db)
    .await?;

    let _ = state.event_bus.publish(&DomainEvent::PostCreated {
        post_id: row.0,
        user_id: auth.user_id,
        group_id: None,
        page_id: None,
    }).await;

    Ok((StatusCode::CREATED, Json(json!({ "data": { "id": row.0 } }))))
}

/// GET /v1/reels/{id}
pub async fn get_reel(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64, i64, Option<String>, Option<Value>, i64, i64, i64, String, String, time::OffsetDateTime)>(
        r#"SELECT p.id, p.user_id, p.content, p.media, p.like_count, p.comment_count, p.view_count,
                  u.username, u.avatar, p.created_at
           FROM posts p JOIN users u ON u.id = p.user_id
           WHERE p.id = $1 AND p.is_reel = TRUE AND p.deleted_at IS NULL"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Reel not found".into()))?;

    Ok(Json(json!({
        "data": {
            "id": row.0, "user_id": row.1, "content": row.2, "media": row.3,
            "like_count": row.4, "comment_count": row.5, "view_count": row.6,
            "author": { "username": row.7, "avatar": row.8 },
            "created_at": row.9.to_string()
        }
    })))
}

/// DELETE /v1/reels/{id}
pub async fn delete_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND user_id = $2 AND is_reel = TRUE AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Reel not found or not owned".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct ReelReactRequest {
    pub reaction: String,
}

/// POST /v1/reels/{id}/react
pub async fn react_to_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReelReactRequest>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        r#"INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
           VALUES ($1, 'post', $2, $3)
           ON CONFLICT (user_id, target_type, target_id) DO UPDATE SET reaction_type = $3"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&req.reaction)
    .execute(&state.db)
    .await?;

    sqlx::query("UPDATE posts SET like_count = (SELECT COUNT(*) FROM reactions WHERE target_type = 'post' AND target_id = $1) WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "reaction": req.reaction } })))
}

/// GET /v1/reels/{id}/comments
pub async fn reel_comments(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, (i64, i64, String, String, String, time::OffsetDateTime)>(
        r#"SELECT c.id, c.user_id, c.content, u.username, u.avatar, c.created_at
           FROM comments c JOIN users u ON u.id = c.user_id
           WHERE c.post_id = $1 AND c.parent_id IS NULL AND c.id < $2
           ORDER BY c.id DESC LIMIT $3"#,
    )
    .bind(id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<Value> = rows.into_iter().take(limit as usize).map(|r| {
        json!({
            "id": r.0, "user_id": r.1, "content": r.2,
            "author": { "username": r.3, "avatar": r.4 },
            "created_at": r.5.to_string()
        })
    }).collect();
    let next_cursor = data.last().and_then(|d| d["id"].as_i64());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[derive(Debug, Deserialize)]
pub struct ReelCommentRequest {
    pub content: String,
}

/// POST /v1/reels/{id}/comment
pub async fn add_reel_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReelCommentRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let row = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO comments (post_id, user_id, content) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.content)
    .fetch_one(&state.db)
    .await?;

    sqlx::query("UPDATE posts SET comment_count = comment_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok((StatusCode::CREATED, Json(json!({ "data": { "id": row.0 } }))))
}
