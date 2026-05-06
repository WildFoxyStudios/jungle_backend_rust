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
pub struct CreateMovieRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub cover: Option<String>,
    pub video_url: String,
    pub iframe_url: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub country: Option<String>,
    pub stars: Option<String>,
    pub producer: Option<String>,
    pub release_year: Option<i32>,
    pub duration: Option<String>,
    pub quality: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMovieRequest {
    pub name: Option<String>,
    pub cover: Option<String>,
    pub video_url: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MovieRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub cover: String,
    pub video_url: String,
    pub iframe_url: String,
    pub description: String,
    pub genre: String,
    pub country: String,
    pub stars: String,
    pub producer: String,
    pub release_year: Option<i32>,
    pub duration: String,
    pub quality: String,
    pub view_count: i32,
    pub is_approved: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct ListMoviesQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub genre: Option<String>,
    pub country: Option<String>,
}

impl ListMoviesQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
    fn cursor_id(&self) -> Option<i64> {
        self.cursor.as_ref().and_then(|c| c.parse::<i64>().ok())
    }
}

pub async fn list_movies(
    State(state): State<AppState>,
    Query(params): Query<ListMoviesQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let genre = params.genre.as_deref().filter(|s| !s.is_empty());
    let country = params.country.as_deref().filter(|s| !s.is_empty());

    let movies = sqlx::query_as::<_, MovieRow>(
        r#"
        SELECT * FROM movies
        WHERE is_approved = TRUE
          AND ($1::bigint IS NULL OR id < $1)
          AND ($2::text IS NULL OR LOWER(genre) = LOWER($2))
          AND ($3::text IS NULL OR LOWER(country) = LOWER($3))
        ORDER BY id DESC LIMIT $4
        "#,
    )
    .bind(cursor)
    .bind(genre)
    .bind(country)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = movies.len() as i64 > limit;
    let movies: Vec<_> = movies.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": movies, "meta": { "has_more": has_more } }),
    ))
}

pub async fn create_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateMovieRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let movie = sqlx::query_as::<_, MovieRow>(
        r#"
        INSERT INTO movies (user_id, name, cover, video_url, iframe_url, description, genre, country, stars, producer, release_year, duration, quality)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(req.cover.as_deref().unwrap_or(""))
    .bind(&req.video_url)
    .bind(req.iframe_url.as_deref().unwrap_or(""))
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.genre.as_deref().unwrap_or(""))
    .bind(req.country.as_deref().unwrap_or(""))
    .bind(req.stars.as_deref().unwrap_or(""))
    .bind(req.producer.as_deref().unwrap_or(""))
    .bind(req.release_year)
    .bind(req.duration.as_deref().unwrap_or(""))
    .bind(req.quality.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": movie })))
}

pub async fn get_movie(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE movies SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    let movie =
        sqlx::query_as::<_, MovieRow>("SELECT * FROM movies WHERE id = $1 AND is_approved = TRUE")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::NotFound("Movie not found".into()))?;

    Ok(Json(json!({ "data": movie })))
}

pub async fn update_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateMovieRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM movies WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Movie not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let movie = sqlx::query_as::<_, MovieRow>(
        r#"
        UPDATE movies SET
            name = COALESCE($2, name),
            cover = COALESCE($3, cover),
            video_url = COALESCE($4, video_url),
            description = COALESCE($5, description),
            genre = COALESCE($6, genre)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.cover)
    .bind(&req.video_url)
    .bind(&req.description)
    .bind(&req.genre)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": movie })))
}

// ── Movie Comments & Reactions ──

pub async fn list_movie_comments(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let rows = sqlx::query_as::<_, (i64, i64, String, String, Option<String>, OffsetDateTime)>(
        r#"
        SELECT c.id, c.user_id, c.text, u.username, u.avatar, c.created_at
        FROM comments c JOIN users u ON u.id = c.user_id
        WHERE c.target_type = 'movie' AND c.target_id = $1 AND c.parent_id IS NULL
          AND ($2::bigint IS NULL OR c.id < $2)
        ORDER BY c.id DESC LIMIT $3
        "#,
    )
    .bind(id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<Value> = rows
        .into_iter()
        .take(limit as usize)
        .map(|(cid, uid, text, username, avatar, created_at)| {
            json!({
                "id": cid, "user_id": uid, "text": text,
                "username": username, "avatar": avatar,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(
        json!({ "data": data, "meta": { "has_more": has_more } }),
    ))
}

pub async fn add_movie_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CommentRequest>,
) -> Result<Json<Value>, ApiError> {
    let cid = sqlx::query_scalar::<_, i64>(
        "INSERT INTO comments (user_id, target_type, target_id, text) VALUES ($1, 'movie', $2, $3) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&req.text)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": cid } })))
}

pub async fn react_to_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReactionRequest>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        r#"INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
        VALUES ($1, 'movie', $2, $3)
        ON CONFLICT (user_id, target_type, target_id) DO UPDATE SET reaction_type = $3"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&req.reaction)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "reacted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct CommentRequest {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct ReactionRequest {
    pub reaction: String,
}

pub async fn delete_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM movies WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Movie not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM movies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

/// POST /v1/movies/{id}/watch — Increment movie view count (PHP: watch.php)
pub async fn watch_movie(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query("UPDATE movies SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(ApiError::NotFound("Movie not found".into()));
    }

    Ok(Json(json!({ "data": { "watched": true } })))
}

// ── Watch Progress & Watch Later (Phases 13-15) ──

#[derive(Deserialize)]
pub struct WatchProgressRequest {
    pub position_ms: i64,
    pub duration_ms: Option<i64>,
}

pub async fn update_watch_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(movie_id): Path<i64>,
    Json(body): Json<WatchProgressRequest>,
) -> Result<Json<()>, ApiError> {
    sqlx::query(
        "INSERT INTO watch_progress (user_id, movie_id, position_ms, duration_ms, updated_at)
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (user_id, movie_id) DO UPDATE
         SET position_ms = $3, duration_ms = $4, updated_at = NOW()"
    )
    .bind(auth.user_id).bind(movie_id).bind(body.position_ms).bind(body.duration_ms)
    .execute(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(()))
}

pub async fn get_watch_progress(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query(
        "SELECT wp.movie_id, wp.position_ms, wp.duration_ms, wp.updated_at, m.title
         FROM watch_progress wp JOIN movies m ON m.id = wp.movie_id
         WHERE wp.user_id = $1 ORDER BY wp.updated_at DESC LIMIT 20"
    )
    .bind(auth.user_id)
    .fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let items: Vec<Value> = rows.iter().map(|r| {
        json!({
            "movie_id": r.get::<i64, _>("movie_id"),
            "position_ms": r.get::<i64, _>("position_ms"),
            "duration_ms": r.get::<Option<i64>, _>("duration_ms"),
            "title": r.get::<String, _>("title"),
        })
    }).collect();
    Ok(Json(items))
}

pub async fn add_to_watch_later(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(movie_id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    sqlx::query("INSERT INTO watch_later (user_id, movie_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(auth.user_id).bind(movie_id)
        .execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(()))
}
