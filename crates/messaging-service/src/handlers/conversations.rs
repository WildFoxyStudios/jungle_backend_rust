use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct CreateConversationRequest {
    pub recipient_id: i64,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateGroupRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub avatar: Option<String>,
    pub member_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateGroupRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateColorRequest {
    pub color: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ConversationRow {
    pub id: i64,
    pub r#type: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub color: Option<String>,
    pub last_message_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub id: i64,
    pub r#type: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub color: Option<String>,
    pub last_message_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub members: Vec<MemberInfo>,
    pub last_message: Option<LastMessageInfo>,
    pub unread_count: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MemberInfo {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_online: bool,
    pub role: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LastMessageInfo {
    pub id: i64,
    pub sender_id: i64,
    pub content: String,
    pub message_type: String,
    pub created_at: OffsetDateTime,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn list_conversations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let fetch_limit = limit + 1;

    let rows = sqlx::query_as::<_, ConversationRow>(
        r#"
        SELECT c.id, c.type, c.name, c.avatar, c.color, c.last_message_at, c.created_at
        FROM conversations c
        JOIN conversation_members cm ON cm.conversation_id = c.id
        WHERE cm.user_id = $1 AND cm.is_active = TRUE AND cm.archived = FALSE
          AND ($2::bigint IS NULL OR c.id < $2)
        ORDER BY c.last_message_at DESC NULLS LAST
        LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(fetch_limit)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.id.to_string());

    let mut conversations = Vec::with_capacity(rows.len());
    for row in rows {
        let conv = enrich_conversation(&state, row, auth.user_id).await?;
        conversations.push(conv);
    }

    Ok(Json(json!({
        "data": conversations,
        "meta": {
            "cursor": next_cursor,
            "has_more": has_more
        }
    })))
}

pub async fn list_pinned(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, ConversationRow>(
        r#"
        SELECT c.id, c.type, c.name, c.avatar, c.color, c.last_message_at, c.created_at
        FROM conversations c
        JOIN conversation_members cm ON cm.conversation_id = c.id
        WHERE cm.user_id = $1 AND cm.is_active = TRUE AND cm.pinned = TRUE
        ORDER BY c.last_message_at DESC NULLS LAST
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mut conversations = Vec::with_capacity(rows.len());
    for row in rows {
        let conv = enrich_conversation(&state, row, auth.user_id).await?;
        conversations.push(conv);
    }

    Ok(Json(json!({ "data": conversations })))
}

pub async fn list_archived(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let fetch_limit = limit + 1;

    let rows = sqlx::query_as::<_, ConversationRow>(
        r#"
        SELECT c.id, c.type, c.name, c.avatar, c.color, c.last_message_at, c.created_at
        FROM conversations c
        JOIN conversation_members cm ON cm.conversation_id = c.id
        WHERE cm.user_id = $1 AND cm.is_active = TRUE AND cm.archived = TRUE
          AND ($2::bigint IS NULL OR c.id < $2)
        ORDER BY c.last_message_at DESC NULLS LAST
        LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(fetch_limit)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.id.to_string());

    let mut conversations = Vec::with_capacity(rows.len());
    for row in rows {
        let conv = enrich_conversation(&state, row, auth.user_id).await?;
        conversations.push(conv);
    }

    Ok(Json(json!({
        "data": conversations,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

pub async fn get_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, id, auth.user_id).await?;

    let row = sqlx::query_as::<_, ConversationRow>(
        "SELECT id, type, name, avatar, color, last_message_at, created_at FROM conversations WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

    let conv = enrich_conversation(&state, row, auth.user_id).await?;
    Ok(Json(json!({ "data": conv })))
}

pub async fn create_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.recipient_id == auth.user_id {
        return Err(ApiError::BadRequest("Cannot message yourself".into()));
    }

    // Check if blocked
    let blocked = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE (blocker_id = $1 AND blocked_id = $2) OR (blocker_id = $2 AND blocked_id = $1))",
    )
    .bind(auth.user_id)
    .bind(req.recipient_id)
    .fetch_one(&state.db)
    .await?;

    if blocked {
        return Err(ApiError::Forbidden("".into()));
    }

    // Check if direct conversation already exists between these two users
    let existing = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT c.id FROM conversations c
        WHERE c.type = 'direct'
          AND EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $1 AND is_active = TRUE)
          AND EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $2 AND is_active = TRUE)
        LIMIT 1
        "#,
    )
    .bind(auth.user_id)
    .bind(req.recipient_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(conv_id) = existing {
        // Reactivate if needed
        sqlx::query("UPDATE conversation_members SET is_active = TRUE, archived = FALSE WHERE conversation_id = $1 AND user_id = $2")
            .bind(conv_id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;

        let row = sqlx::query_as::<_, ConversationRow>(
            "SELECT id, type, name, avatar, color, last_message_at, created_at FROM conversations WHERE id = $1",
        )
        .bind(conv_id)
        .fetch_one(&state.db)
        .await?;

        let conv = enrich_conversation(&state, row, auth.user_id).await?;
        return Ok(Json(json!({ "data": conv })));
    }

    // Create new direct conversation
    let mut tx = state.db.begin().await?;

    let conv_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO conversations (type) VALUES ('direct') RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO conversation_members (conversation_id, user_id, role) VALUES ($1, $2, 'owner'), ($1, $3, 'member')",
    )
    .bind(conv_id)
    .bind(auth.user_id)
    .bind(req.recipient_id)
    .execute(&mut *tx)
    .await?;

    // Optionally send the first message
    if let Some(content) = &req.message
        && !content.trim().is_empty() {
            sqlx::query(
                "INSERT INTO messages (conversation_id, sender_id, content, message_type) VALUES ($1, $2, $3, 'text')",
            )
            .bind(conv_id)
            .bind(auth.user_id)
            .bind(content)
            .execute(&mut *tx)
            .await?;

            sqlx::query("UPDATE conversations SET last_message_at = NOW() WHERE id = $1")
                .bind(conv_id)
                .execute(&mut *tx)
                .await?;
        }

    tx.commit().await?;

    let row = sqlx::query_as::<_, ConversationRow>(
        "SELECT id, type, name, avatar, color, last_message_at, created_at FROM conversations WHERE id = $1",
    )
    .bind(conv_id)
    .fetch_one(&state.db)
    .await?;

    let conv = enrich_conversation(&state, row, auth.user_id).await?;
    Ok(Json(json!({ "data": conv })))
}

pub async fn create_group_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    if req.member_ids.is_empty() {
        return Err(ApiError::BadRequest("Group must have at least one member".into()));
    }

    let mut tx = state.db.begin().await?;

    let conv_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO conversations (type, name, avatar, creator_id) VALUES ('group', $1, $2, $3) RETURNING id",
    )
    .bind(&req.name)
    .bind(&req.avatar)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;

    // Add creator as owner
    sqlx::query(
        "INSERT INTO conversation_members (conversation_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(conv_id)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await?;

    // Add members
    for member_id in &req.member_ids {
        if *member_id == auth.user_id {
            continue;
        }
        sqlx::query(
            "INSERT INTO conversation_members (conversation_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING",
        )
        .bind(conv_id)
        .bind(member_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let row = sqlx::query_as::<_, ConversationRow>(
        "SELECT id, type, name, avatar, color, last_message_at, created_at FROM conversations WHERE id = $1",
    )
    .bind(conv_id)
    .fetch_one(&state.db)
    .await?;

    let conv = enrich_conversation(&state, row, auth.user_id).await?;
    Ok(Json(json!({ "data": conv })))
}

pub async fn update_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state, id, auth.user_id).await?;

    let mut sets = Vec::new();
    let mut param_idx = 2u32;
    let mut query_str = String::from("UPDATE conversations SET ");

    if req.name.is_some() {
        sets.push(format!("name = ${}", param_idx));
        param_idx += 1;
    }
    if req.avatar.is_some() {
        sets.push(format!("avatar = ${}", param_idx));
    }

    if sets.is_empty() {
        return Err(ApiError::BadRequest("No fields to update".into()));
    }

    query_str.push_str(&sets.join(", "));
    query_str.push_str(" WHERE id = $1 AND type = 'group'");

    let mut q = sqlx::query(&query_str).bind(id);
    if let Some(ref name) = req.name {
        q = q.bind(name);
    }
    if let Some(ref avatar) = req.avatar {
        q = q.bind(avatar);
    }
    q.execute(&state.db).await?;

    Ok(Json(json!({ "data": { "message": "Group updated" } })))
}

pub async fn delete_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, id, auth.user_id).await?;

    // Soft-delete: mark member as inactive (user can't see it anymore)
    sqlx::query("UPDATE conversation_members SET is_active = FALSE WHERE conversation_id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "message": "Conversation deleted" } })))
}

pub async fn update_color(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateColorRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, id, auth.user_id).await?;

    sqlx::query("UPDATE conversations SET color = $1 WHERE id = $2")
        .bind(&req.color)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "message": "Color updated" } })))
}

