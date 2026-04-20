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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateEventRequest {
    #[validate(length(min = 1, max = 150))]
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub cover: Option<String>,
    pub start_at: String,
    pub end_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub cover: Option<String>,
    pub start_at: Option<String>,
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
    pub cover: String,
    pub start_at: OffsetDateTime,
    pub end_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct EventSummary {
    pub id: i64,
    pub name: String,
    pub cover: String,
    pub start_at: OffsetDateTime,
    pub end_at: OffsetDateTime,
    pub going_count: Option<i64>,
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

    let mut tx = state.db.begin().await?;

    let event = sqlx::query_as::<_, EventRow>(
        r#"
        INSERT INTO events (creator_id, name, description, location, cover, start_at, end_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.location.as_deref().unwrap_or(""))
    .bind(req.cover.as_deref().unwrap_or("default-cover.jpg"))
    .bind(start_at)
    .bind(end_at)
    .fetch_one(&mut *tx)
    .await?;

    // Creator auto-going
    sqlx::query("INSERT INTO event_responses (event_id, user_id, response) VALUES ($1, $2, 'going')")
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

    let event = sqlx::query_as::<_, EventRow>(
        r#"
        UPDATE events SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            location = COALESCE($4, location),
            cover = COALESCE($5, cover),
            start_at = COALESCE($6, start_at),
            end_at = COALESCE($7, end_at)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.location)
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
        return Err(ApiError::BadRequest("Invalid response. Use: going, interested, not_going".into()));
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
        SELECT e.id, e.name, e.cover, e.start_at, e.end_at,
            (SELECT COUNT(*) FROM event_responses WHERE event_id = e.id AND response = 'going') AS going_count
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

    Ok(Json(json!({ "data": events, "meta": { "has_more": has_more } })))
}

pub async fn my_events(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let events = sqlx::query_as::<_, EventSummary>(
        r#"
        SELECT e.id, e.name, e.cover, e.start_at, e.end_at,
            (SELECT COUNT(*) FROM event_responses WHERE event_id = e.id AND response = 'going') AS going_count
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
        SELECT e.id, e.name, e.cover, e.start_at, e.end_at,
            (SELECT COUNT(*) FROM event_responses WHERE event_id = e.id AND response = 'going') AS going_count
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
        SELECT e.id, e.name, e.cover, e.start_at, e.end_at,
            (SELECT COUNT(*) FROM event_responses WHERE event_id = e.id AND response = 'going') AS going_count
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

    Ok(Json(json!({ "data": events, "meta": { "has_more": has_more } })))
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
        SELECT DISTINCT e.id, e.name, e.cover, e.start_at, e.end_at,
            (SELECT COUNT(*) FROM event_responses WHERE event_id = e.id AND response = 'going') AS going_count
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

    Ok(Json(json!({ "data": events, "meta": { "has_more": has_more } })))
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

    Ok(Json(json!({ "data": attendees, "meta": { "has_more": has_more } })))
}

fn parse_datetime(s: &str) -> Result<OffsetDateTime, ApiError> {
    // Accept ISO 8601 format
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .map_err(|_| ApiError::BadRequest(format!("Invalid datetime format: {}. Use RFC3339 (e.g. 2025-12-31T18:00:00Z)", s)))
}
