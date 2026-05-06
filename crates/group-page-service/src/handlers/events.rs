use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::{FromRow, Row};
use time::OffsetDateTime;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateEventRequest {
    /// Canonical field for API clients; `title` accepted for web-client parity.
    #[validate(length(min = 1, max = 150))]
    #[serde(alias = "title")]
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub cover: Option<String>,
    #[serde(alias = "start_date")]
    pub start_at: String,
    #[serde(alias = "end_date")]
    pub end_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    #[serde(alias = "title")]
    pub name: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub cover: Option<String>,
    #[serde(alias = "start_date")]
    pub start_at: Option<String>,
    #[serde(alias = "end_date")]
    pub end_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RespondRequest {
    pub response: String,
}

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub user_ids: Vec<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct EventRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub creator_id: i64,
    pub name: String,
    pub description: String,
    pub location: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub cover: String,
    pub start_at: OffsetDateTime,
    pub end_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct EventSummary {
    pub id: i64,
    pub creator_id: i64,
    pub name: String,
    pub cover: String,
    pub location: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub start_at: OffsetDateTime,
    pub end_at: OffsetDateTime,
    pub going_count: Option<i64>,
    pub interested_count: Option<i64>,
}

fn events_page_meta(has_more: bool, events: &[EventSummary]) -> Value {
    json!({
        "has_more": has_more,
        "cursor": if has_more {
            events.last().map(|e| e.id.to_string())
        } else {
            None::<String>
        }
    })
}

#[derive(Debug, Serialize, FromRow)]
pub struct AttendeeRow {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub response: String,
}

pub async fn create_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateEventRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let start_at = parse_datetime(&req.start_at)?;
    let end_at = parse_datetime(&req.end_at)?;

    if end_at <= start_at {
        return Err(ApiError::BadRequest("end_at must be after start_at".into()));
    }

    if let Some(lat) = req.latitude {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ApiError::BadRequest(
                "latitude must be between -90 and 90".into(),
            ));
        }
    }
    if let Some(lng) = req.longitude {
        if !(-180.0..=180.0).contains(&lng) {
            return Err(ApiError::BadRequest(
                "longitude must be between -180 and 180".into(),
            ));
        }
    }
    if req.latitude.is_some() != req.longitude.is_some() {
        return Err(ApiError::BadRequest(
            "latitude and longitude must both be set or both omitted".into(),
        ));
    }

    let mut tx = state.db.begin().await?;

    let event = sqlx::query_as::<_, EventRow>(
        r#"
        INSERT INTO events (creator_id, name, description, location, latitude, longitude, cover, start_at, end_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.location.as_deref().unwrap_or(""))
    .bind(req.latitude)
    .bind(req.longitude)
    .bind(req.cover.as_deref().unwrap_or("default-cover.jpg"))
    .bind(start_at)
    .bind(end_at)
    .fetch_one(&mut *tx)
    .await?;

    // Creator auto-going
    sqlx::query(
        "INSERT INTO event_responses (event_id, user_id, response) VALUES ($1, $2, 'going')",
    )
    .bind(event.id)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(json!({ "data": event })))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let event = sqlx::query_as::<_, EventRow>("SELECT * FROM events WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Event not found".into()))?;

    let going = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM event_responses WHERE event_id = $1 AND response = 'going'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let interested = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM event_responses WHERE event_id = $1 AND response = 'interested'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "event": event,
            "going_count": going,
            "interested_count": interested
        }
    })))
}

pub async fn update_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateEventRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_event_owner(&state, id, auth.user_id).await?;

    let start_at = req.start_at.as_deref().map(parse_datetime).transpose()?;
    let end_at = req.end_at.as_deref().map(parse_datetime).transpose()?;

    if let Some(lat) = req.latitude {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ApiError::BadRequest(
                "latitude must be between -90 and 90".into(),
            ));
        }
    }
    if let Some(lng) = req.longitude {
        if !(-180.0..=180.0).contains(&lng) {
            return Err(ApiError::BadRequest(
                "longitude must be between -180 and 180".into(),
            ));
        }
    }
    if req.latitude.is_some() != req.longitude.is_some() {
        return Err(ApiError::BadRequest(
            "latitude and longitude must both be set or both omitted".into(),
        ));
    }

    let event = sqlx::query_as::<_, EventRow>(
        r#"
        UPDATE events SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            location = COALESCE($4, location),
            latitude = COALESCE($5, latitude),
            longitude = COALESCE($6, longitude),
            cover = COALESCE($7, cover),
            start_at = COALESCE($8, start_at),
            end_at = COALESCE($9, end_at)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.location)
    .bind(req.latitude)
    .bind(req.longitude)
    .bind(&req.cover)
    .bind(start_at)
    .bind(end_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": event })))
}

