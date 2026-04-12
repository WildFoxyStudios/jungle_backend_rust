use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::{AppState, AuthUser}, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct PendingQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub content_type: Option<String>,
}

pub async fn pending_posts(
    State(state): State<AppState>,
    Query(q): Query<PendingQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let offset = (q.page.unwrap_or(1) - 1).max(0) * limit;

    let posts = sqlx::query_as::<_, (i64, Option<String>, String, time::OffsetDateTime)>(
        r#"SELECT p.id, p.content, u.username, p.created_at
        FROM posts p JOIN users u ON u.id = p.user_id
        WHERE p.is_approved = FALSE AND p.deleted_at IS NULL
          AND ($3::text IS NULL OR p.post_type = $3)
        ORDER BY p.created_at DESC
        LIMIT $1 OFFSET $2"#,
    )
    .bind(limit)
    .bind(offset)
    .bind(&q.content_type)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = posts
        .into_iter()
        .map(|(id, content, username, created_at)| {
            json!({
                "id": id,
                "content": content,
                "username": username,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn approve_post(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("UPDATE posts SET is_approved = TRUE WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found".into()));
    }

    Ok(Json(json!({ "data": { "approved": true, "post_id": id } })))
}

pub async fn reject_post(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("UPDATE posts SET deleted_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found".into()));
    }

    Ok(Json(json!({ "data": { "rejected": true, "post_id": id } })))
}

pub async fn pending_blogs(
    State(state): State<AppState>,
    Query(q): Query<PendingQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let offset = (q.page.unwrap_or(1) - 1).max(0) * limit;

    let blogs = sqlx::query_as::<_, (i64, String, String, time::OffsetDateTime)>(
        r#"SELECT b.id, b.title, u.username, b.created_at
        FROM blogs b JOIN users u ON u.id = b.user_id
        WHERE b.is_approved = FALSE
        ORDER BY b.created_at DESC
        LIMIT $1 OFFSET $2"#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = blogs
        .into_iter()
        .map(|(id, title, username, created_at)| {
            json!({
                "id": id,
                "title": title,
                "username": username,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn approve_blog(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE blogs SET is_approved = TRUE WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "approved": true, "blog_id": id } })))
}

pub async fn reject_blog(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM blogs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "rejected": true, "blog_id": id } })))
}

/// DELETE /v1/admin/posts/{id} — hard delete a post (admin only)
pub async fn admin_delete_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    // Soft delete with timestamp
    let result = sqlx::query("UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true, "post_id": id } })))
}
