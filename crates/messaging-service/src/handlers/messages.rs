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
    metrics::CHAT_MESSAGES_SENT,
    pagination::PaginationParams,
    sanitize::sanitize_text,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: Option<String>,
    /// Frontend sends `type` for message_type.
    #[serde(alias = "type")]
    pub message_type: Option<String>,
    pub media: Option<Value>,
    /// Frontend sends `reply_to`.
    #[serde(alias = "reply_to")]
    pub reply_to_id: Option<i64>,
    /// Resolve `uploaded_media` row into `media` JSON (image / video / audio / file).
    pub media_id: Option<i64>,
    pub sticker_id: Option<i64>,
    pub gift_id: Option<i64>,
}

/// Forward a message to one or many conversations.
/// Accepts either `conversation_ids: [..]` (preferred, matches the frontend)
/// or `conversation_id: 123` (legacy single-target form).
#[derive(Debug, Deserialize)]
pub struct ForwardRequest {
    #[serde(default)]
    pub conversation_ids: Option<Vec<i64>>,
    #[serde(default)]
    pub conversation_id: Option<i64>,
}

impl ForwardRequest {
    /// Resolve the request into a deduplicated list of target conversation ids.
    fn targets(&self) -> Vec<i64> {
        let mut ids: Vec<i64> = self.conversation_ids.clone().unwrap_or_default();
        if let Some(single) = self.conversation_id {
            ids.push(single);
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }
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

// ─── Rate limiting (Redis) ───────────────────────────────────────────────────

async fn enforce_chat_send_rate_limits(
    state: &AppState,
    user_id: i64,
    conversation_id: i64,
) -> Result<(), ApiError> {
    const WINDOW_SECS: i64 = 60;
    const MAX_PER_USER: i64 = 45;
    const MAX_PER_CONV_USER: i64 = 25;
    let mut conn = state.redis.clone();
    let ku = format!("rl:chatsend:u:{user_id}");
    let n: i64 = redis::cmd("INCR")
        .arg(&ku)
        .query_async(&mut conn)
        .await
        .unwrap_or(0);
    if n == 1 {
        let _: Result<(), _> = redis::cmd("EXPIRE")
            .arg(&ku)
            .arg(WINDOW_SECS)
            .query_async(&mut conn)
            .await;
    }
    if n > MAX_PER_USER {
        return Err(ApiError::RateLimited);
    }
    let kc = format!("rl:chatsend:c:{user_id}:{conversation_id}");
    let c: i64 = redis::cmd("INCR")
        .arg(&kc)
        .query_async(&mut conn)
        .await
        .unwrap_or(0);
    if c == 1 {
        let _: Result<(), _> = redis::cmd("EXPIRE")
            .arg(&kc)
            .arg(WINDOW_SECS)
            .query_async(&mut conn)
            .await;
    }
    if c > MAX_PER_CONV_USER {
        return Err(ApiError::RateLimited);
    }
    Ok(())
}

fn count_url_delimiters(s: &str) -> usize {
    s.match_indices("http://").count() + s.match_indices("https://").count()
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
    enforce_chat_send_rate_limits(&state, auth.user_id, conversation_id).await?;
    verify_can_message_peer(&state, conversation_id, auth.user_id).await?;

    let spec_count = [
        req.media_id.is_some(),
        req.sticker_id.is_some(),
        req.gift_id.is_some(),
    ]
    .iter()
    .filter(|&&x| x)
    .count();
    if spec_count > 1 {
        return Err(ApiError::BadRequest(
            "Only one of media_id, sticker_id, gift_id is allowed".into(),
        ));
    }

    let mut content = req.content.unwrap_or_default();
    let mut msg_type = req.message_type.clone().unwrap_or_else(|| "text".to_string());
    let mut media = req.media.clone().unwrap_or(json!([]));

    if let Some(mid) = req.media_id {
        let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
            r#"SELECT file_url, file_type, COALESCE(file_name, ''), thumbnail_url
               FROM uploaded_media WHERE id = $1 AND user_id = $2"#,
        )
        .bind(mid)
        .bind(auth.user_id)
        .fetch_optional(&state.db)
        .await?;

        let (url, ftype, fname, thumb) =
            row.ok_or_else(|| ApiError::BadRequest("Invalid media_id".into()))?;

        if req.message_type.is_none() {
            msg_type = match ftype.as_str() {
                "video" => "video".into(),
                "audio" => "audio".into(),
                "file" => "file".into(),
                _ => "image".into(),
            };
        }

        let entry_type = match msg_type.as_str() {
            "video" => "video",
            "audio" => "audio",
            "file" => "file",
            _ => "image",
        };

        media = json!([{
            "id": mid,
            "url": url,
            "type": entry_type,
            "name": fname,
            "thumbnail": thumb.unwrap_or_default(),
        }]);

        if content.trim().is_empty() && ftype == "file" {
            content = fname;
        }
    }

    if let Some(sid) = req.sticker_id {
        msg_type = "sticker".into();
        let try_image =
            sqlx::query_scalar::<_, String>("SELECT image FROM stickers WHERE id = $1")
                .bind(sid)
                .fetch_optional(&state.db)
                .await;
        let url: String = match try_image {
            Ok(Some(s)) if !s.is_empty() => s,
            _ => sqlx::query_scalar::<_, String>(
                "SELECT image_url FROM stickers WHERE id = $1",
            )
            .bind(sid)
            .fetch_optional(&state.db)
            .await?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ApiError::BadRequest("Invalid sticker_id".into()))?,
        };

        media = json!([{ "url": url, "type": "image" }]);
    }