pub async fn delete_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_event_owner(&state, id, auth.user_id).await?;

    sqlx::query("DELETE FROM events WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn respond_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<RespondRequest>,
) -> Result<Json<Value>, ApiError> {
    let valid = ["going", "interested", "not_going"];
    if !valid.contains(&req.response.as_str()) {
        return Err(ApiError::BadRequest(
            "Invalid response. Use: going, interested, not_going".into(),
        ));
    }

    if req.response == "not_going" {
        sqlx::query("DELETE FROM event_responses WHERE event_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;
    } else {
        sqlx::query(
            r#"
            INSERT INTO event_responses (event_id, user_id, response)
            VALUES ($1, $2, $3)
            ON CONFLICT (event_id, user_id) DO UPDATE SET response = EXCLUDED.response
            "#,
        )
        .bind(id)
        .bind(auth.user_id)
        .bind(&req.response)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(json!({ "data": { "response": req.response } })))
}

pub async fn list_going(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    list_by_response(&state, id, "going", &params).await
}

pub async fn list_interested(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    list_by_response(&state, id, "interested", &params).await
}

pub async fn invite_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<InviteRequest>,
) -> Result<Json<Value>, ApiError> {
    // Verify event exists
    sqlx::query_scalar::<_, i64>("SELECT id FROM events WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Event not found".into()))?;

    let mut invited = 0i64;
    for uid in &req.user_ids {
        let result = sqlx::query(
            "INSERT INTO event_responses (event_id, user_id, response, inviter_id) VALUES ($1, $2, 'invited', $3) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(uid)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;
        invited += result.rows_affected() as i64;
    }

    Ok(Json(json!({ "data": { "invited": invited } })))
}

pub async fn upcoming_events(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let events = sqlx::query_as::<_, EventSummary>(
        r#"
        SELECT
            e.id,
            e.creator_id,
            e.name,
            e.cover,
            e.location,
            e.latitude,
            e.longitude,
            e.start_at,
            e.end_at,
            (SELECT COUNT(*)::bigint FROM event_responses er_g WHERE er_g.event_id = e.id AND er_g.response = 'going') AS going_count,
            (SELECT COUNT(*)::bigint FROM event_responses er_i WHERE er_i.event_id = e.id AND er_i.response = 'interested') AS interested_count
        FROM events e
        WHERE e.start_at > NOW()
          AND ($1::bigint IS NULL OR e.id < $1)
        ORDER BY e.start_at ASC LIMIT $2
        "#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = events.len() as i64 > limit;
    let events: Vec<_> = events.into_iter().take(limit as usize).collect();

    Ok(Json(json!({
        "data": events,
        "meta": events_page_meta(has_more, &events),
    })))
}

pub async fn my_events(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let events = sqlx::query_as::<_, EventSummary>(
        r#"
        SELECT
            e.id,
            e.creator_id,
            e.name,
            e.cover,
            e.location,
            e.latitude,
            e.longitude,
            e.start_at,
            e.end_at,
            (SELECT COUNT(*)::bigint FROM event_responses er_g WHERE er_g.event_id = e.id AND er_g.response = 'going') AS going_count,
            (SELECT COUNT(*)::bigint FROM event_responses er_i WHERE er_i.event_id = e.id AND er_i.response = 'interested') AS interested_count
        FROM events e WHERE e.creator_id = $1
        ORDER BY e.start_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": events })))
}

pub async fn attending_events(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let events = sqlx::query_as::<_, EventSummary>(
        r#"
        SELECT
            e.id,
            e.creator_id,
            e.name,
            e.cover,
            e.location,
            e.latitude,
            e.longitude,
            e.start_at,
            e.end_at,
            (SELECT COUNT(*)::bigint FROM event_responses er_g WHERE er_g.event_id = e.id AND er_g.response = 'going') AS going_count,
            (SELECT COUNT(*)::bigint FROM event_responses er_i WHERE er_i.event_id = e.id AND er_i.response = 'interested') AS interested_count
        FROM events e
        JOIN event_responses er ON er.event_id = e.id
        WHERE er.user_id = $1 AND er.response IN ('going', 'interested')
          AND e.end_at > NOW()
        ORDER BY e.start_at ASC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": events })))
}

async fn list_my_events_by_response(
    state: &AppState,
    user_id: i64,
    response: &str,
    params: &PaginationParams,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let events = sqlx::query_as::<_, EventSummary>(
        r#"
        SELECT
            e.id,
            e.creator_id,
            e.name,
            e.cover,
            e.location,
            e.latitude,
            e.longitude,
            e.start_at,
            e.end_at,
            (SELECT COUNT(*)::bigint FROM event_responses er_g WHERE er_g.event_id = e.id AND er_g.response = 'going') AS going_count,
            (SELECT COUNT(*)::bigint FROM event_responses er_i WHERE er_i.event_id = e.id AND er_i.response = 'interested') AS interested_count
        FROM events e
        JOIN event_responses er ON er.event_id = e.id
        WHERE er.user_id = $1 AND er.response = $2
          AND e.end_at > NOW()
          AND ($3::bigint IS NULL OR e.id < $3)
        ORDER BY e.start_at ASC LIMIT $4
        "#,
    )
    .bind(user_id)
    .bind(response)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = events.len() as i64 > limit;
    let events: Vec<_> = events.into_iter().take(limit as usize).collect();

    Ok(Json(json!({
        "data": events,
        "meta": events_page_meta(has_more, &events),
    })))
}

/// GET /v1/events/going — Current user's "going" events.
pub async fn going_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    list_my_events_by_response(&state, auth.user_id, "going", &params).await
}

/// GET /v1/events/interested — Current user's "interested" events.
pub async fn interested_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    list_my_events_by_response(&state, auth.user_id, "interested", &params).await
}

/// GET /v1/events/invited — Current user's pending invitations.
pub async fn invited_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    list_my_events_by_response(&state, auth.user_id, "invited", &params).await
}

