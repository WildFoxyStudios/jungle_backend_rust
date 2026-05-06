use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    permissions::Permission,
};
use sqlx::Row;

#[derive(Deserialize)]
pub struct ListTicketsParams {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// GET /v1/admin/support/tickets
pub async fn list_tickets(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListTicketsParams>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewUsers, &state).await?;

    let rows = sqlx::query(
        "SELECT t.id, t.user_id, t.subject, t.status, t.priority, t.assigned_to, t.created_at, t.updated_at,
                u.username, u.first_name, u.last_name
         FROM support_tickets t
         JOIN users u ON u.id = t.user_id
         WHERE ($1::text IS NULL OR t.status = $1)
           AND ($2::text IS NULL OR t.priority = $2)
         ORDER BY
           CASE t.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END,
           t.created_at DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(&params.status)
    .bind(&params.priority)
    .bind(params.limit.unwrap_or(20))
    .bind(params.offset.unwrap_or(0))
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    let tickets: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64, _>("id"),
                "user_id": r.get::<i64, _>("user_id"),
                "subject": r.get::<String, _>("subject"),
                "status": r.get::<String, _>("status"),
                "priority": r.get::<String, _>("priority"),
                "assigned_to": r.get::<Option<i64>, _>("assigned_to"),
                "created_at": r.get::<String, _>("created_at"),
                "username": r.get::<String, _>("username"),
                "first_name": r.get::<String, _>("first_name"),
                "last_name": r.get::<String, _>("last_name"),
            })
        })
        .collect();

    Ok(Json(json!({ "items": tickets, "total": tickets.len() })))
}

// GET /v1/admin/support/tickets/{id}
pub async fn get_ticket(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ticket_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewUsers, &state).await?;

    let ticket = sqlx::query(
        "SELECT t.*, u.username, u.first_name, u.last_name, u.avatar
         FROM support_tickets t JOIN users u ON u.id = t.user_id WHERE t.id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    let messages = sqlx::query(
        "SELECT m.id, m.user_id, m.content, m.is_staff_reply, m.created_at, u.username, u.first_name
         FROM support_ticket_messages m JOIN users u ON u.id = m.user_id
         WHERE m.ticket_id = $1 ORDER BY m.created_at",
    )
    .bind(ticket_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let msgs: Vec<Value> = messages
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64, _>("id"),
                "user_id": r.get::<i64, _>("user_id"),
                "content": r.get::<String, _>("content"),
                "is_staff_reply": r.get::<bool, _>("is_staff_reply"),
                "created_at": r.get::<String, _>("created_at"),
                "username": r.get::<String, _>("username"),
            })
        })
        .collect();

    Ok(Json(json!({
        "ticket": ticket.map(|r| json!({
            "id": r.get::<i64, _>("id"),
            "subject": r.get::<String, _>("subject"),
            "status": r.get::<String, _>("status"),
            "priority": r.get::<String, _>("priority"),
            "username": r.get::<String, _>("username"),
        })),
        "messages": msgs,
    })))
}

// PUT /v1/admin/support/tickets/{id}
#[derive(Deserialize)]
pub struct UpdateTicketRequest {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assigned_to: Option<i64>,
}

pub async fn update_ticket(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ticket_id): Path<i64>,
    Json(body): Json<UpdateTicketRequest>,
) -> Result<Json<()>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;

    sqlx::query(
        "UPDATE support_tickets SET status = COALESCE($1, status), priority = COALESCE($2, priority), assigned_to = COALESCE($3, assigned_to), updated_at = NOW() WHERE id = $4",
    )
    .bind(&body.status)
    .bind(&body.priority)
    .bind(body.assigned_to)
    .bind(ticket_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;
    Ok(Json(()))
}

// POST /v1/admin/support/tickets/{id}/reply
#[derive(Deserialize)]
pub struct ReplyRequest {
    pub content: String,
}

pub async fn reply_to_ticket(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ticket_id): Path<i64>,
    Json(body): Json<ReplyRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;

    let row = sqlx::query(
        "INSERT INTO support_ticket_messages (ticket_id, user_id, content, is_staff_reply) VALUES ($1, $2, $3, TRUE) RETURNING id, created_at",
    )
    .bind(ticket_id)
    .bind(auth.user_id)
    .bind(&body.content)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    sqlx::query(
        "UPDATE support_tickets SET updated_at = NOW(), status = 'in_progress' WHERE id = $1 AND status = 'open'",
    )
    .bind(ticket_id)
    .execute(&state.db)
    .await
    .ok();

    Ok(Json(json!({
        "id": row.get::<i64, _>("id"),
        "content": body.content,
        "is_staff_reply": true,
        "created_at": row.get::<String, _>("created_at"),
    })))
}