    if let Some(gid) = req.gift_id {
        msg_type = "gift".into();
        let row: Option<(String, String, f64)> = sqlx::query_as(
            "SELECT name, image, COALESCE(price, 0)::float8 FROM gifts WHERE id = $1",
        )
        .bind(gid)
        .fetch_optional(&state.db)
        .await?;

        let (name, gimg, price) = row.ok_or_else(|| ApiError::BadRequest("Invalid gift_id".into()))?;
        if content.trim().is_empty() {
            content = name.clone();
        }
        media = json!([{
            "url": gimg,
            "name": name,
            "type": "image",
            "gift_id": gid,
            "price": price,
            "currency": "USD",
        }]);
    }

    // Plain `media` JSON without media_id: ensure array is non-empty for non-text types.
    if req.media_id.is_none() && req.sticker_id.is_none() && req.gift_id.is_none() {
        let mt = msg_type.as_str();
        if matches!(mt, "image" | "video" | "audio" | "file") {
            if !media.as_array().is_some_and(|a| !a.is_empty()) {
                return Err(ApiError::BadRequest(
                    "media array or media_id required for this message type".into(),
                ));
            }
        }
        if mt == "text" && content.trim().is_empty() {
            return Err(ApiError::BadRequest("Text message content required".into()));
        }
    }

    content = sanitize_text(&content);
    const MAX_LINK_LIKE: usize = 28;
    if count_url_delimiters(&content) > MAX_LINK_LIKE {
        return Err(ApiError::BadRequest(
            "Message contains too many links".into(),
        ));
    }

    const MAX_CHAT_CONTENT: usize = 10_000;
    if content.chars().count() > MAX_CHAT_CONTENT {
        return Err(ApiError::BadRequest(format!(
            "Message content exceeds {} characters",
            MAX_CHAT_CONTENT
        )));
    }

    if content.trim().is_empty() && media.as_array().is_some_and(|a| a.is_empty()) {
        return Err(ApiError::BadRequest(
            "Message content or media required".into(),
        ));
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
            return Err(ApiError::BadRequest(
                "Reply target not found in this conversation".into(),
            ));
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
    .bind(&msg_type)
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

    let reply_to = if let Some(rid) = message.reply_to_id {
        sqlx::query_as::<_, ReplyPreview>(
            r#"
            SELECT m.id, m.sender_id, u.username AS sender_username, m.content, m.message_type
            FROM messages m
            JOIN users u ON u.id = m.sender_id
            WHERE m.id = $1 AND m.conversation_id = $2 AND m.deleted_at IS NULL
            "#,
        )
        .bind(rid)
        .bind(conversation_id)
        .fetch_optional(&state.db)
        .await?
    } else {
        None
    };

    let response = MessageResponse {
        message,
        reply_to,
    };

    tracing::info!(
        conversation_id,
        message_id = response.message.id,
        sender_id = auth.user_id,
        message_type = %response.message.message_type,
        "chat_message_sent"
    );
    CHAT_MESSAGES_SENT
        .with_label_values(&[response.message.message_type.as_str()])
        .inc();

    // Publish event for realtime-service WebSocket relay
    let recipient_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM conversation_members WHERE conversation_id = $1 AND user_id != $2 AND is_active = TRUE",
    )
    .bind(conversation_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let _ = state
        .event_bus
        .publish(&DomainEvent::MessageSent {
            conversation_id,
            sender_id: auth.user_id,
            recipient_ids,
        })
        .await;

    Ok(Json(json!({ "data": response })))
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
    let targets = req.targets();
    if targets.is_empty() {
        return Err(ApiError::BadRequest(
            "conversation_ids or conversation_id is required".into(),
        ));
    }
    // Reasonable cap to prevent abuse.
    if targets.len() > 50 {
        return Err(ApiError::BadRequest(
            "cannot forward to more than 50 conversations at once".into(),
        ));
    }

    // Verify sender has access to original message.
    let original = sqlx::query_as::<_, MessageRow>(
        "SELECT id, conversation_id, sender_id, content, message_type, media, reply_to_id, forwarded_from, is_pinned, is_favorited, deleted_at, created_at FROM messages WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Message not found".into()))?;

