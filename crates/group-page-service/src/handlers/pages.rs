use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePageRequest {
    #[validate(length(min = 3, max = 32))]
    pub page_name: String,
    #[validate(length(min = 1, max = 100))]
    pub page_title: String,
    pub about: Option<String>,
    pub category_id: Option<i64>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub company: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePageRequest {
    pub page_title: Option<String>,
    pub about: Option<String>,
    pub category_id: Option<i64>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub company: Option<String>,
    pub social_links: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct RateRequest {
    pub rating: i16,
    pub review: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Serialize, FromRow)]
pub struct PageRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub user_id: i64,
    pub page_name: String,
    pub page_title: String,
    pub avatar: String,
    pub cover: String,
    pub about: String,
    pub category_id: Option<i64>,
    pub website: String,
    pub phone: String,
    pub address: String,
    pub company: String,
    pub is_verified: bool,
    pub is_boosted: bool,
    pub active: bool,
    pub rating: Option<rust_decimal::Decimal>,
    pub rating_count: Option<i32>,
    pub like_count: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct PageSummary {
    pub id: i64,
    pub page_name: String,
    pub page_title: String,
    pub avatar: String,
    pub is_verified: bool,
    pub like_count: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserSummary {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CategoryRow {
    pub id: i64,
    pub name_key: String,
    pub slug: Option<String>,
    pub parent_id: Option<i64>,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn create_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreatePageRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let page = sqlx::query_as::<_, PageRow>(
        r#"
        INSERT INTO pages (user_id, page_name, page_title, about, category_id, website, phone, address, company)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.page_name)
    .bind(&req.page_title)
    .bind(req.about.as_deref().unwrap_or(""))
    .bind(req.category_id)
    .bind(req.website.as_deref().unwrap_or(""))
    .bind(req.phone.as_deref().unwrap_or(""))
    .bind(req.address.as_deref().unwrap_or(""))
    .bind(req.company.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    // Auto-add creator as admin
    sqlx::query("INSERT INTO page_admins (page_id, user_id) VALUES ($1, $2)")
        .bind(page.id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": page })))
}

pub async fn get_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let page = sqlx::query_as::<_, PageRow>("SELECT * FROM pages WHERE page_name = $1 AND active = TRUE")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Page not found".into()))?;

    Ok(Json(json!({ "data": page })))
}

pub async fn update_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePageRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_page_admin(&state, id, auth.user_id).await?;

    let page = sqlx::query_as::<_, PageRow>(
        r#"
        UPDATE pages SET
            page_title = COALESCE($2, page_title),
            about = COALESCE($3, about),
            category_id = COALESCE($4, category_id),
            website = COALESCE($5, website),
            phone = COALESCE($6, phone),
            address = COALESCE($7, address),
            company = COALESCE($8, company),
            social_links = COALESCE($9, social_links),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.page_title)
    .bind(&req.about)
    .bind(req.category_id)
    .bind(&req.website)
    .bind(&req.phone)
    .bind(&req.address)
    .bind(&req.company)
    .bind(&req.social_links)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": page })))
}

pub async fn delete_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM pages WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Page not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("UPDATE pages SET active = FALSE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn like_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("INSERT INTO page_likes (page_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE pages SET like_count = (SELECT COUNT(*) FROM page_likes WHERE page_id = $1) WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    let _ = state.event_bus.publish(&DomainEvent::PageLiked {
        page_id: id,
        user_id: auth.user_id,
    }).await;

    Ok(Json(json!({ "data": { "liked": true } })))
}

pub async fn unlike_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM page_likes WHERE page_id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE pages SET like_count = (SELECT COUNT(*) FROM page_likes WHERE page_id = $1) WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "liked": false } })))
}