pub async fn pin_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE conversation_members SET pinned = TRUE WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "pinned": true } })))
}

pub async fn unpin_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE conversation_members SET pinned = FALSE WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "pinned": false } })))
}

pub async fn archive_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE conversation_members SET archived = TRUE WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "archived": true } })))
}

pub async fn unarchive_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE conversation_members SET archived = FALSE WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "archived": false } })))
}

pub async fn mark_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE conversation_members SET last_read_at = NOW() WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "read": true } })))
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

async fn verify_group_admin(state: &AppState, conversation_id: i64, user_id: i64) -> Result<(), ApiError> {
    let role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM conversation_members WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Forbidden("".into()))?;

    if role != "owner" && role != "admin" {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

async fn enrich_conversation(
    state: &AppState,
    row: ConversationRow,
    viewer_id: i64,
) -> Result<ConversationResponse, ApiError> {
    let members = sqlx::query_as::<_, MemberInfo>(
        r#"
        SELECT cm.user_id, u.username, u.first_name, u.last_name, u.avatar, u.is_online, cm.role
        FROM conversation_members cm
        JOIN users u ON u.id = cm.user_id
        WHERE cm.conversation_id = $1 AND cm.is_active = TRUE
        "#,
    )
    .bind(row.id)
    .fetch_all(&state.db)
    .await?;

    let last_message = sqlx::query_as::<_, LastMessageInfo>(
        "SELECT id, sender_id, content, message_type, created_at FROM messages WHERE conversation_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
    )
    .bind(row.id)
    .fetch_optional(&state.db)
    .await?;

    let unread_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM messages m
        WHERE m.conversation_id = $1 AND m.deleted_at IS NULL AND m.sender_id != $2
          AND m.created_at > (
            SELECT COALESCE(last_read_at, '1970-01-01') FROM conversation_members
            WHERE conversation_id = $1 AND user_id = $2
          )
        "#,
    )
    .bind(row.id)
    .bind(viewer_id)
    .fetch_one(&state.db)
    .await?;

    Ok(ConversationResponse {
        id: row.id,
        r#type: row.r#type,
        name: row.name,
        avatar: row.avatar,
        color: row.color,
        last_message_at: row.last_message_at,
        created_at: row.created_at,
        members,
        last_message,
        unread_count,
    })
}

