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

#[derive(Debug, Serialize, FromRow)]
pub struct GiftRow {
    pub id: i64,
    pub category_id: Option<i64>,
    pub name: String,
    pub image: String,
    pub price: rust_decimal::Decimal,
    pub is_active: bool,
}

pub async fn list_gifts(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let gifts = sqlx::query_as::<_, GiftRow>(
        "SELECT id, category_id, name, image, price, is_active FROM gifts WHERE is_active = TRUE ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": gifts })))
}

pub async fn list_gift_categories(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let cats = sqlx::query_as::<_, (i64, String, i32)>(
        "SELECT id, name, sort_order FROM gift_categories WHERE is_active = TRUE ORDER BY sort_order ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = cats
        .into_iter()
        .map(|(id, name, sort)| json!({ "id": id, "name": name, "sort_order": sort }))
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct SendGiftRequest {
    pub gift_id: i64,
    pub message: Option<String>,
}

pub async fn send_gift(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(recipient_id): Path<i64>,
    Json(req): Json<SendGiftRequest>,
) -> Result<Json<Value>, ApiError> {
    // Check gift exists and get price
    let gift = sqlx::query_as::<_, (i64, rust_decimal::Decimal)>(
        "SELECT id, price FROM gifts WHERE id = $1 AND is_active = TRUE",
    )
    .bind(req.gift_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Gift not found".into()))?;

    let price = gift.1;

    // Deduct from wallet
    let result = sqlx::query(
        "UPDATE users SET wallet = wallet - $1 WHERE id = $2 AND wallet >= $1",
    )
    .bind(price)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::BadRequest("Insufficient wallet balance".into()));
    }

    // Record the gift
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO user_gifts (sender_id, recipient_id, gift_id, message) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(recipient_id)
    .bind(req.gift_id)
    .bind(&req.message)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id, "price_deducted": price } })))
}

pub async fn my_received_gifts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, i64, String, String, String, Option<String>, OffsetDateTime)>(
        r#"SELECT ug.id, ug.sender_id, u.username, g.name, g.image, ug.message, ug.created_at
        FROM user_gifts ug
        JOIN users u ON u.id = ug.sender_id
        JOIN gifts g ON g.id = ug.gift_id
        WHERE ug.recipient_id = $1
        ORDER BY ug.created_at DESC
        LIMIT 50"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, sender_id, username, gift_name, gift_image, message, created_at)| {
            json!({
                "id": id, "sender_id": sender_id, "sender_username": username,
                "gift_name": gift_name, "gift_image": gift_image,
                "message": message, "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

// ── Stickers ──

#[derive(Debug, Serialize, FromRow)]
pub struct StickerPackRow {
    pub id: i64,
    pub name: String,
    pub preview_url: Option<String>,
    pub is_premium: bool,
    pub price: rust_decimal::Decimal,
    pub is_active: bool,
}

pub async fn list_sticker_packs(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let packs = sqlx::query_as::<_, StickerPackRow>(
        "SELECT id, name, preview_url, is_premium, price, is_active FROM sticker_packs WHERE is_active = TRUE ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": packs })))
}

pub async fn get_sticker_pack(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let pack = sqlx::query_as::<_, StickerPackRow>(
        "SELECT id, name, preview_url, is_premium, price, is_active FROM sticker_packs WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Sticker pack not found".into()))?;

    let stickers = sqlx::query_as::<_, (i64, String, i32)>(
        "SELECT id, image_url, sort_order FROM stickers WHERE pack_id = $1 ORDER BY sort_order ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let sticker_data: Vec<Value> = stickers
        .into_iter()
        .map(|(sid, url, sort)| json!({ "id": sid, "image_url": url, "sort_order": sort }))
        .collect();

    Ok(Json(json!({ "data": { "pack": pack, "stickers": sticker_data } })))
}

pub async fn purchase_sticker_pack(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let pack = sqlx::query_as::<_, (rust_decimal::Decimal, bool)>(
        "SELECT price, is_premium FROM sticker_packs WHERE id = $1 AND is_active = TRUE",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Sticker pack not found".into()))?;

    if pack.1 {
        // Premium — deduct from wallet
        let result = sqlx::query(
            "UPDATE users SET wallet = wallet - $1 WHERE id = $2 AND wallet >= $1",
        )
        .bind(pack.0)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::BadRequest("Insufficient wallet balance".into()));
        }
    }

    sqlx::query(
        "INSERT INTO user_sticker_packs (user_id, pack_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "purchased": true } })))
}

pub async fn my_sticker_packs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let packs = sqlx::query_as::<_, StickerPackRow>(
        r#"SELECT sp.id, sp.name, sp.preview_url, sp.is_premium, sp.price, sp.is_active
        FROM sticker_packs sp
        JOIN user_sticker_packs usp ON usp.pack_id = sp.id
        WHERE usp.user_id = $1
        ORDER BY usp.purchased_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": packs })))
}
