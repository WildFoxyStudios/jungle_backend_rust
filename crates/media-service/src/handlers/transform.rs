//! Image transformation endpoints (rotate + crop).
//!
//! These mirror the WoWonder `rotate_image` / `crop-avatar` XHR actions: the
//! client asks the server to re-encode an already-uploaded image in place, so
//! the stored object URL stays the same and all referencing posts/avatars
//! automatically pick up the new version (with a new cache-buster via the
//! `updated_at` timestamp returned to the caller).

use axum::{
    extract::{Path, State},
    Json,
};
use image::{GenericImageView, ImageFormat};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use std::io::Cursor;

use crate::handlers::upload::MediaRow;

// ═══════════════════════════════════════════════════════════════════
// Rotate
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct RotateRequest {
    /// Degrees to rotate clockwise; must be one of 90, 180, 270 (or -90/-180/-270).
    pub degrees: i32,
}

/// POST /v1/media/{id}/rotate  body: `{ "degrees": 90 }`
pub async fn rotate_image(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<RotateRequest>,
) -> Result<Json<Value>, ApiError> {
    let normalized = ((req.degrees % 360) + 360) % 360;
    if ![0, 90, 180, 270].contains(&normalized) {
        return Err(ApiError::BadRequest(
            "degrees must be a multiple of 90".into(),
        ));
    }
    if normalized == 0 {
        // No-op, nothing to do.
        return Ok(Json(json!({ "data": { "rotated": false } })));
    }

    let media = load_own_image(&state, id, auth.user_id).await?;

    let (key, public_url) = split_key(&media.file_url)?;
    let storage = shared::storage::create_storage().await;
    let original = storage.download(&key).await?;

    let img = image::load_from_memory(&original)
        .map_err(|e| ApiError::BadRequest(format!("not a decodable image: {e}")))?;

    let rotated = match normalized {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => unreachable!(),
    };

    let (new_bytes, mime) = encode_same_format(&rotated, &media.mime_type)?;
    let (new_w, new_h) = rotated.dimensions();

    storage
        .upload(&key, &new_bytes, &mime)
        .await
        .map_err(|e| ApiError::Internal(format!("re-upload failed: {e}")))?;

    let updated_at: time::OffsetDateTime = sqlx::query_scalar(
        r#"UPDATE uploaded_media
              SET file_size = $1,
                  width    = $2,
                  height   = $3,
                  mime_type = $4,
                  updated_at = NOW()
            WHERE id = $5
        RETURNING updated_at"#,
    )
    .bind(new_bytes.len() as i64)
    .bind(new_w as i32)
    .bind(new_h as i32)
    .bind(&mime)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "id": id,
            "file_url": format!("{}/{}", public_url, key),
            "width": new_w,
            "height": new_h,
            "updated_at": updated_at,
        }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// Crop
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CropRequest {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// POST /v1/media/{id}/crop  body: `{ "x":0, "y":0, "width":512, "height":512 }`
pub async fn crop_image(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CropRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.width == 0 || req.height == 0 {
        return Err(ApiError::BadRequest("width/height must be > 0".into()));
    }
    if req.width > 8192 || req.height > 8192 {
        return Err(ApiError::BadRequest("crop dimensions too large".into()));
    }

    let media = load_own_image(&state, id, auth.user_id).await?;

    let (key, public_url) = split_key(&media.file_url)?;
    let storage = shared::storage::create_storage().await;
    let original = storage.download(&key).await?;

    let mut img = image::load_from_memory(&original)
        .map_err(|e| ApiError::BadRequest(format!("not a decodable image: {e}")))?;

    let (full_w, full_h) = img.dimensions();
    if req.x + req.width > full_w || req.y + req.height > full_h {
        return Err(ApiError::BadRequest(format!(
            "crop box ({},{},{},{}) exceeds image bounds {}x{}",
            req.x, req.y, req.width, req.height, full_w, full_h
        )));
    }

    let cropped = img.crop(req.x, req.y, req.width, req.height);
    let (new_bytes, mime) = encode_same_format(&cropped, &media.mime_type)?;

    storage
        .upload(&key, &new_bytes, &mime)
        .await
        .map_err(|e| ApiError::Internal(format!("re-upload failed: {e}")))?;

    let updated_at: time::OffsetDateTime = sqlx::query_scalar(
        r#"UPDATE uploaded_media
              SET file_size = $1,
                  width    = $2,
                  height   = $3,
                  mime_type = $4,
                  updated_at = NOW()
            WHERE id = $5
        RETURNING updated_at"#,
    )
    .bind(new_bytes.len() as i64)
    .bind(req.width as i32)
    .bind(req.height as i32)
    .bind(&mime)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "id": id,
            "file_url": format!("{}/{}", public_url, key),
            "width": req.width,
            "height": req.height,
            "updated_at": updated_at,
        }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

async fn load_own_image(
    state: &AppState,
    id: i64,
    user_id: i64,
) -> Result<MediaRow, ApiError> {
    let media = sqlx::query_as::<_, MediaRow>(
        "SELECT id, user_id, file_url, file_type, file_name, file_size, mime_type,
                width, height, duration, thumbnail_url, created_at
           FROM uploaded_media WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Media not found".into()))?;

    if media.user_id != user_id {
        return Err(ApiError::Forbidden("Not your media".into()));
    }
    if media.file_type != "image" {
        return Err(ApiError::BadRequest(
            "Only image media can be transformed".into(),
        ));
    }
    Ok(media)
}

/// Split a stored `file_url` into `(key, public_url_base)`. Works for both S3
/// absolute URLs (`https://host/bucket/path`) and local (`/uploads/path`).
fn split_key(file_url: &str) -> Result<(String, String), ApiError> {
    // Find the last occurrence of `/uploads/` or fall back to splitting at the
    // first slash after the authority.
    if let Some(pos) = file_url.rfind("/uploads/") {
        let (base, rest) = file_url.split_at(pos);
        // `rest` starts with "/uploads/..."; strip the leading slash for the key.
        return Ok((rest.trim_start_matches('/').to_string(), base.to_string()));
    }

    // Generic split — pick everything after the first triple-slash group.
    if let Some(idx) = file_url.find("://") {
        let after_scheme = &file_url[idx + 3..];
        if let Some(slash) = after_scheme.find('/') {
            let key = &after_scheme[slash + 1..];
            let base_len = idx + 3 + slash;
            return Ok((key.to_string(), file_url[..base_len].to_string()));
        }
    }

    Err(ApiError::Internal(format!(
        "cannot parse storage key from {file_url}"
    )))
}

fn encode_same_format(img: &image::DynamicImage, mime: &str) -> Result<(Vec<u8>, String), ApiError> {
    let (format, out_mime) = match mime {
        "image/png" => (ImageFormat::Png, "image/png"),
        "image/gif" => (ImageFormat::Png, "image/png"), // re-encode animated gifs as static png after transform
        "image/webp" => (ImageFormat::WebP, "image/webp"),
        _ => (ImageFormat::Jpeg, "image/jpeg"),
    };
    let mut out: Vec<u8> = Vec::with_capacity(img.as_bytes().len());
    img.write_to(&mut Cursor::new(&mut out), format)
        .map_err(|e| ApiError::Internal(format!("encode failed: {e}")))?;
    Ok((out, out_mime.to_string()))
}