/// GET /v1/events/past — Past events the user attended or created.
pub async fn past_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let events = sqlx::query_as::<_, EventSummary>(
        r#"
        SELECT DISTINCT
            e.id,
            e.creator_id,
            e.name,
            e.cover,
            e.location,
            e.latitude,
            e.longitude,
            e.start_at,
            e.end_at,
            (SELECT COUNT(*)::bigint FROM event_responses er_g WHERE er_g.event_id = e.id AND er_g.response = 'going') AS going_count,
            (SELECT COUNT(*)::bigint FROM event_responses er_i WHERE er_i.event_id = e.id AND er_i.response = 'interested') AS interested_count
        FROM events e
        LEFT JOIN event_responses er ON er.event_id = e.id AND er.user_id = $1
        WHERE e.end_at < NOW()
          AND (e.creator_id = $1 OR er.response IN ('going', 'interested'))
          AND ($2::bigint IS NULL OR e.id < $2)
        ORDER BY e.end_at DESC LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = events.len() as i64 > limit;
    let events: Vec<_> = events.into_iter().take(limit as usize).collect();

    Ok(Json(json!({
        "data": events,
        "meta": events_page_meta(has_more, &events),
    })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn verify_event_owner(state: &AppState, event_id: i64, user_id: i64) -> Result<(), ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT creator_id FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Event not found".into()))?;

    if owner != user_id {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

async fn list_by_response(
    state: &AppState,
    event_id: i64,
    response: &str,
    params: &PaginationParams,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let attendees = sqlx::query_as::<_, AttendeeRow>(
        r#"
        SELECT er.user_id, u.username, u.first_name, u.last_name, u.avatar, er.response
        FROM event_responses er JOIN users u ON u.id = er.user_id
        WHERE er.event_id = $1 AND er.response = $2
          AND ($3::bigint IS NULL OR er.id < $3)
        ORDER BY er.id DESC LIMIT $4
        "#,
    )
    .bind(event_id)
    .bind(response)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = attendees.len() as i64 > limit;
    let attendees: Vec<_> = attendees.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": attendees, "meta": { "has_more": has_more } }),
    ))
}

fn parse_datetime(s: &str) -> Result<OffsetDateTime, ApiError> {
    // Accept ISO 8601 format
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).map_err(|_| {
        ApiError::BadRequest(format!(
            "Invalid datetime format: {}. Use RFC3339 (e.g. 2025-12-31T18:00:00Z)",
            s
        ))
    })
}

// ─── Event Cohosts ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddCohostRequest {
    pub user_id: i64,
}