    verify_membership(&state, original.conversation_id, auth.user_id).await?;

    // Verify membership in every target conversation first — reject the whole
    // request if one of them is unauthorized, to avoid partial writes.
    for &target_id in &targets {
        verify_membership(&state, target_id, auth.user_id).await?;
        verify_can_message_peer(&state, target_id, auth.user_id).await?;
    }

    enforce_chat_send_rate_limits(&state, auth.user_id, original.conversation_id).await?;

    let safe_content = sanitize_text(&original.content);
    const MAX_LINK_LIKE: usize = 28;
    if count_url_delimiters(&safe_content) > MAX_LINK_LIKE {
        return Err(ApiError::BadRequest(
            "Forwarded text contains too many links".into(),
        ));
    }

    let mut new_ids: Vec<i64> = Vec::with_capacity(targets.len());
    let mut tx = state.db.begin().await?;

    for &target_id in &targets {
        let new_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO messages (conversation_id, sender_id, content, message_type, media, forwarded_from)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(target_id)
        .bind(auth.user_id)
        .bind(&safe_content)
        .bind(&original.message_type)
        .bind(&original.media)
        .bind(original.id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE conversations SET last_message_at = NOW() WHERE id = $1")
            .bind(target_id)
            .execute(&mut *tx)
            .await?;

        new_ids.push(new_id);
    }

    tx.commit().await?;

    // Return the hydrated copies in the same order the client asked for.
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
        WHERE m.id = ANY($1)
        ORDER BY m.id
        "#,
    )
    .bind(&new_ids)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "data": messages,
        "meta": { "forwarded_to": targets.len() }
    })))
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

    let _ = state
        .event_bus
        .publish(&DomainEvent::TypingStarted {
            conversation_id,
            user_id: auth.user_id,
        })
        .await;

    Ok(Json(json!({ "data": { "typing": true } })))
}

