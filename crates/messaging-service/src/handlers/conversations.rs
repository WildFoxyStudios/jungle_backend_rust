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
    pagination::PaginationParams,
};
use sqlx::{FromRow, Row};
use time::OffsetDateTime;
use validator::Validate;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct CreateConversationRequest {
    /// Accepts either `recipient_id` (preferred) or `user_id` (frontend legacy
    /// form). `serde(alias)` makes both deserialize into the same field.
    #[serde(alias = "user_id")]
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

#[derive(Debug, Deserialize)]
pub struct MuteRequest {
    /// If provided, mute only until this moment; otherwise mute indefinitely.
    #[serde(default)]
    pub until: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWallpaperRequest {
    pub wallpaper_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDestructRequest {
    /// Seconds after which each message auto-expires. NULL disables the feature.
    pub destruct_after_seconds: Option<i32>,
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
    pub destruct_after_seconds: Option<i32>,
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
    /// Disappearing-messages lifetime in seconds (NULL = disabled).
    pub destruct_after_seconds: Option<i32>,
    /// Viewer-scoped flags from conversation_members.
    pub muted: bool,
    pub muted_until: Option<OffsetDateTime>,
    pub wallpaper_url: Option<String>,
    pub pinned: bool,
    pub archived: bool,
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
        SELECT c.id, c.type, c.name, c.avatar, c.color, c.last_message_at, c.created_at,
               c.destruct_after_seconds
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
        SELECT c.id, c.type, c.name, c.avatar, c.color, c.last_message_at, c.created_at,
               c.destruct_after_seconds
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
        SELECT c.id, c.type, c.name, c.avatar, c.color, c.last_message_at, c.created_at,
               c.destruct_after_seconds
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
        "SELECT id, type, name, avatar, color, last_message_at, created_at, destruct_after_seconds FROM conversations WHERE id = $1",
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
            "SELECT id, type, name, avatar, color, last_message_at, created_at, destruct_after_seconds FROM conversations WHERE id = $1",
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
        && !content.trim().is_empty()
    {
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
        "SELECT id, type, name, avatar, color, last_message_at, created_at, destruct_after_seconds FROM conversations WHERE id = $1",
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
        return Err(ApiError::BadRequest(
            "Group must have at least one member".into(),
        ));
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
        "SELECT id, type, name, avatar, color, last_message_at, created_at, destruct_after_seconds FROM conversations WHERE id = $1",
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

    Ok(Json(
        json!({ "data": { "message": "Conversation deleted" } }),
    ))
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

    if let Err(e) = state
        .event_bus
        .publish(&DomainEvent::ChatColorChanged {
            conversation_id: id,
            user_id: auth.user_id,
            color: req.color.clone(),
        })
        .await
    {
        tracing::warn!(conversation_id = id, error = %e, "failed to publish ChatColorChanged");
    }

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

/// POST /v1/conversations/{id}/mute — Mute notifications for this conversation.
/// Accepts optional `until` timestamp to mute temporarily.
pub async fn mute_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<MuteRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, id, auth.user_id).await?;

    sqlx::query(
        r#"
        UPDATE conversation_members
        SET muted = TRUE, muted_until = $3
        WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(req.until)
    .execute(&state.db)
    .await?;

    Ok(Json(
        json!({ "data": { "muted": true, "muted_until": req.until } }),
    ))
}

/// DELETE /v1/conversations/{id}/mute — Un-mute.
pub async fn unmute_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, id, auth.user_id).await?;

    sqlx::query(
        r#"
        UPDATE conversation_members
        SET muted = FALSE, muted_until = NULL
        WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "muted": false } })))
}

/// PUT /v1/conversations/{id}/wallpaper — Per-user chat background override.
/// Send `{ "wallpaper_url": null }` to reset.
pub async fn update_wallpaper(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateWallpaperRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, id, auth.user_id).await?;

    // URL sanity: http(s), data:image, or same-site path (e.g. /wallpapers/... from Next public/).
    if let Some(ref url) = req.wallpaper_url {
        let ok = url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("data:image/")
            || (url.starts_with('/')
                && !url.contains("..")
                && url
                    .chars()
                    .all(|c| !c.is_control() && c != '\\'));
        if !ok {
            return Err(ApiError::BadRequest(
                "wallpaper_url must be http(s), data:image, or a safe site path starting with /"
                    .into(),
            ));
        }
        if url.len() > 2048 {
            return Err(ApiError::BadRequest("wallpaper_url too long".into()));
        }
    }

    sqlx::query(
        r#"
        UPDATE conversation_members
        SET wallpaper_url = $3
        WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(req.wallpaper_url.as_deref())
    .execute(&state.db)
    .await?;

    Ok(Json(
        json!({ "data": { "wallpaper_url": req.wallpaper_url } }),
    ))
}

/// PUT /v1/conversations/{id}/destruct — Configure disappearing messages.
/// `destruct_after_seconds: null` disables the feature.
/// Sensible bounds: 60 seconds — 30 days.
pub async fn update_destruct(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateDestructRequest>,
) -> Result<Json<Value>, ApiError> {
    // Only group admins or any member in a direct chat may change this.
    let conv_type = sqlx::query_scalar::<_, String>("SELECT type FROM conversations WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

    if conv_type == "group" {
        verify_group_admin(&state, id, auth.user_id).await?;
    } else {
        verify_membership(&state, id, auth.user_id).await?;
    }

    if let Some(secs) = req.destruct_after_seconds {
        const MIN_SECS: i32 = 60; // 1 minute
        const MAX_SECS: i32 = 30 * 24 * 3600; // 30 days
        if !(MIN_SECS..=MAX_SECS).contains(&secs) {
            return Err(ApiError::BadRequest(format!(
                "destruct_after_seconds must be between {MIN_SECS} and {MAX_SECS}"
            )));
        }
    }

    sqlx::query("UPDATE conversations SET destruct_after_seconds = $1 WHERE id = $2")
        .bind(req.destruct_after_seconds)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "data": { "destruct_after_seconds": req.destruct_after_seconds }
    })))
}

