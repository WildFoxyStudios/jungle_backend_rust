use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct SendMessageRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: Option<String>,
    pub message_type: Option<String>,
    pub media: Option<Value>,
    pub reply_to_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ForwardRequest {
    pub conversation_id: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MessageRow {
    pub id: i64,
    pub conversation_id: i64,
    pub sender_id: i64,
    pub content: String,
    pub message_type: String,
    pub media: Value,
    pub reply_to_id: Option<i64>,
    pub forwarded_from: Option<i64>,
    pub is_pinned: bool,
    pub is_favorited: bool,
    pub deleted_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MessageWithSender {
    pub id: i64,
    pub conversation_id: i64,
    pub sender_id: i64,
    pub sender_username: String,
    pub sender_first_name: String,
    pub sender_last_name: String,
    pub sender_avatar: String,
    pub content: String,
    pub message_type: String,
    pub media: Value,
    pub reply_to_id: Option<i64>,
    pub forwarded_from: Option<i64>,
    pub is_pinned: bool,
    pub is_favorited: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: MessageWithSender,
    pub reply_to: Option<ReplyPreview>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ReplyPreview {
    pub id: i64,
    pub sender_id: i64,
    pub sender_username: String,
    pub content: String,
    pub message_type: String,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn list_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conversation_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, conversation_id, auth.user_id).await?;

    let limit = params.limit();
    let cursor = params.cursor_id();
    let fetch_limit = limit + 1;

    let messages = sqlx::query_as::<_, MessageWithSender>(
        r#"
        SELECT
            m.id, m.conversation_id, m.sender_id,
            u.username AS sender_username,
            u.first_name AS sender_first_name,
            u.last_name AS sender_last_name,
            u.avatar AS sender_avatar,
            m.content, m.message_type, m.media,
            m.reply_to_id, m.forwarded_from,
            m.is_pinned, m.is_favorited, m.created_at
        FROM messages m
        JOIN users u ON u.id = m.sender_id
        WHERE m.conversation_id = $1 AND m.deleted_at IS NULL
          AND ($2::bigint IS NULL OR m.id < $2)
        ORDER BY m.id DESC
        LIMIT $3
        "#,
    )
    .bind(conversation_id)
    .bind(cursor)
    .bind(fetch_limit)
    .fetch_all(&state.db)
    .await?;

    let has_more = messages.len() as i64 > limit;
    let messages: Vec<_> = messages.into_iter().take(limit as usize).collect();
    let next_cursor = messages.last().map(|m| m.id.to_string());

    // Batch-fetch reply previews
    let reply_ids: Vec<i64> = messages.iter().filter_map(|m| m.reply_to_id).collect();
    let replies = if !reply_ids.is_empty() {
        sqlx::query_as::<_, ReplyPreview>(
            r#"
            SELECT m.id, m.sender_id, u.username AS sender_username, m.content, m.message_type
            FROM messages m
            JOIN users u ON u.id = m.sender_id
            WHERE m.id = ANY($1::bigint[])
            "#,
        )
        .bind(&reply_ids)
        .fetch_all(&state.db)
        .await?
    } else {
        vec![]
    };

    let response: Vec<MessageResponse> = messages
        .into_iter()
        .map(|msg| {
            let reply_to = msg
                .reply_to_id
                .and_then(|rid| replies.iter().find(|r| r.id == rid))
                .map(|r| ReplyPreview {
                    id: r.id,
                    sender_id: r.sender_id,
                    sender_username: r.sender_username.clone(),
                    content: r.content.clone(),
                    message_type: r.message_type.clone(),
                });
            MessageResponse {
                message: msg,
                reply_to,
            }
        })
        .collect();

    Ok(Json(json!({
        "data": response,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conversation_id): Path<i64>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, conversation_id, auth.user_id).await?;

    let content = req.content.unwrap_or_default();
    let msg_type = req.message_type.as_deref().unwrap_or("text");
    let media = req.media.unwrap_or(json!([]));

    if content.trim().is_empty() && media.as_array().is_some_and(|a| a.is_empty()) {
        return Err(ApiError::BadRequest("Message content or media required".into()));
    }

    // Validate reply_to exists in same conversation
    if let Some(reply_id) = req.reply_to_id {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE id = $1 AND conversation_id = $2 AND deleted_at IS NULL)",
        )
        .bind(reply_id)
        .bind(conversation_id)
        .fetch_one(&state.db)
        .await?;

        if !exists {
            return Err(ApiError::BadRequest("Reply target not found in this conversation".into()));
        }
    }

    let mut tx = state.db.begin().await?;

    let msg_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO messages (conversation_id, sender_id, content, message_type, media, reply_to_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(conversation_id)
    .bind(auth.user_id)
    .bind(&content)
    .bind(msg_type)
    .bind(&media)
    .bind(req.reply_to_id)
    .fetch_one(&mut *tx)
    .await?;

    // Update conversation last_message_at
    sqlx::query("UPDATE conversations SET last_message_at = NOW() WHERE id = $1")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Fetch the full message with sender info
    let message = sqlx::query_as::<_, MessageWithSender>(
        r#"
        SELECT
            m.id, m.conversation_id, m.sender_id,
            u.username AS sender_username,
            u.first_name AS sender_first_name,
            u.last_name AS sender_last_name,
            u.avatar AS sender_avatar,
            m.content, m.message_type, m.media,
            m.reply_to_id, m.forwarded_from,
            m.is_pinned, m.is_favorited, m.created_at
        FROM messages m
        JOIN users u ON u.id = m.sender_id
        WHERE m.id = $1
        "#,
    )
    .bind(msg_id)
    .fetch_one(&state.db)
    .await?;

    // Publish event for realtime-service WebSocket relay
    let recipient_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM conversation_members WHERE conversation_id = $1 AND user_id != $2 AND is_active = TRUE",
    )
    .bind(conversation_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let _ = state.event_bus.publish(&DomainEvent::MessageSent {
        conversation_id,
        sender_id: auth.user_id,
        recipient_ids,
    }).await;

    Ok(Json(json!({ "data": message })))
}

pub async fn delete_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let msg = sqlx::query_as::<_, MessageRow>(
        "SELECT id, conversation_id, sender_id, content, message_type, media, reply_to_id, forwarded_from, is_pinned, is_favorited, deleted_at, created_at FROM messages WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Message not found".into()))?;

    if msg.sender_id != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("UPDATE messages SET deleted_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn toggle_favorite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let msg = sqlx::query_as::<_, MessageRow>(
        "SELECT id, conversation_id, sender_id, content, message_type, media, reply_to_id, forwarded_from, is_pinned, is_favorited, deleted_at, created_at FROM messages WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Message not found".into()))?;

    verify_membership(&state, msg.conversation_id, auth.user_id).await?;

    let new_val = !msg.is_favorited;
    sqlx::query("UPDATE messages SET is_favorited = $1 WHERE id = $2")
        .bind(new_val)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "is_favorited": new_val } })))
}

