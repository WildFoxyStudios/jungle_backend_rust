//! Analytics + ICS export + autoresponder endpoints for pages/groups/events.
//! Plan §3.5 E1 (events .ics), PG1 (page autoresponder), G1 (group analytics).

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{auth::AppState, auth::AuthUser, errors::ApiError};
use sqlx::FromRow;
use time::OffsetDateTime;
use time::format_description::well_known::Iso8601;

// ═══════════════════════════════════════════════════════════════════
// GET /v1/events/{id}/ics — iCalendar file
// ═══════════════════════════════════════════════════════════════════

#[derive(FromRow)]
struct EventIcsRow {
    id: i64,
    name: String,
    description: String,
    location: String,
    start_at: OffsetDateTime,
    end_at: OffsetDateTime,
    created_at: OffsetDateTime,
}

/// RFC 5545 text escaping: backslash-escape commas, semicolons, newlines.
fn ics_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

/// RFC 5545 `DTSTART/DTEND` value in UTC basic format `YYYYMMDDTHHMMSSZ`.
fn ics_datetime(ts: OffsetDateTime) -> String {
    let ts_utc = ts.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        ts_utc.year(),
        ts_utc.month() as u8,
        ts_utc.day(),
        ts_utc.hour(),
        ts_utc.minute(),
        ts_utc.second(),
    )
}

/// Fold long lines per RFC 5545 §3.1 (max 75 octets, continuation starts
/// with a single space). Keeps Outlook / Apple Calendar happy.
fn fold_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 70);
    let mut count = 0;
    for ch in s.chars() {
        if count >= 73 {
            out.push_str("\r\n ");
            count = 1; // the single leading space
        }
        out.push(ch);
        count += 1;
    }
    out
}

pub async fn export_event_ics(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let event = sqlx::query_as::<_, EventIcsRow>(
        "SELECT id, name, description, location, start_at, end_at, created_at \
           FROM events WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Event not found".into()))?;

    let uid = format!("event-{}@jungle.social", event.id);
    let summary = ics_escape(&event.name);
    let description = ics_escape(&event.description);
    let location = ics_escape(&event.location);

    let dtstart = ics_datetime(event.start_at);
    let dtend = ics_datetime(event.end_at);
    let dtstamp = ics_datetime(event.created_at);

    let lines: Vec<String> = vec![
        "BEGIN:VCALENDAR".into(),
        "VERSION:2.0".into(),
        "PRODID:-//Jungle Social//Events//EN".into(),
        "CALSCALE:GREGORIAN".into(),
        "METHOD:PUBLISH".into(),
        "BEGIN:VEVENT".into(),
        fold_line(&format!("UID:{uid}")),
        format!("DTSTAMP:{dtstamp}"),
        format!("DTSTART:{dtstart}"),
        format!("DTEND:{dtend}"),
        fold_line(&format!("SUMMARY:{summary}")),
        fold_line(&format!("DESCRIPTION:{description}")),
        fold_line(&format!("LOCATION:{location}")),
        "END:VEVENT".into(),
        "END:VCALENDAR".into(),
    ];

    let body = lines.join("\r\n") + "\r\n";
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/calendar; charset=utf-8"),
    );
    let filename = format!("event-{}.ics", event.id);
    if let Ok(cd) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, cd);
    }
    Ok((StatusCode::OK, headers, body))
}

// ═══════════════════════════════════════════════════════════════════
// GET /v1/groups/{id}/analytics — basic analytics dashboard data
// ═══════════════════════════════════════════════════════════════════

#[derive(Serialize)]
pub struct GroupAnalytics {
    pub total_members: i64,
    pub new_members_last_7d: i64,
    pub new_members_last_30d: i64,
    pub total_posts: i64,
    pub posts_last_7d: i64,
    pub posts_last_30d: i64,
    pub total_reactions: i64,
    pub total_comments: i64,
    pub pending_join_requests: i64,
}

/// Only group admins may see the dashboard numbers.
async fn verify_group_admin(
    pool: &sqlx::PgPool,
    group_id: i64,
    user_id: i64,
) -> Result<(), ApiError> {
    let is_admin: Option<bool> = sqlx::query_scalar(
        "SELECT role IN ('owner','admin') FROM group_members \
          WHERE group_id = $1 AND user_id = $2",
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if is_admin.unwrap_or(false) {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Only group admins may view analytics".into(),
        ))
    }
}

