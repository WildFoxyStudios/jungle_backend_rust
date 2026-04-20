use axum::{
    extract::{Multipart, Path, Query, State},
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

const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50 MB
const ALLOWED_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp", "image/svg+xml"];
const ALLOWED_VIDEO_TYPES: &[&str] = &["video/mp4", "video/webm", "video/quicktime", "video/x-msvideo"];
const ALLOWED_AUDIO_TYPES: &[&str] = &["audio/mpeg", "audio/ogg", "audio/wav", "audio/webm", "audio/mp4"];

#[derive(Debug, Serialize, FromRow)]
pub struct MediaRow {
    pub id: i64,
    pub user_id: i64,
    pub file_url: String,
    pub file_type: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct MediaQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub file_type: Option<String>,
}

pub async fn upload_media(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name = String::new();
    let mut content_type = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        if field.name() == Some("file") {
            file_name = field
                .file_name()
                .unwrap_or("unknown")
                .to_string();
            content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;

            if data.len() as u64 > MAX_FILE_SIZE {
                return Err(ApiError::BadRequest(format!(
                    "File too large. Maximum size is {} MB",
                    MAX_FILE_SIZE / 1024 / 1024
                )));
            }

            file_data = Some(data.to_vec());
        }
    }

    let data = file_data.ok_or_else(|| ApiError::BadRequest("No file provided".into()))?;

    let file_type = classify_mime(&content_type);
    validate_mime(&content_type, &file_type)?;

    // Generate unique filename
    let ext = file_name
        .rsplit('.')
        .next()
        .unwrap_or("bin");
    let unique_name = format!(
        "{}/{}/{}.{}",
        file_type,
        auth.user_id,
        uuid::Uuid::new_v4(),
        ext
    );

    let storage = shared::storage::create_storage().await;

    let mut width: Option<i32> = None;
    let mut height: Option<i32> = None;
    let mut duration: Option<i32> = None;
    let mut thumbnail_url: Option<String> = None;

    let upload_data = if file_type == "image" {
        if let Ok(result) = crate::processing::resize_image(&data, 2048, 2048) {
            width = Some(result.width as i32);
            height = Some(result.height as i32);
            result.data
        } else {
            if let Some((w, h)) = crate::processing::get_image_dimensions(&data) {
                width = Some(w as i32);
                height = Some(h as i32);
            }
            data.clone()
        }
    } else {
        data.clone()
    };

    let file_url = storage
        .upload(&unique_name, &upload_data, &content_type)
        .await
        .map_err(|e| {
            tracing::error!("Storage upload failed: {}", e);
            e
        })?;
    let file_size = upload_data.len() as i64;

    if file_type == "image"
        && let Ok(thumb) = crate::processing::generate_thumbnail(&data, 200) {
            let thumb_key = format!("thumbnails/{}/{}.jpg", auth.user_id, uuid::Uuid::new_v4());
            if let Ok(thumb_url) = storage.upload(&thumb_key, &thumb.data, "image/jpeg").await {
                thumbnail_url = Some(thumb_url);
            }
        }

    if file_type == "video" {
        let tmp_dir = std::env::temp_dir();
        let tmp_video = tmp_dir.join(format!("vid_{}.tmp", uuid::Uuid::new_v4()));
        let tmp_thumb = tmp_dir.join(format!("thumb_{}.jpg", uuid::Uuid::new_v4()));

        if tokio::fs::write(&tmp_video, &data).await.is_ok() {
            if let Ok(dur) = crate::video::get_video_duration(&tmp_video).await {
                duration = Some(dur);
            }
            if crate::video::generate_video_thumbnail(&tmp_video, &tmp_thumb).await.is_ok() {
                if let Ok(thumb_data) = tokio::fs::read(&tmp_thumb).await {
                    let thumb_key = format!("thumbnails/{}/{}.jpg", auth.user_id, uuid::Uuid::new_v4());
                    if let Ok(url) = storage.upload(&thumb_key, &thumb_data, "image/jpeg").await {
                        thumbnail_url = Some(url);
                    }
                }
                let _ = tokio::fs::remove_file(&tmp_thumb).await;
            }
            let _ = tokio::fs::remove_file(&tmp_video).await;
        }
    }

    let media = sqlx::query_as::<_, MediaRow>(
        r#"
        INSERT INTO uploaded_media (user_id, file_url, file_type, file_name, file_size, mime_type, width, height, duration, thumbnail_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, user_id, file_url, file_type, file_name, file_size, mime_type, width, height, duration, thumbnail_url, created_at
        "#,
    )
    .bind(auth.user_id)
    .bind(&file_url)
    .bind(&file_type)
    .bind(&file_name)
    .bind(file_size)
    .bind(&content_type)
    .bind(width)
    .bind(height)
    .bind(duration)
    .bind(&thumbnail_url)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": media })))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut content_type = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        if field.name() == Some("avatar") {
            content_type = field
                .content_type()
                .unwrap_or("image/jpeg")
                .to_string();

            if !ALLOWED_IMAGE_TYPES.contains(&content_type.as_str()) {
                return Err(ApiError::BadRequest("Avatar must be an image".into()));
            }

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;

            if data.len() > 5 * 1024 * 1024 {
                return Err(ApiError::BadRequest("Avatar must be under 5 MB".into()));
            }

            file_data = Some(data.to_vec());
        }
    }

    let data = file_data.ok_or_else(|| ApiError::BadRequest("No avatar file provided".into()))?;

    let ext = mime_to_ext(&content_type);
    let unique_name = format!("avatars/{}/{}.{}", auth.user_id, uuid::Uuid::new_v4(), ext);

    let upload_data = if let Ok(result) = crate::processing::resize_image(&data, 500, 500) {
        result.data
    } else {
        data.clone()
    };

    let storage = shared::storage::create_storage().await;
    let file_url = storage.upload(&unique_name, &upload_data, &content_type).await?;

    sqlx::query("UPDATE users SET avatar = $1, updated_at = NOW() WHERE id = $2")
        .bind(&file_url)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    sqlx::query(
        "INSERT INTO uploaded_media (user_id, file_url, file_type, file_name, file_size, mime_type) VALUES ($1, $2, 'image', 'avatar', $3, $4)",
    )
    .bind(auth.user_id)
    .bind(&file_url)
    .bind(upload_data.len() as i64)
    .bind(&content_type)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "avatar": file_url } })))
}

