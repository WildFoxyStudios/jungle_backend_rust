use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    Ok(())
}

// ── Gifts ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct GiftRow {
    pub id: i64,
    pub name: Option<String>,
    pub media_file: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateGiftRequest {
    pub name: Option<String>,
    pub media_file: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGiftRequest {
    pub name: Option<String>,
    pub media_file: Option<String>,
}

/// GET /v1/admin/gifts — List all gifts
pub async fn list_gifts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, GiftRow>(
        "SELECT id, name, media_file, created_at FROM gifts ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

/// POST /v1/admin/gifts — Create a new gift
pub async fn create_gift(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGiftRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, GiftRow>(
        "INSERT INTO gifts (name, media_file) VALUES ($1, $2) RETURNING id, name, media_file, created_at",
    )
    .bind(&req.name)
    .bind(&req.media_file)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": row })))
}

/// PUT /v1/admin/gifts/{id} — Update a gift
pub async fn update_gift(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateGiftRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query(
        r#"UPDATE gifts SET
            name = COALESCE($1, name),
            media_file = COALESCE($2, media_file)
        WHERE id = $3"#,
    )
    .bind(&req.name)
    .bind(&req.media_file)
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Gift not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

/// DELETE /v1/admin/gifts/{id} — Delete a gift
pub async fn delete_gift(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM gifts WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Sticker Packs ────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct StickerPackRow {
    pub id: i64,
    pub name: String,
    pub preview_url: Option<String>,
    pub price: Option<rust_decimal::Decimal>,
    pub is_free: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateStickerPackRequest {
    pub name: String,
    pub preview_url: Option<String>,
    pub price: Option<rust_decimal::Decimal>,
    pub is_free: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStickerPackRequest {
    pub name: Option<String>,
    pub preview_url: Option<String>,
    pub price: Option<rust_decimal::Decimal>,
    pub is_free: Option<bool>,
}

/// GET /v1/admin/sticker-packs — List all sticker packs
pub async fn list_sticker_packs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, StickerPackRow>(
        "SELECT id, name, preview_url, price, is_free, created_at FROM sticker_packs ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

/// POST /v1/admin/sticker-packs — Create a sticker pack
pub async fn create_sticker_pack(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateStickerPackRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, StickerPackRow>(
        r#"INSERT INTO sticker_packs (name, preview_url, price, is_free)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, preview_url, price, is_free, created_at"#,
    )
    .bind(&req.name)
    .bind(&req.preview_url)
    .bind(req.price)
    .bind(req.is_free.unwrap_or(false))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": row })))
}

/// PUT /v1/admin/sticker-packs/{id} — Update a sticker pack
pub async fn update_sticker_pack(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStickerPackRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query(
        r#"UPDATE sticker_packs SET
            name = COALESCE($1, name),
            preview_url = COALESCE($2, preview_url),
            price = COALESCE($3, price),
            is_free = COALESCE($4, is_free)
        WHERE id = $5"#,
    )
    .bind(&req.name)
    .bind(&req.preview_url)
    .bind(req.price)
    .bind(req.is_free)
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Sticker pack not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

/// DELETE /v1/admin/sticker-packs/{id} — Delete a sticker pack (cascades to stickers)
pub async fn delete_sticker_pack(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM sticker_packs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Stickers (individual items within a pack) ────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct StickerRow {
    pub id: i64,
    pub pack_id: i64,
    pub image_url: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStickerRequest {
    pub image_url: String,
    pub sort_order: Option<i32>,
}

/// GET /v1/admin/sticker-packs/{pack_id}/stickers — List stickers in a pack
pub async fn list_stickers(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(pack_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, StickerRow>(
        "SELECT id, pack_id, image_url, sort_order FROM stickers WHERE pack_id = $1 ORDER BY sort_order, id",
    )
    .bind(pack_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

/// POST /v1/admin/sticker-packs/{pack_id}/stickers — Add a sticker to a pack
pub async fn add_sticker(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(pack_id): Path<i64>,
    Json(req): Json<CreateStickerRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, StickerRow>(
        r#"INSERT INTO stickers (pack_id, image_url, sort_order)
        VALUES ($1, $2, $3)
        RETURNING id, pack_id, image_url, sort_order"#,
    )
    .bind(pack_id)
    .bind(&req.image_url)
    .bind(req.sort_order.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": row })))
}

/// DELETE /v1/admin/stickers/{id} — Remove a sticker
pub async fn delete_sticker(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM stickers WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}