pub async fn stop_typing_indicator(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conversation_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, conversation_id, auth.user_id).await?;

    let key = format!("typing:{}:{}", conversation_id, auth.user_id);
    let _: Result<(), _> = redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut state.redis.clone())
        .await;

    let _ = state
        .event_bus
        .publish(&DomainEvent::TypingStopped {
            conversation_id,
            user_id: auth.user_id,
        })
        .await;

    Ok(Json(json!({ "data": { "typing": false } })))
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
    if reaction.len() > 32 {
        return Err(ApiError::BadRequest(
            "reaction too long (max 32 characters)".into(),
        ));
    }
    // Reject control and unassigned characters; reactions must be printable Unicode
    if reaction.contains(|c: char| c.is_control() || c == '\u{fffd}') {
        return Err(ApiError::BadRequest("invalid characters in reaction".into()));
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
            return Ok(Json(
                json!({ "data": { "action": "removed", "reaction": reaction } }),
            ));
        } else {
            // Different reaction — update
            sqlx::query("UPDATE reactions SET reaction_type = $3 WHERE user_id = $1 AND target_type = 'message' AND target_id = $2")
                .bind(auth.user_id)
                .bind(id)
                .bind(reaction)
                .execute(&state.db)
                .await?;
            return Ok(Json(
                json!({ "data": { "action": "updated", "reaction": reaction } }),
            ));
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

    Ok(Json(
        json!({ "data": { "action": "added", "reaction": reaction } }),
    ))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn verify_membership(
    state: &AppState,
    conversation_id: i64,
    user_id: i64,
) -> Result<(), ApiError> {
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

/// Block enforcement for 1:1 chats (groups are not blocked as a whole here).
async fn verify_can_message_peer(
    state: &AppState,
    conversation_id: i64,
    user_id: i64,
) -> Result<(), ApiError> {
    let ctype: String = sqlx::query_scalar(
        "SELECT type FROM conversations WHERE id = $1",
    )
    .bind(conversation_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

    if ctype != "direct" {
        return Ok(());
    }

    let peer: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT user_id FROM conversation_members
        WHERE conversation_id = $1 AND user_id <> $2 AND is_active = TRUE
        LIMIT 1
        "#,
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(pid) = peer {
        let blocked = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM blocks
                WHERE (blocker_id = $1 AND blocked_id = $2)
                   OR (blocker_id = $2 AND blocked_id = $1)
            )
            "#,
        )
        .bind(user_id)
        .bind(pid)
        .fetch_one(&state.db)
        .await?;

        if blocked {
            return Err(ApiError::Forbidden(
                "Messaging is not allowed with this user".into(),
            ));
        }
    }
    Ok(())
}

/// System / call-event row: persists, updates `last_message_at`, fans out `message.new`.
pub(in crate::handlers) async fn append_message_with_notification(
    state: &AppState,
    conversation_id: i64,
    sender_id: i64,
    content: String,
    message_type: String,
    media: Value,
) -> Result<(), ApiError> {
    verify_membership(state, conversation_id, sender_id).await?;

    let mut tx = state.db.begin().await?;
    let _msg_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO messages (conversation_id, sender_id, content, message_type, media, reply_to_id)
        VALUES ($1, $2, $3, $4, $5, NULL)
        RETURNING id
        "#,
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(&content)
    .bind(&message_type)
    .bind(&media)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE conversations SET last_message_at = NOW() WHERE id = $1")
        .bind(conversation_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let recipient_ids: Vec<i64> = sqlx::query_scalar(
        r#"
        SELECT user_id FROM conversation_members
        WHERE conversation_id = $1 AND user_id != $2 AND is_active = TRUE
        "#,
    )
    .bind(conversation_id)
    .bind(sender_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let _ = state
        .event_bus
        .publish(&DomainEvent::MessageSent {
            conversation_id,
            sender_id,
            recipient_ids,
        })
        .await;

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
        return Err(ApiError::Forbidden(
            "Not a member of this conversation".into(),
        ));
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

// ─── Listing endpoints (favorites / pinned) ─────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FavoritesQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub conversation_id: Option<i64>,
}

/// GET /v1/messages/favorites — list all messages I've marked as favorite
/// Optional `conversation_id` scopes the list to a single thread.
/// PHP parity: `api/get_fav_messages` (chat_id optional).
pub async fn list_favorite_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<FavoritesQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.pagination.limit();
    let cursor = params.pagination.cursor_id();

    if let Some(cid) = params.conversation_id {
        verify_membership(&state, cid, auth.user_id).await?;
    }

    let messages = sqlx::query_as::<_, MessageWithSender>(
        r#"
        SELECT
            m.id, m.conversation_id, m.sender_id,
            u.username  AS sender_username,
            u.first_name AS sender_first_name,
            u.last_name  AS sender_last_name,
            u.avatar     AS sender_avatar,
            m.content, m.message_type, m.media,
            m.reply_to_id, m.forwarded_from,
            m.is_pinned, m.is_favorited, m.created_at
        FROM messages m
        JOIN users u ON u.id = m.sender_id
        JOIN conversation_members cm
          ON cm.conversation_id = m.conversation_id
         AND cm.user_id = $1
         AND cm.is_active = TRUE
        WHERE m.is_favorited = TRUE
          AND m.deleted_at IS NULL
          AND ($2::bigint IS NULL OR m.conversation_id = $2)
          AND ($3::bigint IS NULL OR m.id < $3)
        ORDER BY m.id DESC
        LIMIT $4
        "#,
    )
    .bind(auth.user_id)
    .bind(params.conversation_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = messages.len() as i64 > limit;
    let messages: Vec<_> = messages.into_iter().take(limit as usize).collect();
    let next_cursor = messages.last().map(|m| m.id.to_string());

    Ok(Json(json!({
        "data": messages,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// GET /v1/conversations/{id}/pinned-messages — list all pinned messages in a thread
/// PHP parity: `api/get_pin_message`.
pub async fn list_pinned_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conversation_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, conversation_id, auth.user_id).await?;

    let limit = params.limit();
    let cursor = params.cursor_id();

    let messages = sqlx::query_as::<_, MessageWithSender>(
        r#"
        SELECT
            m.id, m.conversation_id, m.sender_id,
            u.username  AS sender_username,
            u.first_name AS sender_first_name,
            u.last_name  AS sender_last_name,
            u.avatar     AS sender_avatar,
            m.content, m.message_type, m.media,
            m.reply_to_id, m.forwarded_from,
            m.is_pinned, m.is_favorited, m.created_at
        FROM messages m
        JOIN users u ON u.id = m.sender_id
        WHERE m.conversation_id = $1
          AND m.is_pinned = TRUE
          AND m.deleted_at IS NULL
          AND ($2::bigint IS NULL OR m.id < $2)
        ORDER BY m.id DESC
        LIMIT $3
        "#,
    )
    .bind(conversation_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = messages.len() as i64 > limit;
    let messages: Vec<_> = messages.into_iter().take(limit as usize).collect();
    let next_cursor = messages.last().map(|m| m.id.to_string());

    Ok(Json(json!({
        "data": messages,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[cfg(test)]
mod chat_policy_tests {
    use super::count_url_delimiters;

    #[test]
    fn counts_distinct_url_occurrences() {
        assert_eq!(count_url_delimiters("hello https://a.com x http://b.org"), 2);
        assert_eq!(count_url_delimiters(""), 0);
    }
}
