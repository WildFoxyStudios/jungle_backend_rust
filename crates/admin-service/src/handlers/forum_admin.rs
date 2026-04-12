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

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    Ok(())
}

// ── Forum Sections ─────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ForumSectionRow {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct ForumSectionRequest {
    pub name: String,
    pub description: Option<String>,
}

pub async fn list_forum_sections(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let sections = sqlx::query_as::<_, ForumSectionRow>(
        "SELECT id, name, description FROM forum_sections ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": sections })))
}

pub async fn create_forum_section(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ForumSectionRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO forum_sections (name, description) VALUES ($1, $2) RETURNING id",
    )
    .bind(req.name.trim())
    .bind(req.description.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn update_forum_section(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ForumSectionRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE forum_sections SET name = $1, description = $2 WHERE id = $3")
        .bind(req.name.trim())
        .bind(req.description.as_deref().unwrap_or(""))
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_forum_section(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM forum_sections WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Forums ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ForumRequest {
    pub section_id: i64,
    pub name: String,
    pub description: Option<String>,
}

pub async fn create_forum(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ForumRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO forums (section_id, name, description) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(req.section_id)
    .bind(req.name.trim())
    .bind(req.description.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

// ── Forum Threads Admin ─────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ForumThreadRow {
    pub id: i64,
    pub forum_id: i64,
    pub user_id: i64,
    pub title: String,
    pub view_count: i32,
    pub reply_count: i32,
    pub created_at: OffsetDateTime,
}

pub async fn list_forum_threads(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, ForumThreadRow>(
        "SELECT id, forum_id, user_id, title, view_count, reply_count, created_at
         FROM forum_threads WHERE id < $1 ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn delete_forum_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM forum_threads WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Forum Replies Admin ─────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ForumReplyRow {
    pub id: i64,
    pub thread_id: i64,
    pub user_id: i64,
    pub content: String,
    pub created_at: OffsetDateTime,
}

pub async fn list_forum_replies(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, ForumReplyRow>(
        "SELECT id, thread_id, user_id, content, created_at
         FROM forum_replies WHERE id < $1 ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn delete_forum_reply(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM forum_replies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Movies CRUD (Admin) ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateMovieRequest {
    pub name: String,
    pub video_url: String,
    pub iframe_url: Option<String>,
    pub cover: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub country: Option<String>,
    pub stars: Option<String>,
    pub producer: Option<String>,
    pub release_year: Option<i32>,
    pub duration: Option<String>,
    pub quality: Option<String>,
    pub category_id: Option<i64>,
    pub is_approved: Option<bool>,
}

pub async fn create_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateMovieRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    if req.video_url.trim().is_empty() {
        return Err(ApiError::BadRequest("video_url is required".into()));
    }

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO movies (user_id, name, video_url, iframe_url, cover, description, genre, country,
               stars, producer, release_year, duration, quality, category_id, is_approved)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(req.name.trim())
    .bind(req.video_url.trim())
    .bind(req.iframe_url.as_deref().unwrap_or(""))
    .bind(req.cover.as_deref().unwrap_or(""))
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.genre.as_deref().unwrap_or(""))
    .bind(req.country.as_deref().unwrap_or(""))
    .bind(req.stars.as_deref().unwrap_or(""))
    .bind(req.producer.as_deref().unwrap_or(""))
    .bind(req.release_year)
    .bind(req.duration.as_deref().unwrap_or(""))
    .bind(req.quality.as_deref().unwrap_or(""))
    .bind(req.category_id)
    .bind(req.is_approved.unwrap_or(true))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn update_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CreateMovieRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query(
        r#"UPDATE movies SET name=$1, video_url=$2, iframe_url=$3, cover=$4, description=$5,
           genre=$6, country=$7, stars=$8, producer=$9, release_year=$10, duration=$11,
           quality=$12, category_id=$13, is_approved=$14, updated_at=NOW() WHERE id=$15"#,
    )
    .bind(req.name.trim())
    .bind(req.video_url.trim())
    .bind(req.iframe_url.as_deref().unwrap_or(""))
    .bind(req.cover.as_deref().unwrap_or(""))
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.genre.as_deref().unwrap_or(""))
    .bind(req.country.as_deref().unwrap_or(""))
    .bind(req.stars.as_deref().unwrap_or(""))
    .bind(req.producer.as_deref().unwrap_or(""))
    .bind(req.release_year)
    .bind(req.duration.as_deref().unwrap_or(""))
    .bind(req.quality.as_deref().unwrap_or(""))
    .bind(req.category_id)
    .bind(req.is_approved.unwrap_or(true))
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}
