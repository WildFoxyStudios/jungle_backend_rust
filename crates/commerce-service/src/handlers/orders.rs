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

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub product_id: i64,
    pub quantity: Option<i32>,
    pub address: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct OrderRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub buyer_id: i64,
    pub seller_id: i64,
    pub product_id: i64,
    pub quantity: i32,
    pub total_price: rust_decimal::Decimal,
    pub status: String,
    pub address: Option<Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub async fn create_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<Value>, ApiError> {
    let product = sqlx::query_as::<_, super::products::ProductRow>(
        "SELECT * FROM products WHERE id = $1 AND status = 'active'",
    )
    .bind(req.product_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Product not found".into()))?;

    if product.user_id == auth.user_id {
        return Err(ApiError::BadRequest("Cannot buy your own product".into()));
    }

    let quantity = req.quantity.unwrap_or(1);
    if quantity > product.units {
        return Err(ApiError::BadRequest("Insufficient stock".into()));
    }

    let total = product.price * rust_decimal::Decimal::from(quantity);

    let mut tx = state.db.begin().await?;

    let order = sqlx::query_as::<_, OrderRow>(
        r#"
        INSERT INTO orders (buyer_id, seller_id, product_id, quantity, total_price, address)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(product.user_id)
    .bind(req.product_id)
    .bind(quantity)
    .bind(total)
    .bind(&req.address)
    .fetch_one(&mut *tx)
    .await?;

    // Decrement stock
    sqlx::query("UPDATE products SET units = units - $1 WHERE id = $2")
        .bind(quantity)
        .bind(req.product_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(json!({ "data": order })))
}

pub async fn get_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    if order.buyer_id != auth.user_id && order.seller_id != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    Ok(Json(json!({ "data": order })))
}

pub async fn my_orders(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let orders = sqlx::query_as::<_, OrderRow>(
        "SELECT * FROM orders WHERE buyer_id = $1 AND ($2::bigint IS NULL OR id < $2) ORDER BY id DESC LIMIT $3",
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = orders.len() as i64 > limit;
    let orders: Vec<_> = orders.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": orders, "meta": { "has_more": has_more } })))
}

pub async fn my_sales(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let orders = sqlx::query_as::<_, OrderRow>(
        "SELECT * FROM orders WHERE seller_id = $1 AND ($2::bigint IS NULL OR id < $2) ORDER BY id DESC LIMIT $3",
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = orders.len() as i64 > limit;
    let orders: Vec<_> = orders.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": orders, "meta": { "has_more": has_more } })))
}

pub async fn update_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<Value>, ApiError> {
    let valid = ["pending", "confirmed", "shipped", "delivered", "cancelled", "refunded"];
    if !valid.contains(&req.status.as_str()) {
        return Err(ApiError::BadRequest(format!("Invalid status. Use: {}", valid.join(", "))));
    }

    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    if order.seller_id != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("UPDATE orders SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(&req.status)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "status": req.status } })))
}

/// GET /v1/orders/{id}/tracking — Get order tracking info (PHP: products.php `tracking` action)
pub async fn get_order_tracking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    if order.buyer_id != auth.user_id && order.seller_id != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("Access denied".into()));
    }

    let s = order.status.as_str();
    let confirmed = matches!(s, "confirmed" | "shipped" | "delivered" | "refund_requested" | "refunded");
    let shipped   = matches!(s, "shipped" | "delivered" | "refund_requested" | "refunded");
    let delivered = matches!(s, "delivered" | "refund_requested" | "refunded");

    Ok(Json(json!({
        "data": {
            "order_id": order.id,
            "status": order.status,
            "created_at": order.created_at,
            "updated_at": order.updated_at,
            "tracking_events": [
                { "status": "pending",   "label": "Order Placed", "completed": true },
                { "status": "confirmed", "label": "Confirmed",    "completed": confirmed },
                { "status": "shipped",   "label": "Shipped",      "completed": shipped },
                { "status": "delivered", "label": "Delivered",    "completed": delivered }
            ]
        }
    })))
}

/// POST /v1/orders/{id}/refund — Request order refund (PHP: products.php `refund` action)
#[derive(Debug, Deserialize)]
pub struct RefundOrderRequest {
    pub reason: String,
}

pub async fn request_order_refund(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<RefundOrderRequest>,
) -> Result<Json<Value>, ApiError> {
    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    if order.buyer_id != auth.user_id {
        return Err(ApiError::Forbidden("Only the buyer can request a refund".into()));
    }

    if !["delivered", "shipped"].contains(&order.status.as_str()) {
        return Err(ApiError::BadRequest(
            "Refund can only be requested for delivered or shipped orders".into(),
        ));
    }

    if req.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("reason is required".into()));
    }

    sqlx::query("UPDATE orders SET status = 'refund_requested', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "requested": true, "order_id": id } })))
}