/// POST /v1/conversations/mark-all-read — Mark all conversations as read (PHP: mark_as_read.php)
pub async fn mark_all_read(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        "UPDATE conversation_members SET last_read_at = NOW()
         WHERE user_id = $1 AND is_active = TRUE",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "marked_read": true } })))
}

// ─── Conversation-scoped search & media tab ──────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConversationSearchQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub q: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ConversationMessageSearchRow {
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
    pub created_at: OffsetDateTime,
}

/// GET /v1/conversations/{id}/search?q=... — Full-text search scoped to this thread.
/// PHP parity: `api/chat` with `type=search`.
pub async fn search_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conversation_id): Path<i64>,
    Query(params): Query<ConversationSearchQuery>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, conversation_id, auth.user_id).await?;

    let query = params.q.trim();
    if query.is_empty() {
        return Err(ApiError::BadRequest("q parameter is required".into()));
    }
    // Cap the pattern length to avoid pathological LIKE patterns.
    let query = if query.len() > 200 { &query[..200] } else { query };

    let like_pattern = format!("%{}%", escape_like(query));
    let limit = params.pagination.limit();
    let cursor = params.pagination.cursor_id();

    let messages = sqlx::query_as::<_, ConversationMessageSearchRow>(
        r#"
        SELECT
            m.id, m.conversation_id, m.sender_id,
            u.username   AS sender_username,
            u.first_name AS sender_first_name,
            u.last_name  AS sender_last_name,
            u.avatar     AS sender_avatar,
            m.content, m.message_type, m.media, m.created_at
        FROM messages m
        JOIN users u ON u.id = m.sender_id
        WHERE m.conversation_id = $1
          AND m.deleted_at IS NULL
          AND m.content ILIKE $2
          AND ($3::bigint IS NULL OR m.id < $3)
        ORDER BY m.id DESC
        LIMIT $4
        "#,
    )
    .bind(conversation_id)
    .bind(&like_pattern)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = messages.len() as i64 > limit;
    let messages: Vec<_> = messages.into_iter().take(limit as usize).collect();
    let next_cursor = messages.last().map(|m| m.id.to_string());

    Ok(Json(json!({
        "data": messages,
        "meta": { "cursor": next_cursor, "has_more": has_more, "q": params.q }
    })))
}

