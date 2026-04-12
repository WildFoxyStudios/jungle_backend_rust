use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

pub async fn list_custom_pages(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, String, String, bool, time::OffsetDateTime)>(
        "SELECT id, title, slug, page_type, is_active, created_at FROM custom_pages ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, slug, page_type, active, created_at)| {
            json!({
                "id": id,
                "title": title,
                "slug": slug,
                "page_type": page_type,
                "is_active": active,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct CreatePageRequest {
    pub title: String,
    pub slug: String,
    pub content: Option<String>,
    pub page_type: Option<String>,
}

pub async fn create_custom_page(
    State(state): State<AppState>,
    Json(req): Json<CreatePageRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO custom_pages (title, slug, content, page_type) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&req.title)
    .bind(&req.slug)
    .bind(&req.content)
    .bind(req.page_type.as_deref().unwrap_or("custom"))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePageRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn update_custom_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePageRequest>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        r#"UPDATE custom_pages SET
            title = COALESCE($1, title),
            content = COALESCE($2, content),
            is_active = COALESCE($3, is_active),
            updated_at = NOW()
        WHERE id = $4"#,
    )
    .bind(&req.title)
    .bind(&req.content)
    .bind(req.is_active)
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Page not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_custom_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM custom_pages WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn get_custom_page_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64, String, String, Option<String>, String, bool)>(
        "SELECT id, title, slug, content, page_type, is_active FROM custom_pages WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Page not found".into()))?;

    let (id, title, slug, content, page_type, is_active) = row;
    Ok(Json(json!({
        "data": {
            "id": id,
            "title": title,
            "slug": slug,
            "content": content,
            "page_type": page_type,
            "is_active": is_active
        }
    })))
}
