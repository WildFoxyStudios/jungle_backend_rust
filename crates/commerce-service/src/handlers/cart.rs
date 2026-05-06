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
pub struct CartItemRow {
    pub id: i64,
    pub user_id: i64,
    pub product_id: i64,
    pub units: i32,
    pub product_name: String,
    pub product_price: rust_decimal::Decimal,
    pub product_image: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
struct CartItemSimple {
    pub id: i64,
    pub product_id: i64,
    pub units: i32,
    pub created_at: OffsetDateTime,
}

/// GET /v1/cart — list current user's cart items
pub async fn list_cart(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let items = sqlx::query_as::<_, CartItemRow>(
        r#"SELECT c.id, c.user_id, c.product_id, c.units,
                  COALESCE(p.name, '') AS product_name,
                  COALESCE(p.price, 0) AS product_price,
                  COALESCE(p.media->0->>'url', '') AS product_image,
                  c.created_at
           FROM shopping_cart c
           LEFT JOIN products p ON p.id = c.product_id
           WHERE c.user_id = $1 AND c.id < $2
           ORDER BY c.id DESC LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = items.len() as i64 > limit;
    let data: Vec<_> = items.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|i| i.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[derive(Debug, Deserialize)]
pub struct AddToCartRequest {
    pub product_id: i64,
    #[serde(default = "default_units")]
    pub units: i32,
}

fn default_units() -> i32 {
    1
}

/// POST /v1/cart — add item to cart
pub async fn add_to_cart(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AddToCartRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.units < 1 {
        return Err(ApiError::BadRequest("units must be at least 1".into()));
    }

    // Verify product exists
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM products WHERE id = $1 AND status = 'active')",
    )
    .bind(req.product_id)
    .fetch_one(&state.db)
    .await?;

    if !exists {
        return Err(ApiError::NotFound("Product not found".into()));
    }

    let item = sqlx::query_as::<_, CartItemSimple>(
        r#"INSERT INTO shopping_cart (user_id, product_id, units)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_id, product_id) DO UPDATE SET units = shopping_cart.units + EXCLUDED.units
           RETURNING id, product_id, units, created_at"#,
    )
    .bind(auth.user_id)
    .bind(req.product_id)
    .bind(req.units)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::Internal("Failed to add to cart".into()))?;

    Ok(Json(json!({ "data": item })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCartRequest {
    pub units: i32,
}

/// PUT /v1/cart/{id} — update cart item quantity
pub async fn update_cart_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCartRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.units < 1 {
        return Err(ApiError::BadRequest("units must be at least 1".into()));
    }

    let result = sqlx::query("UPDATE shopping_cart SET units = $3 WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .bind(req.units)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Cart item not found".into()));
    }

    Ok(Json(json!({ "data": { "units": req.units } })))
}

/// DELETE /v1/cart/{id} — remove item from cart
pub async fn remove_from_cart(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM shopping_cart WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Cart item not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

/// DELETE /v1/cart — clear entire cart
pub async fn clear_cart(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM shopping_cart WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(
        json!({ "data": { "cleared": result.rows_affected() } }),
    ))
}
