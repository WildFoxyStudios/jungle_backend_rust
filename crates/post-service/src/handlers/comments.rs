use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, FromRow, Clone)]
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
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
struct CommentWithPublisherRow {
    pub id: i64,
    pub user_id: i64,
    pub post_id: i64,
    pub parent_id: Option<i64>,
    pub content: String,
    pub media: Value,
    pub like_count: i32,
    pub reply_count: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub user_uuid: Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub is_pro: i16,
    pub is_online: bool,
    pub my_reaction: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct CommentPublisher {
    pub id: i64,
    pub uuid: Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub is_online: bool,
    pub is_pro: i16,
}

#[derive(Debug, Serialize, Clone)]
struct CommentResponse {
    pub id: i64,
    pub post_id: i64,
    pub user_id: i64,
    pub content: String,
    pub media: Option<Value>,
    pub like_count: i32,
    pub my_reaction: Option<String>,
    pub replies: Vec<CommentResponse>,
    pub reply_count: i32,
    pub publisher: CommentPublisher,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

fn normalize_comment_media(media: Value) -> Option<Value> {
    if media.is_null() {
        return None;
    }

    if let Some(items) = media.as_array() {
        return items.iter().find(|item| !item.is_null()).cloned();
    }

    if media.is_object() {
        return Some(media);
    }

    None
}

fn map_comment_row(row: CommentWithPublisherRow, replies: Vec<CommentResponse>) -> CommentResponse {
    CommentResponse {
        id: row.id,
        post_id: row.post_id,
        user_id: row.user_id,
        content: row.content,
        media: normalize_comment_media(row.media),
        like_count: row.like_count,
        my_reaction: row.my_reaction,
        replies,
        reply_count: row.reply_count,
        publisher: CommentPublisher {
            id: row.user_id,
            uuid: row.user_uuid,
            username: row.username,
            first_name: row.first_name,
            last_name: row.last_name,
            avatar: row.avatar,
            is_verified: row.is_verified,
            is_online: row.is_online,
            is_pro: row.is_pro,
        },
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

async fn fetch_comment_rows_for_post(
    db: &PgPool,
    post_id: i64,
    viewer_id: Option<i64>,
    cursor_id: i64,
    limit: i64,
) -> Result<Vec<CommentWithPublisherRow>, ApiError> {
    let rows = sqlx::query_as::<_, CommentWithPublisherRow>(
        r#"SELECT c.id, c.user_id, c.post_id, c.parent_id, c.content, c.media, c.like_count,
                  c.reply_count, c.created_at, c.updated_at,
                  u.uuid AS user_uuid, u.username, u.first_name, u.last_name, u.avatar,
                  u.is_verified, u.is_pro, u.is_online,
                  (
                    SELECT r.reaction_type
                    FROM reactions r
                    WHERE r.target_type = 'comment'
                      AND r.target_id = c.id
                      AND r.user_id = $4
                    LIMIT 1
                  ) AS my_reaction
           FROM comments c
           JOIN users u ON u.id = c.user_id
           WHERE c.post_id = $1
             AND c.parent_id IS NULL
             AND c.id > $2
           ORDER BY c.created_at ASC
           LIMIT $3"#,
    )
    .bind(post_id)
    .bind(cursor_id)
    .bind(limit)
    .bind(viewer_id)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

async fn fetch_comment_rows_for_parent(
    db: &PgPool,
    parent_id: i64,
    viewer_id: Option<i64>,
    cursor_id: i64,
    limit: i64,
) -> Result<Vec<CommentWithPublisherRow>, ApiError> {
    let rows = sqlx::query_as::<_, CommentWithPublisherRow>(
        r#"SELECT c.id, c.user_id, c.post_id, c.parent_id, c.content, c.media, c.like_count,
                  c.reply_count, c.created_at, c.updated_at,
                  u.uuid AS user_uuid, u.username, u.first_name, u.last_name, u.avatar,
                  u.is_verified, u.is_pro, u.is_online,
                  (
                    SELECT r.reaction_type
                    FROM reactions r
                    WHERE r.target_type = 'comment'
                      AND r.target_id = c.id
                      AND r.user_id = $4
                    LIMIT 1
                  ) AS my_reaction
           FROM comments c
           JOIN users u ON u.id = c.user_id
           WHERE c.parent_id = $1
             AND c.id > $2
           ORDER BY c.created_at ASC
           LIMIT $3"#,
    )
    .bind(parent_id)
    .bind(cursor_id)
    .bind(limit)
    .bind(viewer_id)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

async fn fetch_replies_for_parent(
    db: &PgPool,
    parent_id: i64,
    viewer_id: Option<i64>,
) -> Result<Vec<CommentResponse>, ApiError> {
    let rows = fetch_comment_rows_for_parent(db, parent_id, viewer_id, 0, 50).await?;
    Ok(rows
        .into_iter()
        .map(|row| map_comment_row(row, Vec::new()))
        .collect())
}

async fn fetch_comment_response_by_id(
    db: &PgPool,
    comment_id: i64,
    viewer_id: Option<i64>,
    include_replies: bool,
) -> Result<Option<CommentResponse>, ApiError> {
    let row = sqlx::query_as::<_, CommentWithPublisherRow>(
        r#"SELECT c.id, c.user_id, c.post_id, c.parent_id, c.content, c.media, c.like_count,
                  c.reply_count, c.created_at, c.updated_at,
                  u.uuid AS user_uuid, u.username, u.first_name, u.last_name, u.avatar,
                  u.is_verified, u.is_pro, u.is_online,
                  (
                    SELECT r.reaction_type
                    FROM reactions r
                    WHERE r.target_type = 'comment'
                      AND r.target_id = c.id
                      AND r.user_id = $2
                    LIMIT 1
                  ) AS my_reaction
           FROM comments c
           JOIN users u ON u.id = c.user_id
           WHERE c.id = $1"#,
    )
    .bind(comment_id)
    .bind(viewer_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let replies = if include_replies && row.parent_id.is_none() {
        fetch_replies_for_parent(db, row.id, viewer_id).await?
    } else {
        Vec::new()
    };

    Ok(Some(map_comment_row(row, replies)))
}

pub async fn get_comments(
    State(state): State<AppState>,
    auth: OptionalAuth,
    Path(post_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(0);
    let viewer_id = auth.0.as_ref().map(|u| u.user_id);

    let comments =
        fetch_comment_rows_for_post(&state.db, post_id, viewer_id, cursor_id, limit + 1).await?;

    let has_more = comments.len() as i64 > limit;
    let taken: Vec<_> = comments.into_iter().take(limit as usize).collect();

    // Batch-fetch replies for all top-level comments (replaces N+1 queries)
    let parent_ids: Vec<i64> = taken.iter().map(|c| c.id).collect();
    let all_replies: Vec<CommentWithPublisherRow> = if parent_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, CommentWithPublisherRow>(
            r#"SELECT c.id, c.user_id, c.post_id, c.parent_id, c.content, c.media, c.like_count,
                      c.reply_count, c.created_at, c.updated_at,
                      u.uuid AS user_uuid, u.username, u.first_name, u.last_name, u.avatar,
                      u.is_verified, u.is_pro, u.is_online,
                      (
                        SELECT r.reaction_type
                        FROM reactions r
                        WHERE r.target_type = 'comment'
                          AND r.target_id = c.id
                          AND r.user_id = $2
                        LIMIT 1
                      ) AS my_reaction
               FROM comments c
               JOIN users u ON u.id = c.user_id
               WHERE c.parent_id = ANY($1)
               ORDER BY c.created_at ASC"#,
        )
        .bind(&parent_ids)
        .bind(viewer_id)
        .fetch_all(&state.db)
        .await?
    };

    // Group replies by parent_id
    let mut replies_map: std::collections::HashMap<i64, Vec<CommentResponse>> = std::collections::HashMap::new();
    for reply in all_replies {
        if let Some(pid) = reply.parent_id {
            replies_map.entry(pid).or_default().push(map_comment_row(reply, Vec::new()));
        }
    }

    let mut data = Vec::new();
    for row in taken {
        let replies = replies_map.remove(&row.id).unwrap_or_default();
        data.push(map_comment_row(row, replies));
    }
    let next_cursor = data.last().map(|c| c.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

pub async fn get_replies(
    State(state): State<AppState>,
    auth: OptionalAuth,
    Path(comment_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(0);
    let viewer_id = auth.0.as_ref().map(|u| u.user_id);

    let replies =
        fetch_comment_rows_for_parent(&state.db, comment_id, viewer_id, cursor_id, limit + 1)
            .await?;

    let has_more = replies.len() as i64 > limit;
    let data: Vec<_> = replies
        .into_iter()
        .take(limit as usize)
        .map(|row| map_comment_row(row, Vec::new()))
        .collect();
    let next_cursor = data.last().map(|c| c.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCommentRequest {
    #[validate(length(max = 10000))]
    pub content: Option<String>,
    pub media: Option<Value>,
    #[serde(alias = "reply_to")]
    pub parent_id: Option<i64>,
}

pub async fn create_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let content = req.content.unwrap_or_default();
    let trimmed_content = content.trim().to_string();
    let media = req.media.clone().unwrap_or(json!([]));

    if trimmed_content.is_empty() && normalize_comment_media(media.clone()).is_none() {
        return Err(ApiError::BadRequest(
            "Comment must include text or media".into(),
        ));
    }

    let comment = sqlx::query_as::<_, CommentRow>(
        r#"INSERT INTO comments (user_id, post_id, parent_id, content, media)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, user_id, post_id, parent_id, content, media, like_count, reply_count, created_at, updated_at"#,
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(req.parent_id)
    .bind(&trimmed_content)
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

    // Enqueue for AI moderation (non-blocking)
    if !trimmed_content.is_empty() {
        let db = state.db.clone();
        let user_id = auth.user_id;
        let comment_id = comment.id;
        let content_clone = trimmed_content.clone();
        tokio::spawn(async move {
            let _ = shared::moderation::enqueue_moderation(
                &db, "comment", comment_id, user_id, &content_clone,
            )
            .await;
        });
    }

    // Publish event for notifications
    let author_id: i64 = sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let _ = state
        .event_bus
        .publish(&DomainEvent::CommentCreated {
            comment_id: comment.id,
            post_id,
            user_id: auth.user_id,
            author_id,
        })
        .await;

    // If this is a reply (has a parent), also publish the granular event so the
    // realtime hub can fan-out only to the parent comment's author.
    if let Some(parent_id) = req.parent_id {
        let _ = state
            .event_bus
            .publish(&DomainEvent::CommentReplyCreated {
                parent_comment_id: parent_id,
                comment_id: comment.id,
                post_id,
            })
            .await;
    }

    let response = fetch_comment_response_by_id(&state.db, comment.id, Some(auth.user_id), false)
        .await?
        .ok_or(ApiError::NotFound("Comment not found".into()))?;

    Ok(Json(json!({ "data": response })))
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
           RETURNING id, user_id, post_id, parent_id, content, media, like_count, reply_count, created_at, updated_at"#,
    )
    .bind(comment_id)
    .bind(auth.user_id)
    .bind(&req.content)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Comment not found".into()))?;

    let response = fetch_comment_response_by_id(&state.db, comment.id, Some(auth.user_id), false)
        .await?
        .ok_or(ApiError::NotFound("Comment not found".into()))?;

    Ok(Json(json!({ "data": response })))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get post_id and parent_id before deleting
    #[derive(FromRow)]
    struct CommentMeta {
        post_id: i64,
        parent_id: Option<i64>,
    }

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

    Ok(Json(json!({ "data": { "message": "Comment deleted" } })))
}