pub async fn pin_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conv_id = get_message_conversation(&state, id).await?;
    verify_membership(&state, conv_id, auth.user_id).await?;

    sqlx::query("UPDATE messages SET is_pinned = TRUE WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "pinned": true } })))
}

pub async fn unpin_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conv_id = get_message_conversation(&state, id).await?;
    verify_membership(&state, conv_id, auth.user_id).await?;

    sqlx::query("UPDATE messages SET is_pinned = FALSE WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "pinned": false } })))
}

pub async fn forward_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ForwardRequest>,
) -> Result<Json<Value>, ApiError> {
    // Verify sender has access to original message
    let original = sqlx::query_as::<_, MessageRow>(
        "SELECT id, conversation_id, sender_id, content, message_type, media, reply_to_id, forwarded_from, is_pinned, is_favorited, deleted_at, created_at FROM messages WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Message not found".into()))?;

    verify_membership(&state, original.conversation_id, auth.user_id).await?;
    verify_membership(&state, req.conversation_id, auth.user_id).await?;

    let mut tx = state.db.begin().await?;

    let new_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO messages (conversation_id, sender_id, content, message_type, media, forwarded_from)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(req.conversation_id)
    .bind(auth.user_id)
    .bind(&original.content)
    .bind(&original.message_type)
    .bind(&original.media)
    .bind(original.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE conversations SET last_message_at = NOW() WHERE id = $1")
        .bind(req.conversation_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let message = sqlx::query_as::<_, MessageWithSender>(
        r#"
        SELECT
            m.id, m.conversation_id, m.sender_id,
            u.username AS sender_username,
            u.first_name AS sender_first_name,
            u.last_name AS sender_last_name,
            u.avatar AS sender_avatar,
            m.content, m.message_type, m.media,
            m.reply_to_id, m.forwarded_from,
            m.is_pinned, m.is_favorited, m.created_at
        FROM messages m
        JOIN users u ON u.id = m.sender_id
        WHERE m.id = $1
        "#,
    )
    .bind(new_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": message })))
}

pub async fn typing_indicator(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conversation_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, conversation_id, auth.user_id).await?;

    // Store typing status in Redis with short TTL (3 seconds)
    let key = format!("typing:{}:{}", conversation_id, auth.user_id);
    redis::cmd("SETEX")
        .arg(&key)
        .arg(3i64)
        .arg("1")
        .query_async::<String>(&mut state.redis.clone())
        .await
        .ok();

    let _ = state.event_bus.publish(&DomainEvent::TypingStarted {
        conversation_id,
        user_id: auth.user_id,
    }).await;

    Ok(Json(json!({ "data": { "typing": true } })))
}

// ─── Message Reactions ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReactMessageRequest {
    pub reaction: String,
}

