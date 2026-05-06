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
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct AlbumPost {
    pub id: i64,
    pub user_id: i64,
    pub album_name: Option<String>,
    pub content: String,
    pub media: Value,
    pub like_count: i32,
    pub comment_count: i32,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AlbumMediaRow {
    pub id: i64,
    pub post_id: i64,
    pub user_id: i64,
    pub image: String,
    pub created_at: OffsetDateTime,
}

/// GET /v1/users/{user_id}/albums — list user's photo albums
pub async fn list_user_albums(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let albums = sqlx::query_as::<_, AlbumPost>(
        r#"SELECT id, user_id, album_name, content, media, like_count, comment_count, created_at
           FROM posts
           WHERE user_id = $1 AND album_name IS NOT NULL AND album_name != ''
             AND deleted_at IS NULL AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = albums.len() as i64 > limit;
    let data: Vec<_> = albums.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|a| a.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[derive(Debug, Deserialize)]
pub struct CreateAlbumRequest {
    pub album_name: String,
    pub images: Vec<String>,
}

/// POST /v1/albums — create a new photo album
pub async fn create_album(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateAlbumRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.album_name.trim().is_empty() {
        return Err(ApiError::BadRequest("album_name is required".into()));
    }
    if req.images.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one image is required".into(),
        ));
    }

    let mut tx = state.db.begin().await?;

    // Create the album post
    let album_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO posts (user_id, album_name, post_type, privacy, media, content)
           VALUES ($1, $2, 'album', 'everyone', '[]'::jsonb, '')
           RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(req.album_name.trim())
    .fetch_one(&mut *tx)
    .await?;

    // Add album media
    for image_url in &req.images {
        sqlx::query("INSERT INTO albums_media (post_id, user_id, image) VALUES ($1, $2, $3)")
            .bind(album_id)
            .bind(auth.user_id)
            .bind(image_url.trim())
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Json(
        json!({ "data": { "id": album_id, "album_name": req.album_name.trim() } }),
    ))
}

/// POST /v1/albums/{id}/images — add images to an existing album
pub async fn add_album_images(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(album_id): Path<i64>,
    Json(req): Json<AddAlbumImagesRequest>,
) -> Result<Json<Value>, ApiError> {
    // Verify ownership
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1 AND user_id = $2 AND album_name IS NOT NULL AND deleted_at IS NULL)",
    )
    .bind(album_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !exists {
        return Err(ApiError::NotFound("Album not found".into()));
    }

    let mut count = 0i64;
    for image_url in &req.images {
        sqlx::query("INSERT INTO albums_media (post_id, user_id, image) VALUES ($1, $2, $3)")
            .bind(album_id)
            .bind(auth.user_id)
            .bind(image_url.trim())
            .execute(&state.db)
            .await?;
        count += 1;
    }

    Ok(Json(json!({ "data": { "added": count } })))
}

#[derive(Debug, Deserialize)]
pub struct AddAlbumImagesRequest {
    pub images: Vec<String>,
}

/// GET /v1/albums/{id}/images — list images in an album
pub async fn list_album_images(
    State(state): State<AppState>,
    Path(album_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let images = sqlx::query_as::<_, AlbumMediaRow>(
        r#"SELECT id, post_id, user_id, image, created_at
           FROM albums_media
           WHERE post_id = $1 AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(album_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = images.len() as i64 > limit;
    let data: Vec<_> = images.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|i| i.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// DELETE /v1/albums/{album_id}/images/{image_id}
pub async fn delete_album_image(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((album_id, image_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    let result =
        sqlx::query("DELETE FROM albums_media WHERE id = $1 AND post_id = $2 AND user_id = $3")
            .bind(image_id)
            .bind(album_id)
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Album image not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}