pub async fn rate_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<RateRequest>,
) -> Result<Json<Value>, ApiError> {
    if !(1..=5).contains(&req.rating) {
        return Err(ApiError::BadRequest("Rating must be 1-5".into()));
    }

    sqlx::query(
        r#"
        INSERT INTO page_ratings (page_id, user_id, rating, review)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (page_id, user_id)
        DO UPDATE SET rating = EXCLUDED.rating, review = EXCLUDED.review
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(req.rating)
    .bind(req.review.as_deref().unwrap_or(""))
    .execute(&state.db)
    .await?;

    // Recalculate page average
    sqlx::query(
        r#"
        UPDATE pages SET
            rating = (SELECT AVG(rating)::DECIMAL(3,2) FROM page_ratings WHERE page_id = $1),
            rating_count = (SELECT COUNT(*) FROM page_ratings WHERE page_id = $1)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "rated": true } })))
}

pub async fn page_likers(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let users = sqlx::query_as::<_, UserSummary>(
        r#"
        SELECT pl.user_id, u.username, u.first_name, u.last_name, u.avatar
        FROM page_likes pl JOIN users u ON u.id = pl.user_id
        WHERE pl.page_id = $1 AND ($2::bigint IS NULL OR pl.id < $2)
        ORDER BY pl.id DESC LIMIT $3
        "#,
    )
    .bind(id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = users.len() as i64 > limit;
    let users: Vec<_> = users.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": users, "meta": { "has_more": has_more } })))
}

pub async fn list_admins(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let admins = sqlx::query_as::<_, UserSummary>(
        r#"
        SELECT pa.user_id, u.username, u.first_name, u.last_name, u.avatar
        FROM page_admins pa JOIN users u ON u.id = pa.user_id
        WHERE pa.page_id = $1
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": admins })))
}

pub async fn add_admin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    verify_page_admin(&state, id, auth.user_id).await?;

    let user_id = body["user_id"].as_i64().ok_or_else(|| ApiError::BadRequest("user_id required".into()))?;

    sqlx::query("INSERT INTO page_admins (page_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "added": true } })))
}

pub async fn remove_admin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    // Only page owner can remove admins
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM pages WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Page not found".into()))?;

    if owner != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM page_admins WHERE page_id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "removed": true } })))
}

pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let cats = sqlx::query_as::<_, CategoryRow>(
        "SELECT id, name_key, slug, parent_id FROM categories WHERE type = 'page' AND active = TRUE ORDER BY sort_order",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": cats })))
}

pub async fn search_pages(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let ilike = format!("%{}%", q.q);
    let limit = q.pagination.limit();

    let pages = sqlx::query_as::<_, PageSummary>(
        "SELECT id, page_name, page_title, avatar, is_verified, like_count FROM pages WHERE active = TRUE AND (page_name ILIKE $1 OR page_title ILIKE $1) ORDER BY like_count DESC LIMIT $2",
    )
    .bind(&ilike)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": pages })))
}

pub async fn suggested_pages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let pages = sqlx::query_as::<_, PageSummary>(
        r#"
        SELECT id, page_name, page_title, avatar, is_verified, like_count
        FROM pages
        WHERE active = TRUE AND id NOT IN (SELECT page_id FROM page_likes WHERE user_id = $1)
          AND user_id != $1
        ORDER BY like_count DESC LIMIT 20
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": pages })))
}

pub async fn my_pages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let pages = sqlx::query_as::<_, PageSummary>(
        "SELECT id, page_name, page_title, avatar, is_verified, like_count FROM pages WHERE user_id = $1 AND active = TRUE ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": pages })))
}

pub async fn liked_pages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let pages = sqlx::query_as::<_, PageSummary>(
        r#"
        SELECT p.id, p.page_name, p.page_title, p.avatar, p.is_verified, p.like_count
        FROM pages p JOIN page_likes pl ON pl.page_id = p.id
        WHERE pl.user_id = $1 AND p.active = TRUE
        ORDER BY pl.created_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": pages })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn verify_page_admin(state: &AppState, page_id: i64, user_id: i64) -> Result<(), ApiError> {
    let is_owner = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pages WHERE id = $1 AND user_id = $2)",
    )
    .bind(page_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    if is_owner {
        return Ok(());
    }

    let is_admin = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM page_admins WHERE page_id = $1 AND user_id = $2)",
    )
    .bind(page_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_admin {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}
