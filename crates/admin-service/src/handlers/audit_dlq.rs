//! Admin audit log & DLQ management endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

// ── Audit log ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub admin_user_id: Option<i64>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub from: Option<String>, // ISO8601
    pub to: Option<String>,
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_audit_log(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let from_ts = q
        .from
        .as_deref()
        .and_then(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT).ok());
    let to_ts = q
        .to
        .as_deref()
        .and_then(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT).ok());

    type AuditRow = (
        i64,
        i64,
        String,
        String,
        Option<String>,
        String,
        i32,
        Option<Value>,
        Option<String>,
        Option<String>,
        time::OffsetDateTime,
    );

    let rows: Vec<AuditRow> = sqlx::query_as(
        r#"SELECT id, admin_user_id, action, resource_type, resource_id,
                  endpoint, status, changes, ip_address::text, user_agent, created_at
             FROM admin_audit_log
            WHERE ($1::bigint IS NULL OR admin_user_id = $1)
              AND ($2::text IS NULL OR action = $2)
              AND ($3::text IS NULL OR resource_type = $3)
              AND ($4::timestamptz IS NULL OR created_at >= $4)
              AND ($5::timestamptz IS NULL OR created_at <= $5)
              AND ($6::bigint IS NULL OR id < $6)
         ORDER BY id DESC
            LIMIT $7"#,
    )
    .bind(q.admin_user_id)
    .bind(&q.action)
    .bind(&q.resource_type)
    .bind(from_ts)
    .bind(to_ts)
    .bind(q.cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<AuditRow> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.0);

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, uid, action, rtype, rid, endpoint, status, changes, ip, ua, ts)| {
            json!({
                "id": id,
                "admin_user_id": uid,
                "action": action,
                "resource_type": rtype,
                "resource_id": rid,
                "endpoint": endpoint,
                "status": status,
                "changes": changes,
                "ip_address": ip,
                "user_agent": ua,
                "created_at": ts.to_string(),
            })
        })
        .collect();

    Ok(Json(json!({
        "data": data,
        "meta": { "has_more": has_more, "next_cursor": next_cursor }
    })))
}

// ── DLQ ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DlqQuery {
    pub subject: Option<String>,
    pub include_consumed: Option<bool>,
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_dlq(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<DlqQuery>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let include_consumed = q.include_consumed.unwrap_or(false);

    type DlqRow = (
        i64,
        String,
        Value,
        Option<String>,
        i32,
        Option<time::OffsetDateTime>,
        Option<time::OffsetDateTime>,
        Option<i64>,
        time::OffsetDateTime,
    );

    let rows: Vec<DlqRow> = sqlx::query_as(
        r#"SELECT id, subject, payload, error, attempt,
                  retry_at, consumed_at, consumed_by, created_at
             FROM event_dlq
            WHERE ($1::text IS NULL OR subject LIKE $1 || '%')
              AND ($2::bool OR consumed_at IS NULL)
              AND ($3::bigint IS NULL OR id < $3)
         ORDER BY id DESC
            LIMIT $4"#,
    )
    .bind(&q.subject)
    .bind(include_consumed)
    .bind(q.cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<DlqRow> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.0);

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, subject, payload, error, attempt, retry_at, consumed_at, consumed_by, created_at)| {
            json!({
                "id": id,
                "subject": subject,
                "payload": payload,
                "error": error,
                "attempt": attempt,
                "retry_at": retry_at.map(|t| t.to_string()),
                "consumed_at": consumed_at.map(|t| t.to_string()),
                "consumed_by": consumed_by,
                "created_at": created_at.to_string(),
            })
        })
        .collect();

    Ok(Json(json!({
        "data": data,
        "meta": { "has_more": has_more, "next_cursor": next_cursor }
    })))
}

pub async fn discard_dlq_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    sqlx::query(
        r#"UPDATE event_dlq
              SET consumed_at = NOW(), consumed_by = $2
            WHERE id = $1 AND consumed_at IS NULL"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "discarded": true } })))
}

pub async fn retry_dlq_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    // Fetch the event
    let row: Option<(String, Value, i32)> = sqlx::query_as(
        r#"SELECT subject, payload, attempt FROM event_dlq
            WHERE id = $1 AND consumed_at IS NULL"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let (subject, payload, attempt) = row.ok_or(ApiError::NotFound("DLQ item not found".into()))?;

    // Strip the `dlq.` prefix and re-publish
    let real_subject = subject.strip_prefix("dlq.").unwrap_or(&subject);
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|e| ApiError::Internal(format!("serialize: {}", e)))?;

    match state
        .event_bus
        .publish_raw(real_subject, &payload_bytes)
        .await
    {
        Ok(_) => {
            sqlx::query(
                r#"UPDATE event_dlq
                      SET consumed_at = NOW(),
                          consumed_by = $2,
                          attempt = attempt + 1
                    WHERE id = $1"#,
            )
            .bind(id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;
            tracing::info!(
                dlq_id = id,
                subject = real_subject,
                admin_id = auth.user_id,
                "DLQ event re-published"
            );
            Ok(Json(json!({
                "data": { "retried": true, "subject": real_subject, "attempt": attempt + 1 }
            })))
        }
        Err(e) => Err(ApiError::Internal(format!("republish failed: {}", e))),
    }
}