#[derive(Debug, Deserialize)]
pub struct ConversationMediaQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    /// Optional filter: image | video | audio | file
    pub r#type: Option<String>,
}

/// GET /v1/conversations/{id}/media — Photos / files / videos shared inside this chat.
/// PHP parity: `api/chat` with `type=get_media`.
pub async fn list_conversation_media(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conversation_id): Path<i64>,
    Query(params): Query<ConversationMediaQuery>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, conversation_id, auth.user_id).await?;

    let limit = params.pagination.limit();
    let cursor = params.pagination.cursor_id();
    let kind = params.r#type.as_deref().map(str::trim).filter(|s| !s.is_empty());

    // Accept: image | video | audio | file.  "file" maps to `message_type = 'file'`
    // (generic attachment).  Anything else is rejected.
    let allowed = ["image", "video", "audio", "file"];
    if let Some(k) = kind {
        if !allowed.contains(&k) {
            return Err(ApiError::BadRequest(format!(
                "type must be one of {}",
                allowed.join(", ")
            )));
        }
    }

    let messages = sqlx::query_as::<_, MessageMediaRow>(
        r#"
        SELECT m.id, m.sender_id, u.username AS sender_username, u.avatar AS sender_avatar,
               m.message_type, m.media, m.created_at
          FROM messages m
          JOIN users u ON u.id = m.sender_id
         WHERE m.conversation_id = $1
           AND m.deleted_at IS NULL
           AND (
                m.message_type IN ('image', 'video', 'audio', 'file')
                OR jsonb_array_length(COALESCE(m.media, '[]'::jsonb)) > 0
               )
           AND ($2::text IS NULL OR m.message_type = $2)
           AND ($3::bigint IS NULL OR m.id < $3)
         ORDER BY m.id DESC
         LIMIT $4
        "#,
    )
    .bind(conversation_id)
    .bind(kind)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = messages.len() as i64 > limit;
    let messages: Vec<_> = messages.into_iter().take(limit as usize).collect();
    let next_cursor = messages.last().map(|m| m.id.to_string());

    Ok(Json(json!({
        "data": messages,
        "meta": { "cursor": next_cursor, "has_more": has_more, "type": params.r#type }
    })))
}

#[derive(Debug, Serialize, FromRow)]
pub struct MessageMediaRow {
    pub id: i64,
    pub sender_id: i64,
    pub sender_username: String,
    pub sender_avatar: String,
    pub message_type: String,
    pub media: Value,
    pub created_at: OffsetDateTime,
}

/// Escape `%` and `_` so a user-supplied query is matched literally by ILIKE.
fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}
