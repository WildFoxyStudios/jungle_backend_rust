use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// ── Pages Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminPageRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub category_id: Option<i64>,
    pub likes_count: i64,
    pub is_boosted: bool,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/pages — list all pages (admin only)
pub async fn list_pages(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let pages = sqlx::query_as::<_, AdminPageRow>(
        r#"SELECT id, user_id, name, category_id, likes_count, is_boosted, created_at
           FROM pages WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = pages.len() as i64 > limit;
    let data: Vec<_> = pages.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// DELETE /v1/admin/pages/{id}
pub async fn delete_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("DELETE FROM pages WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Page not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Groups Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminGroupRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub category_id: Option<i64>,
    pub privacy: String,
    pub member_count: i64,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/groups
pub async fn list_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let groups = sqlx::query_as::<_, AdminGroupRow>(
        r#"SELECT id, user_id, name, category_id, privacy, member_count, created_at
           FROM groups WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = groups.len() as i64 > limit;
    let data: Vec<_> = groups.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|g| g.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// DELETE /v1/admin/groups/{id}
pub async fn delete_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("DELETE FROM groups WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Group not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Blogs Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminBlogRow {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub category_id: Option<i64>,
    pub status: String,
    pub view_count: i64,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/blogs
pub async fn list_blogs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let blogs = sqlx::query_as::<_, AdminBlogRow>(
        r#"SELECT id, user_id, title, category_id, status, view_count, created_at
           FROM blogs WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = blogs.len() as i64 > limit;
    let data: Vec<_> = blogs.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|b| b.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// POST /v1/admin/site-blogs/{id}/approve
pub async fn approve_blog(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("UPDATE blogs SET status = 'published' WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Blog not found".into()));
    }

    Ok(Json(json!({ "data": { "approved": true } })))
}

/// DELETE /v1/admin/blogs/{id}
pub async fn delete_blog(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("DELETE FROM blogs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Blog not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Products Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminProductRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub status: String,
    pub category_id: Option<i64>,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/site-products
pub async fn list_products(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let products = sqlx::query_as::<_, AdminProductRow>(
        r#"SELECT id, user_id, name, price, currency, status, category_id, created_at
           FROM products WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = products.len() as i64 > limit;
    let data: Vec<_> = products.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// DELETE /v1/admin/site-products/{id}
pub async fn delete_product(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("UPDATE products SET status = 'deleted' WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Product not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Jobs Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminJobRow {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub location: Option<String>,
    pub job_type: String,
    pub status: String,
    pub apply_count: i64,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/site-jobs
pub async fn list_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let jobs = sqlx::query_as::<_, AdminJobRow>(
        r#"SELECT id, user_id, title, location, job_type, status, apply_count, created_at
           FROM jobs WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = jobs.len() as i64 > limit;
    let data: Vec<_> = jobs.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|j| j.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// DELETE /v1/admin/site-jobs/{id}
pub async fn delete_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("UPDATE jobs SET status = 'deleted' WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Job not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Funding Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminFundingRow {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub goal: rust_decimal::Decimal,
    pub raised: rust_decimal::Decimal,
    pub status: String,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/site-funding
pub async fn list_funding(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let items = sqlx::query_as::<_, AdminFundingRow>(
        r#"SELECT id, user_id, title, goal, raised, status, created_at
           FROM funding WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = items.len() as i64 > limit;
    let data: Vec<_> = items.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|f| f.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// DELETE /v1/admin/site-funding/{id}
pub async fn delete_funding(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("DELETE FROM funding WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Funding campaign not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Events Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminEventRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub location: Option<String>,
    pub start_date: OffsetDateTime,
    pub end_date: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/site-events
pub async fn list_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let events = sqlx::query_as::<_, AdminEventRow>(
        r#"SELECT id, user_id, name, location, start_date, end_date, created_at
           FROM events WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = events.len() as i64 > limit;
    let data: Vec<_> = events.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|e| e.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// DELETE /v1/admin/site-events/{id}
pub async fn delete_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let result = sqlx::query("DELETE FROM events WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Event not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Forums Admin ──

#[derive(Debug, Serialize, FromRow)]
pub struct AdminForumRow {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub category_id: Option<i64>,
    pub reply_count: i64,
    pub view_count: i64,
    pub is_pinned: bool,
    pub is_closed: bool,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/site-forums
pub async fn list_forums(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let forums = sqlx::query_as::<_, AdminForumRow>(
        r#"SELECT id, user_id, title, category_id, reply_count, view_count, is_pinned, is_closed, created_at
           FROM forum_threads WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = forums.len() as i64 > limit;
    let data: Vec<_> = forums.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|f| f.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// PUT /v1/admin/site-forums/{id}/pin — toggle pin/close
pub async fn update_forum(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateForumRequest>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    sqlx::query(
        "UPDATE forum_threads SET is_pinned = COALESCE($2, is_pinned), is_closed = COALESCE($3, is_closed) WHERE id = $1",
    )
    .bind(id)
    .bind(req.is_pinned)
    .bind(req.is_closed)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateForumRequest {
    pub is_pinned: Option<bool>,
    pub is_closed: Option<bool>,
}

/// DELETE /v1/admin/site-forums/{id}
pub async fn delete_forum(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    sqlx::query("DELETE FROM forum_replies WHERE thread_id = $1").bind(id).execute(&state.db).await?;
    let result = sqlx::query("DELETE FROM forum_threads WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Forum thread not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}
