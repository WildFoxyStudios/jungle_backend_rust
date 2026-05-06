use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// ── Blog Reaction ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReactRequest {
    pub reaction: String,
}

/// POST /v1/blogs/{id}/react
pub async fn react_to_blog(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReactRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        r#"INSERT INTO reactions (user_id, target_type, target_id, reaction)
           VALUES ($1, 'blog', $2, $3)
           ON CONFLICT (user_id, target_type, target_id) DO UPDATE SET reaction = $3"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&req.reaction)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "reaction": req.reaction } })))
}

// ── Blogs by Category ──────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct BlogListRow {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub thumbnail: String,
    pub created_at: OffsetDateTime,
}

/// GET /v1/blogs/category/{id} — blogs filtered by category
pub async fn blogs_by_category(
    State(state): State<AppState>,
    Path(category_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let blogs = sqlx::query_as::<_, BlogListRow>(
        r#"SELECT id, user_id, title, COALESCE(thumbnail, '') as thumbnail, created_at
           FROM blogs
           WHERE category_id = $1 AND is_approved = TRUE AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(category_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = blogs.len() as i64 > limit;
    let data: Vec<_> = blogs.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|b| b.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ── Public Custom Pages ──────────────────────────────────────────

/// GET /v1/pages/custom/{slug} — Public access to custom pages (terms, privacy, about, etc.)
pub async fn get_custom_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64, String, String, Option<String>, String)>(
        r#"SELECT id, title, slug, content, page_type
        FROM custom_pages
        WHERE slug = $1 AND is_active = true"#,
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Page not found".into()))?;

    let (id, title, slug, content, page_type) = row;
    Ok(Json(json!({
        "data": {
            "id": id,
            "title": title,
            "slug": slug,
            "content": content,
            "page_type": page_type
        }
    })))
}

/// GET /v1/pages/custom — List all active public custom pages (titles only)
pub async fn list_custom_pages(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, String, String)>(
        r#"SELECT id, title, slug, page_type
        FROM custom_pages
        WHERE is_active = true
        ORDER BY title"#,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, title, slug, page_type)| {
            json!({ "id": id, "title": title, "slug": slug, "page_type": page_type })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

/// POST /v1/blogs/comments/{id}/react — React to a blog comment
pub async fn react_to_blog_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM reactions WHERE user_id=$1 AND target_type='blog_comment' AND target_id=$2)",
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    if exists {
        sqlx::query(
            "DELETE FROM reactions WHERE user_id=$1 AND target_type='blog_comment' AND target_id=$2",
        )
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

        sqlx::query(
            "UPDATE blog_comments SET like_count = GREATEST(like_count - 1, 0) WHERE id = $1",
        )
        .bind(id)
        .execute(&state.db)
        .await?;

        Ok(Json(json!({ "data": { "liked": false } })))
    } else {
        sqlx::query(
            "INSERT INTO reactions (user_id, target_type, target_id, reaction_type) VALUES ($1,'blog_comment',$2,'like') ON CONFLICT DO NOTHING",
        )
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

        sqlx::query("UPDATE blog_comments SET like_count = like_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;

        Ok(Json(json!({ "data": { "liked": true } })))
    }
}
