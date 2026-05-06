use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserSearchResult {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub is_pro: i16,
}

pub async fn search_users(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Query(params): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query = params.q.trim();
    if query.is_empty() || query.len() < 2 {
        return Ok(Json(serde_json::json!({ "data": [] })));
    }

    let ilike = format!("%{}%", query);
    let starts = format!("{}%", query);
    let limit = params.pagination.limit();

    let users = sqlx::query_as::<_, UserSearchResult>(
        r#"SELECT id, uuid, username, first_name, last_name, avatar, is_verified, is_pro
           FROM users
           WHERE deleted_at IS NULL AND is_active = TRUE
             AND (username ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1)
           ORDER BY
             CASE WHEN LOWER(username) = LOWER($3) THEN 0
                  WHEN LOWER(username) LIKE LOWER($2) THEN 1
                  ELSE 2 END,
             is_verified DESC, is_pro DESC
           LIMIT $4"#,
    )
    .bind(&ilike)
    .bind(&starts)
    .bind(query)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "data": users })))
}

#[derive(Debug, Serialize, FromRow)]
pub struct ProfessionalSearchResult {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub is_pro: i16,
    pub working: String,
    pub school: String,
    pub about: String,
    pub address: String,
    pub city: String,
    pub website: String,
}

#[derive(Debug, Deserialize)]
pub struct ProfessionalSearchQuery {
    pub q: String,
    pub location: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// GET /v1/search/professional — LinkedIn-style search with rich profile fields.
/// Matches against username, name, working title, school, and about.
/// Optional `location` narrows by city/address.
pub async fn search_professionals(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Query(params): Query<ProfessionalSearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query = params.q.trim();
    if query.len() < 2 {
        return Ok(Json(serde_json::json!({ "data": [] })));
    }

    let ilike = format!("%{}%", query);
    let limit = params.pagination.limit();
    let location_filter = params
        .location
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s));

    let users = sqlx::query_as::<_, ProfessionalSearchResult>(
        r#"SELECT id, username, first_name, last_name, avatar, is_verified, is_pro,
                  working, school, about, address, city, website
           FROM users
           WHERE deleted_at IS NULL AND is_active = TRUE
             AND (
                  username ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1
                  OR working ILIKE $1 OR school ILIKE $1 OR about ILIKE $1
             )
             AND ($2::text IS NULL OR city ILIKE $2 OR address ILIKE $2)
           ORDER BY is_verified DESC, is_pro DESC, id DESC
           LIMIT $3"#,
    )
    .bind(&ilike)
    .bind(&location_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "data": users })))
}

pub async fn suggestions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let users = sqlx::query_as::<_, UserSearchResult>(
        r#"SELECT id, uuid, username, first_name, last_name, avatar, is_verified, is_pro
           FROM users
           WHERE deleted_at IS NULL AND is_active = TRUE
             AND id != $1
             AND id NOT IN (SELECT following_id FROM follows WHERE follower_id = $1 AND status = 'active')
             AND id NOT IN (SELECT blocked_id FROM blocks WHERE blocker_id = $1)
           ORDER BY RANDOM()
           LIMIT 10"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "data": users })))
}

/// GET /v1/mentions?q=john — @mention autocomplete (PHP: mention.php)
#[derive(Debug, Deserialize)]
pub struct MentionQuery {
    pub q: String,
}

pub async fn mention_search(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<MentionQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ilike = format!("{}%", params.q.trim());

    let users = sqlx::query_as::<_, UserSearchResult>(
        r#"SELECT id, username, first_name, last_name, avatar, is_verified, is_pro
           FROM users
           WHERE deleted_at IS NULL AND is_active = TRUE
             AND (username ILIKE $1 OR first_name ILIKE $1)
           ORDER BY is_verified DESC, username
           LIMIT 10"#,
    )
    .bind(&ilike)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "data": users })))
}

/// GET /v1/activities — Current user's activity feed (PHP: activities.php)
#[derive(Debug, Serialize, FromRow)]
pub struct ActivityRow {
    pub id: i64,
    pub activity_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub created_at: OffsetDateTime,
}

pub async fn list_my_activities(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, ActivityRow>(
        "SELECT id, activity_type, target_type, target_id, created_at
         FROM activities WHERE user_id = $1 AND id < $2 ORDER BY id DESC LIMIT $3",
    )
    .bind(auth.user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(
        serde_json::json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } }),
    ))
}

/// PUT /v1/users/me/location — Save user GPS coordinates (PHP: save_user_location.php)
#[derive(Debug, Deserialize)]
pub struct LocationRequest {
    pub lat: f64,
    pub lng: f64,
}

pub async fn update_location(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<LocationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE users SET lat = $1, lng = $2 WHERE id = $3")
        .bind(req.lat)
        .bind(req.lng)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "data": { "updated": true } })))
}