// POST /v1/events/{id}/cohosts
pub async fn add_event_cohost(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<i64>,
    Json(body): Json<AddCohostRequest>,
) -> Result<Json<()>, ApiError> {
    // Verify user is the event creator
    let owner = sqlx::query("SELECT user_id FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_optional(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    match owner {
        Some(row) => {
            let owner_id: i64 = row.get("user_id");
            if owner_id != auth.user_id {
                return Err(ApiError::Forbidden("Only the event creator can manage co-hosts".into()));
            }
        }
        None => return Err(ApiError::NotFound("Event not found".into())),
    }

    sqlx::query("INSERT INTO event_cohosts (event_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(event_id).bind(body.user_id)
        .execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(()))
}

// DELETE /v1/events/{eid}/cohosts/{uid}
pub async fn remove_event_cohost(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((event_id, user_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    // Verify user is the event creator
    let owner = sqlx::query("SELECT user_id FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_optional(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    match owner {
        Some(row) => {
            let owner_id: i64 = row.get("user_id");
            if owner_id != auth.user_id {
                return Err(ApiError::Forbidden("Only the event creator can manage co-hosts".into()));
            }
        }
        None => return Err(ApiError::NotFound("Event not found".into())),
    }

    sqlx::query("DELETE FROM event_cohosts WHERE event_id = $1 AND user_id = $2")
        .bind(event_id).bind(user_id)
        .execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(()))
}

// ─── Event Discussion ────────────────────────────────────────────────────────

// GET /v1/events/{id}/discussion
pub async fn list_event_discussion(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let rows = sqlx::query(
        "SELECT ed.id, ed.user_id, ed.content, ed.created_at, u.username, u.first_name, u.last_name, u.avatar
         FROM event_discussions ed JOIN users u ON u.id = ed.user_id
         WHERE ed.event_id = $1 ORDER BY ed.created_at DESC LIMIT 50"
    )
    .bind(event_id)
    .fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<i64, _>("id"),
        "user_id": r.get::<i64, _>("user_id"),
        "content": r.get::<String, _>("content"),
        "created_at": r.get::<String, _>("created_at"),
        "username": r.get::<String, _>("username"),
        "first_name": r.get::<String, _>("first_name"),
        "last_name": r.get::<String, _>("last_name"),
        "avatar": r.get::<Option<String>, _>("avatar"),
    })).collect();
    Ok(Json(items))
}

// POST /v1/events/{id}/discussion
#[derive(Deserialize)]
pub struct DiscussionMessage {
    pub content: String,
}

pub async fn add_event_discussion(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<i64>,
    Json(body): Json<DiscussionMessage>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query(
        "INSERT INTO event_discussions (event_id, user_id, content) VALUES ($1, $2, $3) RETURNING id, created_at"
    )
    .bind(event_id).bind(auth.user_id).bind(&body.content)
    .fetch_one(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "user_id": auth.user_id,
        "content": body.content,
        "created_at": row.get::<String, _>("created_at"),
    })))
}

// ── Event Tickets ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PurchaseTicketRequest {
    pub tier: Option<String>,
}

pub async fn purchase_ticket(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<i64>,
    Json(body): Json<PurchaseTicketRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let tier = body.tier.as_deref().unwrap_or("general");
    let qr_data = format!(
        "ticket:{}:{}:{}",
        event_id,
        auth.user_id,
        time::OffsetDateTime::now_utc().unix_timestamp()
    );

    let row = sqlx::query(
        "INSERT INTO event_tickets (event_id, user_id, tier, price_cents, qr_code, created_at)
         VALUES ($1, $2, $3, 0, $4, NOW())
         RETURNING id",
    )
    .bind(event_id)
    .bind(auth.user_id)
    .bind(tier)
    .bind(&qr_data)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "event_id": event_id,
        "tier": tier,
        "qr_code": qr_data,
    })))
}

// GET /v1/events/{id}/tickets -- list user's tickets
pub async fn list_my_tickets(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<i64>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, event_id, user_id, tier, price_cents, qr_code, is_used, created_at
         FROM event_tickets WHERE event_id = $1 AND user_id = $2 ORDER BY created_at DESC",
    )
    .bind(event_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    let tickets: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "event_id": r.get::<i64, _>("event_id"),
                "tier": r.get::<String, _>("tier"),
                "qr_code": r.get::<String, _>("qr_code"),
                "is_used": r.get::<bool, _>("is_used"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(tickets))
}