pub async fn group_analytics(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state.db, id, auth.user_id).await?;

    // One round-trip per metric keeps the SQL simple; this endpoint is
    // cold-cached at the frontend level so throughput isn't a concern.
    let total_members: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM group_members WHERE group_id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    let new_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM group_members \
          WHERE group_id = $1 AND joined_at > NOW() - INTERVAL '7 day'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let new_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM group_members \
          WHERE group_id = $1 AND joined_at > NOW() - INTERVAL '30 day'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let total_posts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts \
          WHERE group_id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let posts_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts \
          WHERE group_id = $1 AND deleted_at IS NULL \
            AND created_at > NOW() - INTERVAL '7 day'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let posts_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts \
          WHERE group_id = $1 AND deleted_at IS NULL \
            AND created_at > NOW() - INTERVAL '30 day'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let total_reactions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM post_reactions pr \
           JOIN posts p ON p.id = pr.post_id \
          WHERE p.group_id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let total_comments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM post_comments pc \
           JOIN posts p ON p.id = pc.post_id \
          WHERE p.group_id = $1 AND pc.deleted_at IS NULL",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM group_join_requests \
          WHERE group_id = $1 AND status = 'pending'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Snapshot `created_at` bucket — lets the frontend render a sparkline
    // of the last 14 days without a second endpoint.
    let timeseries: Vec<(OffsetDateTime, i64)> = sqlx::query_as(
        r#"SELECT DATE_TRUNC('day', created_at)::timestamptz AS day, COUNT(*)::bigint
             FROM posts
            WHERE group_id = $1 AND deleted_at IS NULL
              AND created_at > NOW() - INTERVAL '14 day'
            GROUP BY day
            ORDER BY day"#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let timeseries_json: Vec<Value> = timeseries
        .into_iter()
        .map(|(day, count)| {
            json!({
                "day": day.format(&Iso8601::DEFAULT).unwrap_or_default(),
                "count": count,
            })
        })
        .collect();

    Ok(Json(json!({
        "data": {
            "analytics": GroupAnalytics {
                total_members,
                new_members_last_7d: new_7d,
                new_members_last_30d: new_30d,
                total_posts,
                posts_last_7d: posts_7d,
                posts_last_30d: posts_30d,
                total_reactions,
                total_comments,
                pending_join_requests: pending,
            },
            "timeseries": timeseries_json,
        }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// PUT /v1/pages/{id}/autoresponder — save auto-reply for messages
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct AutoresponderRequest {
    pub enabled: bool,
    pub message: String,
}

async fn verify_page_admin(
    pool: &sqlx::PgPool,
    page_id: i64,
    user_id: i64,
) -> Result<(), ApiError> {
    let is_admin: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pages WHERE id = $1 AND owner_id = $2) \
          OR EXISTS (SELECT 1 FROM page_admins WHERE page_id = $1 AND user_id = $2)",
    )
    .bind(page_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if is_admin.unwrap_or(false) {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Only page admins may edit autoresponder".into(),
        ))
    }
}

pub async fn get_autoresponder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_page_admin(&state.db, id, auth.user_id).await?;

    // Stored on the page row as two columns; falls back to disabled.
    let row: Option<(bool, String)> = sqlx::query_as(
        "SELECT COALESCE(autoresponder_enabled, FALSE), \
                COALESCE(autoresponder_message, '') \
           FROM pages WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let (enabled, message) = row.unwrap_or((false, String::new()));
    Ok(Json(json!({
        "data": { "enabled": enabled, "message": message }
    })))
}

pub async fn put_autoresponder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AutoresponderRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_page_admin(&state.db, id, auth.user_id).await?;

    if req.message.len() > 2000 {
        return Err(ApiError::BadRequest("message too long".into()));
    }

    sqlx::query(
        "UPDATE pages \
            SET autoresponder_enabled = $2, \
                autoresponder_message = $3 \
          WHERE id = $1",
    )
    .bind(id)
    .bind(req.enabled)
    .bind(req.message.trim())
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "data": { "enabled": req.enabled, "message": req.message }
    })))
}
