use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateBroadcastRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub avatar: Option<String>,
    pub member_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateBroadcastRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMembersRequest {
    pub member_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SendBroadcastRequest {
    pub content: String,
    pub message_type: Option<String>,
    pub media: Option<Value>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BroadcastRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub avatar: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BroadcastMemberInfo {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

pub async fn list_broadcasts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let broadcasts = sqlx::query_as::<_, BroadcastRow>(
        "SELECT id, user_id, name, avatar, created_at FROM broadcasts WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": broadcasts })))
}

pub async fn create_broadcast(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateBroadcastRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    if req.member_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "Broadcast must have at least one member".into(),
        ));
    }

    let mut tx = state.db.begin().await?;

    let broadcast_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO broadcasts (user_id, name, avatar) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&req.avatar)
    .fetch_one(&mut *tx)
    .await?;

    for member_id in &req.member_ids {
        sqlx::query(
            "INSERT INTO broadcast_members (broadcast_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(broadcast_id)
        .bind(member_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let broadcast = sqlx::query_as::<_, BroadcastRow>(
        "SELECT id, user_id, name, avatar, created_at FROM broadcasts WHERE id = $1",
    )
    .bind(broadcast_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": broadcast })))
}

pub async fn update_broadcast(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateBroadcastRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    let mut sets = Vec::new();
    let mut param_idx = 2u32;

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

    let query_str = format!("UPDATE broadcasts SET {} WHERE id = $1", sets.join(", "));
    let mut q = sqlx::query(&query_str).bind(id);
    if let Some(ref name) = req.name {
        q = q.bind(name);
    }
    if let Some(ref avatar) = req.avatar {
        q = q.bind(avatar);
    }
    q.execute(&state.db).await?;

    Ok(Json(json!({ "data": { "message": "Broadcast updated" } })))
}

pub async fn delete_broadcast(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    sqlx::query("DELETE FROM broadcasts WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    let members = sqlx::query_as::<_, BroadcastMemberInfo>(
        r#"
        SELECT bm.user_id, u.username, u.first_name, u.last_name, u.avatar
        FROM broadcast_members bm
        JOIN users u ON u.id = bm.user_id
        WHERE bm.broadcast_id = $1
        ORDER BY bm.created_at
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": members })))
}

pub async fn add_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AddMembersRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    let mut added = 0i64;
    for member_id in &req.member_ids {
        let result = sqlx::query(
            "INSERT INTO broadcast_members (broadcast_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(member_id)
        .execute(&state.db)
        .await?;
        added += result.rows_affected() as i64;
    }

    Ok(Json(json!({ "data": { "added": added } })))
}

pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    sqlx::query("DELETE FROM broadcast_members WHERE broadcast_id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "removed": true } })))
}

pub async fn send_broadcast(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<SendBroadcastRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    if req.content.trim().is_empty() {
        return Err(ApiError::BadRequest("Message content is required".into()));
    }

    let msg_type = req.message_type.as_deref().unwrap_or("text");
    let media = req.media.unwrap_or(json!([]));

    let members = sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM broadcast_members WHERE broadcast_id = $1",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let mut sent = 0i64;
    for member_id in &members {
        let conv_id = find_or_create_direct(&state, auth.user_id, *member_id).await?;

        sqlx::query(
            "INSERT INTO messages (conversation_id, sender_id, content, message_type, media) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(conv_id)
        .bind(auth.user_id)
        .bind(&req.content)
        .bind(msg_type)
        .bind(&media)
        .execute(&state.db)
        .await?;

        sqlx::query("UPDATE conversations SET last_message_at = NOW() WHERE id = $1")
            .bind(conv_id)
            .execute(&state.db)
            .await?;

        sent += 1;
    }

    Ok(Json(json!({ "data": { "sent_to": sent } })))
}

// ---- Helpers ----

async fn verify_owner(state: &AppState, broadcast_id: i64, user_id: i64) -> Result<(), ApiError> {
    let owner_id = sqlx::query_scalar::<_, i64>("SELECT user_id FROM broadcasts WHERE id = $1")
        .bind(broadcast_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Broadcast not found".into()))?;

    if owner_id != user_id {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

async fn find_or_create_direct(
    state: &AppState,
    user_a: i64,
    user_b: i64,
) -> Result<i64, ApiError> {
    let existing = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT c.id FROM conversations c
        WHERE c.type = 'direct'
          AND EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $1 AND is_active = TRUE)
          AND EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $2 AND is_active = TRUE)
        LIMIT 1
        "#,
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_optional(&state.db)
    .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

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
    .bind(user_a)
    .bind(user_b)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(conv_id)
}
