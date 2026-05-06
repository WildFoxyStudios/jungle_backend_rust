use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use super::products::ProductRow;
use sqlx::{FromRow, Row};
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub product_id: i64,
    pub quantity: Option<i32>,
    pub address: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWalletLine {
    pub product_id: i64,
    pub quantity: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWalletRequest {
    pub lines: Vec<CheckoutWalletLine>,
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
    pub payment_status: String,
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

    let _ = state.event_bus.publish(&DomainEvent::OrderCreated {
        order_id: order.id,
        buyer_id: auth.user_id,
        seller_id: product.user_id,
    }).await;

    Ok(Json(json!({ "data": order })))
}

/// POST /v1/orders/checkout-wallet — Place several marketplace lines in **one** DB transaction:
/// debit the buyer wallet once, insert each order as `wallet_paid`, decrement stock per line.
pub async fn checkout_wallet(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CheckoutWalletRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.lines.is_empty() {
        return Err(ApiError::BadRequest("No order lines".into()));
    }

    struct PreparedLine {
        product_id: i64,
        seller_id: i64,
        quantity: i32,
        total_price: rust_decimal::Decimal,
    }

    let mut tx = state.db.begin().await?;

    let balance = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(balance, 0) FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut prepared: Vec<PreparedLine> = Vec::with_capacity(req.lines.len());
    let mut grand_total = rust_decimal::Decimal::ZERO;
    let mut line_currency: Option<String> = None;

    for line in &req.lines {
        let quantity = line.quantity.unwrap_or(1).max(1);

        let product = sqlx::query_as::<_, ProductRow>(
            "SELECT * FROM products WHERE id = $1 FOR UPDATE",
        )
        .bind(line.product_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::NotFound("Product not found".into()))?;

        if product.status != "active" {
            return Err(ApiError::BadRequest("Product not available".into()));
        }

        if product.user_id == auth.user_id {
            return Err(ApiError::BadRequest("Cannot buy your own product".into()));
        }

        if quantity > product.units {
            return Err(ApiError::BadRequest("Insufficient stock".into()));
        }

        let cur = product.currency.to_uppercase();
        match &line_currency {
            None => line_currency = Some(cur),
            Some(c) if c == &cur => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Mixed currencies in one checkout are not supported".into(),
                ));
            }
        }

        let total_price = product.price * rust_decimal::Decimal::from(quantity);
        grand_total += total_price;

        prepared.push(PreparedLine {
            product_id: product.id,
            seller_id: product.user_id,
            quantity,
            total_price,
        });
    }

    if line_currency.as_deref() != Some("USD") {
        return Err(ApiError::BadRequest(
            "Wallet checkout currently supports USD-priced products only".into(),
        ));
    }

    if balance < grand_total {
        return Err(ApiError::BadRequest("Insufficient wallet balance".into()));
    }

    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
        .bind(grand_total)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    let mut ids: Vec<i64> = Vec::with_capacity(prepared.len());
    let mut orders_out: Vec<OrderRow> = Vec::with_capacity(prepared.len());

    for p in prepared {
        let order = sqlx::query_as::<_, OrderRow>(
            r#"
            INSERT INTO orders (buyer_id, seller_id, product_id, quantity, total_price, address, payment_status)
            VALUES ($1, $2, $3, $4, $5, $6, 'wallet_paid')
            RETURNING *
            "#,
        )
        .bind(auth.user_id)
        .bind(p.seller_id)
        .bind(p.product_id)
        .bind(p.quantity)
        .bind(p.total_price)
        .bind(&req.address)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE products SET units = units - $1 WHERE id = $2")
            .bind(p.quantity)
            .bind(p.product_id)
            .execute(&mut *tx)
            .await?;

        ids.push(order.id);
        orders_out.push(order);
    }

    tx.commit().await?;

    Ok(Json(json!({
        "data": {
            "ids": ids,
            "orders": orders_out
        }
    })))
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

    Ok(Json(
        json!({ "data": orders, "meta": { "has_more": has_more } }),
    ))
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

    Ok(Json(
        json!({ "data": orders, "meta": { "has_more": has_more } }),
    ))
}

pub async fn update_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<Value>, ApiError> {
    let valid = [
        "pending",
        "confirmed",
        "shipped",
        "delivered",
        "cancelled",
        "refunded",
    ];
    if !valid.contains(&req.status.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid status. Use: {}",
            valid.join(", ")
        )));
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

    let _ = state.event_bus.publish(&DomainEvent::OrderStatusChanged {
        order_id: id,
        buyer_id: order.buyer_id,
        seller_id: order.seller_id,
        new_status: req.status.clone(),
    }).await;

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
    let confirmed = matches!(
        s,
        "confirmed" | "shipped" | "delivered" | "refund_requested" | "refunded"
    );
    let shipped = matches!(s, "shipped" | "delivered" | "refund_requested" | "refunded");
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
        return Err(ApiError::Forbidden(
            "Only the buyer can request a refund".into(),
        ));
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

    Ok(Json(
        json!({ "data": { "requested": true, "order_id": id } }),
    ))
}

// ── Order Disputes ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateDisputeRequest {
    pub reason: String,
}

pub async fn create_order_dispute(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<i64>,
    Json(body): Json<CreateDisputeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let order = sqlx::query("SELECT buyer_id FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("DB error".into())
        })?
        .ok_or(ApiError::NotFound("Order not found".into()))?;

    let buyer_id: i64 = order.get("buyer_id");
    if buyer_id != auth.user_id {
        return Err(ApiError::Forbidden(
            "Only the buyer can open a dispute".into(),
        ));
    }

    let row = sqlx::query(
        "INSERT INTO order_disputes (order_id, buyer_id, reason, status, created_at)
         VALUES ($1, $2, $3, 'open', NOW())
         ON CONFLICT (order_id) DO UPDATE SET reason = $3, status = 'open'
         RETURNING id, status, created_at",
    )
    .bind(order_id)
    .bind(auth.user_id)
    .bind(&body.reason)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "status": row.get::<String, _>("status"),
        "created_at": row.get::<String, _>("created_at"),
    })))
}

// GET /v1/users/me/disputes
pub async fn list_my_disputes(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let rows = sqlx::query(
        "SELECT d.id, d.order_id, d.reason, d.status, d.admin_notes, d.created_at, d.resolved_at
         FROM order_disputes d
         WHERE d.buyer_id = $1
         ORDER BY d.created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "order_id": r.get::<i64, _>("order_id"),
                "reason": r.get::<String, _>("reason"),
                "status": r.get::<String, _>("status"),
                "admin_notes": r.get::<Option<String>, _>("admin_notes"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(items))
}