/// POST /v1/messages/{id}/react — react to a message (toggle)
pub async fn react_to_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReactMessageRequest>,
) -> Result<Json<Value>, ApiError> {
    let conv_id = get_message_conversation(&state, id).await?;
    verify_membership(&state, conv_id, auth.user_id).await?;

    let reaction = req.reaction.trim();
    if reaction.is_empty() {
        return Err(ApiError::BadRequest("reaction cannot be empty".into()));
    }

    // Check if user already reacted to this message
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT reaction_type FROM reactions WHERE user_id = $1 AND target_type = 'message' AND target_id = $2",
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(existing_reaction) = existing {
        if existing_reaction == reaction {
            // Same reaction — remove it (toggle off)
            sqlx::query("DELETE FROM reactions WHERE user_id = $1 AND target_type = 'message' AND target_id = $2")
                .bind(auth.user_id)
                .bind(id)
                .execute(&state.db)
                .await?;
            return Ok(Json(json!({ "data": { "action": "removed", "reaction": reaction } })));
        } else {
            // Different reaction — update
            sqlx::query("UPDATE reactions SET reaction_type = $3 WHERE user_id = $1 AND target_type = 'message' AND target_id = $2")
                .bind(auth.user_id)
                .bind(id)
                .bind(reaction)
                .execute(&state.db)
                .await?;
            return Ok(Json(json!({ "data": { "action": "updated", "reaction": reaction } })));
        }
    }

    // New reaction
    sqlx::query(
        "INSERT INTO reactions (user_id, target_type, target_id, reaction_type) VALUES ($1, 'message', $2, $3)",
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(reaction)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "action": "added", "reaction": reaction } })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn verify_membership(state: &AppState, conversation_id: i64, user_id: i64) -> Result<(), ApiError> {
    let is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE)",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

/// POST /v1/messages/{id}/listened — Mark voice/audio message as listened (PHP: listening.php)
pub async fn mark_listened(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Verify the user is a member of the conversation
    let conv_id = get_message_conversation(&state, id).await?;

    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE)",
    )
    .bind(conv_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(ApiError::Forbidden("Not a member of this conversation".into()));
    }

    // Mark the audio message as listened using metadata JSONB
    sqlx::query(
        "UPDATE messages SET media = jsonb_set(COALESCE(media, '[]'::jsonb), '{0,listened}', 'true', true) WHERE id = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "listened": true } })))
}

async fn get_message_conversation(state: &AppState, message_id: i64) -> Result<i64, ApiError> {
    sqlx::query_scalar::<_, i64>(
        "SELECT conversation_id FROM messages WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(message_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Message not found".into()))
}