pub async fn mark_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_membership(&state, id, auth.user_id).await?;

    sqlx::query("UPDATE conversation_members SET last_read_at = NOW() WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if let Err(e) = state
        .event_bus
        .publish(&DomainEvent::MessageRead {
            conversation_id: id,
            user_id: auth.user_id,
        })
        .await
    {
        tracing::warn!(
            conversation_id = id,
            user_id = auth.user_id,
            error = %e,
            "failed to publish MessageRead"
        );
    }

    publish_viewer_unread_counts(&state, auth.user_id).await;

    Ok(Json(json!({ "data": { "read": true } })))
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

async fn verify_group_admin(
    state: &AppState,
    conversation_id: i64,
    user_id: i64,
) -> Result<(), ApiError> {
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

/// Viewer-scoped member flags fetched in a single query alongside the
/// generic conversation fields, to avoid N+1 lookups.
#[derive(Debug, FromRow)]
struct ViewerMemberFlags {
    muted: bool,
    muted_until: Option<OffsetDateTime>,
    wallpaper_url: Option<String>,
    pinned: bool,
    archived: bool,
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

    let viewer_flags = sqlx::query_as::<_, ViewerMemberFlags>(
        r#"
        SELECT muted, muted_until, wallpaper_url, pinned, archived
        FROM conversation_members
        WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE
        "#,
    )
    .bind(row.id)
    .bind(viewer_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(ViewerMemberFlags {
        muted: false,
        muted_until: None,
        wallpaper_url: None,
        pinned: false,
        archived: false,
    });

    // An expired `muted_until` transparently turns into "not muted" without
    // requiring a background sweeper, giving the UI instant feedback.
    let now = OffsetDateTime::now_utc();
    let effective_muted = viewer_flags.muted || viewer_flags.muted_until.is_some_and(|t| t > now);

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
        destruct_after_seconds: row.destruct_after_seconds,
        muted: effective_muted,
        muted_until: viewer_flags.muted_until,
        wallpaper_url: viewer_flags.wallpaper_url,
        pinned: viewer_flags.pinned,
        archived: viewer_flags.archived,
    })
}

/// Recompute unread message + notification totals and fan-out `notification.counter` via NATS.
async fn publish_viewer_unread_counts(state: &AppState, user_id: i64) {
    let message_unread: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint
           FROM conversation_members cm
           JOIN messages m ON m.conversation_id = cm.conversation_id
          WHERE cm.user_id = $1
            AND cm.is_active = TRUE
            AND m.sender_id <> $1
            AND m.deleted_at IS NULL
            AND (cm.last_read_at IS NULL OR m.created_at > cm.last_read_at)"#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let notification_unread: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM notifications WHERE recipient_id = $1 AND is_read = FALSE",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let msg_count: i32 = message_unread.try_into().unwrap_or(i32::MAX);
    let notif_count: i32 = notification_unread.try_into().unwrap_or(i32::MAX);

    if let Err(e) = state
        .event_bus
        .publish(&DomainEvent::UnreadCountChanged {
            user_id,
            messages: msg_count,
            notifications: notif_count,
        })
        .await
    {
        tracing::warn!(
            user_id,
            error = %e,
            "failed to publish UnreadCountChanged from messaging-service"
        );
    }
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

    publish_viewer_unread_counts(&state, auth.user_id).await;

    Ok(Json(json!({ "data": { "marked_read": true } })))
}

// ─── Global full-text search across conversations ───────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchMessagesQuery {
    pub q: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MessageSearchResult {
    pub message_id: i64,
    pub conversation_id: i64,
    pub message_text: String,
    pub sender_id: i64,
    pub created_at: OffsetDateTime,
}

/// GET /v1/messages/search?q=...&limit=...
/// Full-text search across all conversations the authenticated user belongs to.
/// Uses the `search_vector` tsvector column (populated from `content`).
pub async fn search_messages_fts(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<SearchMessagesQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(serde_json::json!({ "data": [] })));
    }

    let limit = params.limit.unwrap_or(20).min(50);

    let rows = sqlx::query(
        r#"
        SELECT m.id              AS message_id,
               m.conversation_id,
               m.content         AS message_text,
               m.sender_id,
               m.created_at
        FROM messages m
        JOIN conversation_members cm ON cm.conversation_id = m.conversation_id AND cm.user_id = $1
        WHERE m.search_vector @@ plainto_tsquery('english', $2)
        ORDER BY m.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(query)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "FTS search failed");
        ApiError::Internal("Search failed".into())
    })?;

    let results: Vec<MessageSearchResult> = rows
        .iter()
        .map(|r| MessageSearchResult {
            message_id: r.get("message_id"),
            conversation_id: r.get("conversation_id"),
            message_text: r.get("message_text"),
            sender_id: r.get("sender_id"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": results })))
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
    let query = if query.len() > 200 {
        &query[..200]
    } else {
        query
    };

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
    let kind = params
        .r#type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // Accept: image | video | audio | file.  "file" maps to `message_type = 'file'`
    // (generic attachment).  Anything else is rejected.
    let allowed = ["image", "video", "audio", "file"];
    if let Some(k) = kind
        && !allowed.contains(&k)
    {
        return Err(ApiError::BadRequest(format!(
            "type must be one of {}",
            allowed.join(", ")
        )));
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
