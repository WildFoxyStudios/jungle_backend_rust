use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

#[derive(Debug, Serialize, FromRow)]
pub struct CommentRow {
    pub id: i64,
    pub user_id: i64,
    pub post_id: i64,
    pub parent_id: Option<i64>,
    pub content: String,
    pub media: serde_json::Value,
    pub like_count: i32,
    pub reply_count: i32,
    pub created_at: OffsetDateTime,
}

pub async fn get_comments(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(post_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(0);

    let comments = sqlx::query_as::<_, CommentRow>(
        r#"SELECT id, user_id, post_id, parent_id, content, media, like_count, reply_count, created_at
           FROM comments
           WHERE post_id = $1 AND parent_id IS NULL AND id > $2
           ORDER BY created_at ASC
           LIMIT $3"#,
    )
    .bind(post_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = comments.len() as i64 > limit;
    let data: Vec<_> = comments.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|c| c.id.to_string());

    Ok(Json(serde_json::json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

pub async fn get_replies(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(comment_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(0);

    let replies = sqlx::query_as::<_, CommentRow>(
        r#"SELECT id, user_id, post_id, parent_id, content, media, like_count, reply_count, created_at
           FROM comments
           WHERE parent_id = $1 AND id > $2
           ORDER BY created_at ASC
           LIMIT $3"#,
    )
    .bind(comment_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = replies.len() as i64 > limit;
    let data: Vec<_> = replies.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|c| c.id.to_string());

    Ok(Json(serde_json::json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCommentRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: String,
    pub media: Option<serde_json::Value>,
    pub parent_id: Option<i64>,
}

pub async fn create_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let media = req.media.unwrap_or(serde_json::json!([]));

    let comment = sqlx::query_as::<_, CommentRow>(
        r#"INSERT INTO comments (user_id, post_id, parent_id, content, media)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, user_id, post_id, parent_id, content, media, like_count, reply_count, created_at"#,
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(req.parent_id)
    .bind(&req.content)
    .bind(&media)
    .fetch_one(&state.db)
    .await?;

    // Update denormalized counts
    sqlx::query("UPDATE posts SET comment_count = comment_count + 1 WHERE id = $1")
        .bind(post_id)
        .execute(&state.db)
        .await?;

    if let Some(parent_id) = req.parent_id {
        sqlx::query("UPDATE comments SET reply_count = reply_count + 1 WHERE id = $1")
            .bind(parent_id)
            .execute(&state.db)
            .await?;
    }

    // Publish event for notifications
    let author_id: i64 = sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let _ = state.event_bus.publish(&DomainEvent::CommentCreated {
        comment_id: comment.id,
        post_id,
        user_id: auth.user_id,
        author_id,
    }).await;

    Ok(Json(serde_json::json!({ "data": comment })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCommentRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: String,
}

pub async fn update_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<i64>,
    Json(req): Json<UpdateCommentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let comment = sqlx::query_as::<_, CommentRow>(
        r#"UPDATE comments SET content = $3, updated_at = NOW()
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, post_id, parent_id, content, media, like_count, reply_count, created_at"#,
    )
    .bind(comment_id)
    .bind(auth.user_id)
    .bind(&req.content)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Comment not found".into()))?;

    Ok(Json(serde_json::json!({ "data": comment })))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get post_id and parent_id before deleting
    #[derive(FromRow)]
    struct CommentMeta { post_id: i64, parent_id: Option<i64> }

    let meta = sqlx::query_as::<_, CommentMeta>(
        "SELECT post_id, parent_id FROM comments WHERE id = $1 AND user_id = $2",
    )
    .bind(comment_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Comment not found".into()))?;

    sqlx::query("DELETE FROM comments WHERE id = $1 AND user_id = $2")
        .bind(comment_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    // Update denormalized counts
    sqlx::query("UPDATE posts SET comment_count = GREATEST(comment_count - 1, 0) WHERE id = $1")
        .bind(meta.post_id)
        .execute(&state.db)
        .await?;

    if let Some(parent_id) = meta.parent_id {
        sqlx::query("UPDATE comments SET reply_count = GREATEST(reply_count - 1, 0) WHERE id = $1")
            .bind(parent_id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(serde_json::json!({ "data": { "message": "Comment deleted" } })))
}
