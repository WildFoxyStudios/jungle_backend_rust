use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct NotificationRow {
    pub id: i64,
    pub recipient_id: i64,
    pub sender_id: Option<i64>,
    pub r#type: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub text: String,
    pub url: String,
    pub is_read: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub id: i64,
    pub r#type: String,
    pub text: String,
    pub url: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub is_read: bool,
    pub created_at: OffsetDateTime,
    pub sender: Option<SenderInfo>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct SenderInfo {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn list_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let fetch_limit = limit + 1;

    let rows = sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT id, recipient_id, sender_id, type, target_type, target_id, text, url, is_read, created_at
        FROM notifications
        WHERE recipient_id = $1
          AND ($2::bigint IS NULL OR id < $2)
        ORDER BY id DESC
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

    // Batch fetch senders
    let sender_ids: Vec<i64> = rows.iter().filter_map(|r| r.sender_id).collect();
    let senders = if !sender_ids.is_empty() {
        sqlx::query_as::<_, SenderInfo>(
            "SELECT id, username, first_name, last_name, avatar FROM users WHERE id = ANY($1::bigint[])",
        )
        .bind(&sender_ids)
        .fetch_all(&state.db)
        .await?
    } else {
        vec![]
    };

    let notifications: Vec<NotificationResponse> = rows
        .into_iter()
        .map(|row| {
            let sender = row
                .sender_id
                .and_then(|sid| senders.iter().find(|s| s.id == sid))
                .map(|s| SenderInfo {
                    id: s.id,
                    username: s.username.clone(),
                    first_name: s.first_name.clone(),
                    last_name: s.last_name.clone(),
                    avatar: s.avatar.clone(),
                });

            NotificationResponse {
                id: row.id,
                r#type: row.r#type,
                text: row.text,
                url: row.url,
                target_type: row.target_type,
                target_id: row.target_id,
                is_read: row.is_read,
                created_at: row.created_at,
                sender,
            }
        })
        .collect();

    Ok(Json(json!({
        "data": notifications,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

pub async fn unread_count(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND is_read = FALSE",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "unread_count": count } })))
}

pub async fn mark_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = TRUE WHERE id = $1 AND recipient_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Notification not found".into()));
    }

    Ok(Json(json!({ "data": { "read": true } })))
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = TRUE WHERE recipient_id = $1 AND is_read = FALSE",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "marked": result.rows_affected() } })))
}

pub async fn delete_notification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "DELETE FROM notifications WHERE id = $1 AND recipient_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Notification not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn clear_all(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "DELETE FROM notifications WHERE recipient_id = $1 AND is_read = TRUE",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "cleared": result.rows_affected() } })))
}
