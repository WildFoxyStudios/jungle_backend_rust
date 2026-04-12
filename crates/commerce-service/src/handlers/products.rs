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
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub price: rust_decimal::Decimal,
    pub currency: Option<String>,
    pub location: Option<String>,
    pub condition: Option<String>,
    pub r#type: Option<String>,
    pub media: Option<Value>,
    pub units: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<rust_decimal::Decimal>,
    pub status: Option<String>,
    pub media: Option<Value>,
    pub units: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub category_id: Option<i64>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub rating: i16,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ProductRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub user_id: i64,
    pub page_id: Option<i64>,
    pub name: String,
    pub description: String,
    pub category_id: Option<i64>,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub location: String,
    pub condition: String,
    pub r#type: String,
    pub status: String,
    pub media: Value,
    pub units: i32,
    pub rating: Option<rust_decimal::Decimal>,
    pub review_count: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ReviewRow {
    pub id: i64,
    pub product_id: i64,
    pub user_id: i64,
    pub rating: i16,
    pub text: String,
    pub created_at: OffsetDateTime,
    pub username: String,
    pub avatar: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CategoryRow {
    pub id: i64,
    pub name_key: String,
    pub slug: Option<String>,
}

pub async fn list_products(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let products = sqlx::query_as::<_, ProductRow>(
        "SELECT * FROM products WHERE status = 'active' AND ($1::bigint IS NULL OR id < $1) ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = products.len() as i64 > limit;
    let products: Vec<_> = products.into_iter().take(limit as usize).collect();
    let next_cursor = products.last().map(|p| p.id.to_string());

    Ok(Json(json!({ "data": products, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn create_product(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let product = sqlx::query_as::<_, ProductRow>(
        r#"
        INSERT INTO products (user_id, name, description, category_id, price, currency, location, condition, type, media, units)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.category_id)
    .bind(req.price)
    .bind(req.currency.as_deref().unwrap_or("USD"))
    .bind(req.location.as_deref().unwrap_or(""))
    .bind(req.condition.as_deref().unwrap_or("new"))
    .bind(req.r#type.as_deref().unwrap_or("sell"))
    .bind(req.media.as_ref().unwrap_or(&json!([])))
    .bind(req.units.unwrap_or(1))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": product })))
}

pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let product = sqlx::query_as::<_, ProductRow>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Product not found".into()))?;

    Ok(Json(json!({ "data": product })))
}

pub async fn update_product(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Product not found".into()))?;

    if owner != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    let product = sqlx::query_as::<_, ProductRow>(
        r#"
        UPDATE products SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            price = COALESCE($4, price),
            status = COALESCE($5, status),
            media = COALESCE($6, media),
            units = COALESCE($7, units),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.price)
    .bind(&req.status)
    .bind(&req.media)
    .bind(req.units)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": product })))
}

pub async fn delete_product(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Product not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM products WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn search_products(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let ilike = format!("%{}%", q.q);
    let limit = q.pagination.limit();

    let products = if let Some(cat_id) = q.category_id {
        sqlx::query_as::<_, ProductRow>(
            "SELECT * FROM products WHERE status = 'active' AND category_id = $1 AND name ILIKE $2 ORDER BY created_at DESC LIMIT $3",
        )
        .bind(cat_id)
        .bind(&ilike)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, ProductRow>(
            "SELECT * FROM products WHERE status = 'active' AND name ILIKE $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(&ilike)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(json!({ "data": products })))
}

pub async fn my_products(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let products = sqlx::query_as::<_, ProductRow>(
        "SELECT * FROM products WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": products })))
}

pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let cats = sqlx::query_as::<_, CategoryRow>(
        "SELECT id, name_key, slug FROM categories WHERE type = 'product' AND active = TRUE ORDER BY sort_order",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": cats })))
}

pub async fn list_reviews(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let reviews = sqlx::query_as::<_, ReviewRow>(
        r#"
        SELECT pr.id, pr.product_id, pr.user_id, pr.rating, pr.text, pr.created_at,
            u.username, u.avatar
        FROM product_reviews pr JOIN users u ON u.id = pr.user_id
        WHERE pr.product_id = $1
        ORDER BY pr.created_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": reviews })))
}

#[derive(Debug, Deserialize)]
pub struct NearbyProductsQuery {
    pub lat: f64,
    pub lng: f64,
    pub radius_km: Option<f64>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// POST /v1/products/nearby — find products near a location using Haversine
pub async fn nearby_products(
    State(state): State<AppState>,
    Json(req): Json<NearbyProductsQuery>,
) -> Result<Json<Value>, ApiError> {
    let radius = req.radius_km.unwrap_or(50.0);
    let limit = req.pagination.limit();

    let products = sqlx::query_as::<_, ProductRow>(
        r#"
        SELECT p.* FROM products p
        JOIN users u ON u.id = p.user_id
        WHERE p.status = 'active'
          AND u.lat IS NOT NULL AND u.lng IS NOT NULL
          AND (6371.0 * acos(
                cos(radians($1)) * cos(radians(u.lat))
                * cos(radians(u.lng) - radians($2))
                + sin(radians($1)) * sin(radians(u.lat))
          )) <= $3
        ORDER BY (6371.0 * acos(
                cos(radians($1)) * cos(radians(u.lat))
                * cos(radians(u.lng) - radians($2))
                + sin(radians($1)) * sin(radians(u.lat))
        )) ASC
        LIMIT $4
        "#,
    )
    .bind(req.lat)
    .bind(req.lng)
    .bind(radius)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": products })))
}

pub async fn add_review(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReviewRequest>,
) -> Result<Json<Value>, ApiError> {
    if !(1..=5).contains(&req.rating) {
        return Err(ApiError::BadRequest("Rating must be 1-5".into()));
    }

    sqlx::query(
        r#"
        INSERT INTO product_reviews (product_id, user_id, rating, text)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (product_id, user_id)
        DO UPDATE SET rating = EXCLUDED.rating, text = EXCLUDED.text
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(req.rating)
    .bind(req.text.as_deref().unwrap_or(""))
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        UPDATE products SET
            rating = (SELECT AVG(rating)::DECIMAL(3,2) FROM product_reviews WHERE product_id = $1),
            review_count = (SELECT COUNT(*) FROM product_reviews WHERE product_id = $1)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "reviewed": true } })))
}
