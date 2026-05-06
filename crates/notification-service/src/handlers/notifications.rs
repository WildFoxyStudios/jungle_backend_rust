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
};
use sqlx::{FromRow, Row};
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
    // Phase 5: Grouped notifications + deep links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_link: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct SenderInfo {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

#[derive(Debug, Deserialize)]
pub struct ListNotificationsParams {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub grouped: Option<bool>,
}

impl ListNotificationsParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn cursor_id(&self) -> Option<i64> {
        self.cursor.as_ref().and_then(|c| c.parse::<i64>().ok())
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn list_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListNotificationsParams>,
) -> Result<Json<Value>, ApiError> {
    if params.grouped.unwrap_or(false) {
        list_grouped_notifications(state, auth, params).await
    } else {
        list_flat_notifications(state, auth, params).await
    }
}

/// Flat (non-grouped) listing — original behaviour, unchanged.
async fn list_flat_notifications(
    state: AppState,
    auth: AuthUser,
    params: ListNotificationsParams,
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
                group_key: None,
                group_count: None,
                deep_link: None,
            }
        })
        .collect();

    Ok(Json(json!({
        "data": notifications,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// Grouped listing — merges notifications that share a `group_key` into a single
/// entry with an aggregate `group_count` and a rewritten text like
/// "liked your post and 3 others".
async fn list_grouped_notifications(
    state: AppState,
    auth: AuthUser,
    params: ListNotificationsParams,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let fetch_limit = limit + 1;

    let rows = sqlx::query(
        r#"WITH grouped AS (
            SELECT
                COALESCE(group_key, id::TEXT) AS effective_key,
                MAX(id)                       AS latest_id,
                COUNT(*)                      AS cnt,
                BOOL_AND(is_read)             AS all_read
            FROM notifications
            WHERE recipient_id = $1
            GROUP BY COALESCE(group_key, id::TEXT)
        )
        SELECT
            n.id, n.recipient_id, n.sender_id, n.type,
            n.target_type, n.target_id,
            n.text, n.url, n.created_at,
            n.group_key, n.deep_link,
            g.cnt        AS group_count,
            g.all_read   AS is_read
        FROM grouped g
        JOIN notifications n ON n.id = g.latest_id
        WHERE ($2::bigint IS NULL OR n.id < $2)
        ORDER BY n.id DESC
        LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(fetch_limit)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();

    // Batch fetch senders
    let sender_ids: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.get::<Option<i64>, _>("sender_id"))
        .collect();
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
            let sender_id: Option<i64> = row.get("sender_id");
            let sender = sender_id
                .and_then(|sid| senders.iter().find(|s| s.id == sid))
                .map(|s| SenderInfo {
                    id: s.id,
                    username: s.username.clone(),
                    first_name: s.first_name.clone(),
                    last_name: s.last_name.clone(),
                    avatar: s.avatar.clone(),
                });

            let group_count: i64 = row.get("group_count");
            let base_text: String = row.get("text");
            let text = if group_count > 1 {
                format!("{} and {} others", base_text, group_count - 1)
            } else {
                base_text
            };

            NotificationResponse {
                id: row.get("id"),
                r#type: row.get("type"),
                text,
                url: row.get("url"),
                target_type: row.get("target_type"),
                target_id: row.get("target_id"),
                is_read: row.get("is_read"),
                created_at: row.get("created_at"),
                sender,
                group_key: row.get("group_key"),
                group_count: Some(group_count),
                deep_link: row.get("deep_link"),
            }
        })
        .collect();

    let next_cursor = notifications.last().map(|n| n.id.to_string());

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
    let result =
        sqlx::query("UPDATE notifications SET is_read = TRUE WHERE id = $1 AND recipient_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Notification not found".into()));
    }

    publish_unread_change(&state, auth.user_id).await;

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

    publish_unread_change(&state, auth.user_id).await;

    Ok(Json(
        json!({ "data": { "marked": result.rows_affected() } }),
    ))
}

/// Compute the up-to-date unread message + notification counters and broadcast
/// them via the realtime hub so all sessions stay in sync after a mutation.
async fn publish_unread_change(state: &AppState, user_id: i64) {
    let messages = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)::bigint
           FROM conversation_members cm
           JOIN messages m ON m.conversation_id = cm.conversation_id
          WHERE cm.user_id = $1
            AND cm.is_active = TRUE
            AND m.sender_id <> $1
            AND (cm.last_read_at IS NULL OR m.created_at > cm.last_read_at)"#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let notifications = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND is_read = FALSE",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let event = DomainEvent::UnreadCountChanged {
        user_id,
        messages: messages.try_into().unwrap_or(0),
        notifications: notifications.try_into().unwrap_or(0),
    };
    if let Err(e) = state.event_bus.publish(&event).await {
        tracing::warn!(user_id, error = %e, "failed to publish UnreadCountChanged");
    }
}

pub async fn delete_notification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM notifications WHERE id = $1 AND recipient_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Notification not found".into()));
    }

    publish_unread_change(&state, auth.user_id).await;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn clear_all(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let result =
        sqlx::query("DELETE FROM notifications WHERE recipient_id = $1 AND is_read = TRUE")
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;

    publish_unread_change(&state, auth.user_id).await;

    Ok(Json(
        json!({ "data": { "cleared": result.rows_affected() } }),
    ))
}