pub async fn upload_cover(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut content_type = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        if field.name() == Some("cover") {
            content_type = field
                .content_type()
                .unwrap_or("image/jpeg")
                .to_string();

            if !ALLOWED_IMAGE_TYPES.contains(&content_type.as_str()) {
                return Err(ApiError::BadRequest("Cover must be an image".into()));
            }

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;

            if data.len() > 10 * 1024 * 1024 {
                return Err(ApiError::BadRequest("Cover must be under 10 MB".into()));
            }

            file_data = Some(data.to_vec());
        }
    }

    let data = file_data.ok_or_else(|| ApiError::BadRequest("No cover file provided".into()))?;

    let ext = mime_to_ext(&content_type);
    let unique_name = format!("covers/{}/{}.{}", auth.user_id, uuid::Uuid::new_v4(), ext);

    let upload_data = if let Ok(result) = crate::processing::resize_image(&data, 1920, 1080) {
        result.data
    } else {
        data.clone()
    };

    let storage = shared::storage::create_storage().await;
    let file_url = storage.upload(&unique_name, &upload_data, &content_type).await?;

    sqlx::query("UPDATE users SET cover = $1, updated_at = NOW() WHERE id = $2")
        .bind(&file_url)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    sqlx::query(
        "INSERT INTO uploaded_media (user_id, file_url, file_type, file_name, file_size, mime_type) VALUES ($1, $2, 'image', 'cover', $3, $4)",
    )
    .bind(auth.user_id)
    .bind(&file_url)
    .bind(upload_data.len() as i64)
    .bind(&content_type)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "cover": file_url } })))
}

pub async fn get_media(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let media = sqlx::query_as::<_, MediaRow>(
        "SELECT id, user_id, file_url, file_type, file_name, file_size, mime_type, width, height, duration, thumbnail_url, created_at FROM uploaded_media WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Media not found".into()))?;

    Ok(Json(json!({ "data": media })))
}

pub async fn delete_media(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let media = sqlx::query_as::<_, MediaRow>(
        "SELECT id, user_id, file_url, file_type, file_name, file_size, mime_type, width, height, duration, thumbnail_url, created_at FROM uploaded_media WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Media not found".into()))?;

    if media.user_id != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    // Delete file from storage
    let file_path = media.file_url.trim_start_matches('/');
    tokio::fs::remove_file(file_path).await.ok();

    sqlx::query("DELETE FROM uploaded_media WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn my_media(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<MediaQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.pagination.limit();
    let cursor = params.pagination.cursor_id();
    let fetch_limit = limit + 1;

    let rows = if let Some(ref ft) = params.file_type {
        sqlx::query_as::<_, MediaRow>(
            r#"
            SELECT id, user_id, file_url, file_type, file_name, file_size, mime_type, width, height, duration, thumbnail_url, created_at
            FROM uploaded_media
            WHERE user_id = $1 AND file_type = $2
              AND ($3::bigint IS NULL OR id < $3)
            ORDER BY id DESC LIMIT $4
            "#,
        )
        .bind(auth.user_id)
        .bind(ft)
        .bind(cursor)
        .bind(fetch_limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, MediaRow>(
            r#"
            SELECT id, user_id, file_url, file_type, file_name, file_size, mime_type, width, height, duration, thumbnail_url, created_at
            FROM uploaded_media
            WHERE user_id = $1 AND ($2::bigint IS NULL OR id < $2)
            ORDER BY id DESC LIMIT $3
            "#,
        )
        .bind(auth.user_id)
        .bind(cursor)
        .bind(fetch_limit)
        .fetch_all(&state.db)
        .await?
    };

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.id.to_string());

    Ok(Json(json!({
        "data": rows,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn classify_mime(mime: &str) -> String {
    if ALLOWED_IMAGE_TYPES.contains(&mime) {
        "image".into()
    } else if ALLOWED_VIDEO_TYPES.contains(&mime) {
        "video".into()
    } else if ALLOWED_AUDIO_TYPES.contains(&mime) {
        "audio".into()
    } else {
        "file".into()
    }
}

fn validate_mime(mime: &str, file_type: &str) -> Result<(), ApiError> {
    let allowed = match file_type {
        "image" => ALLOWED_IMAGE_TYPES.contains(&mime),
        "video" => ALLOWED_VIDEO_TYPES.contains(&mime),
        "audio" => ALLOWED_AUDIO_TYPES.contains(&mime),
        "file" => true,
        _ => false,
    };

    if !allowed {
        return Err(ApiError::BadRequest(format!("Unsupported file type: {}", mime)));
    }
    Ok(())
}

fn mime_to_ext(mime: &str) -> &str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        _ => "bin",
    }
}
