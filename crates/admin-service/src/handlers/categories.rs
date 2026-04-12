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

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin { return Err(ApiError::Forbidden("".into())); }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct CategoryRow {
    pub id: i64,
    pub r#type: String,
    pub parent_id: Option<i64>,
    pub name_key: String,
    pub slug: Option<String>,
    pub active: bool,
    pub sort_order: i32,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub r#type: String,
    pub name_key: String,
    pub slug: Option<String>,
    pub parent_id: Option<i64>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name_key: Option<String>,
    pub slug: Option<String>,
    pub active: Option<bool>,
    pub sort_order: Option<i32>,
}

pub async fn list_categories(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let cats = sqlx::query_as::<_, CategoryRow>(
        "SELECT * FROM categories ORDER BY type, sort_order, id",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": cats })))
}

pub async fn create_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let cat = sqlx::query_as::<_, CategoryRow>(
        "INSERT INTO categories (type, name_key, slug, parent_id, sort_order) VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(&req.r#type)
    .bind(&req.name_key)
    .bind(&req.slug)
    .bind(req.parent_id)
    .bind(req.sort_order.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": cat })))
}

pub async fn update_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let cat = sqlx::query_as::<_, CategoryRow>(
        r#"
        UPDATE categories SET
            name_key = COALESCE($2, name_key),
            slug = COALESCE($3, slug),
            active = COALESCE($4, active),
            sort_order = COALESCE($5, sort_order)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name_key)
    .bind(&req.slug)
    .bind(req.active)
    .bind(req.sort_order)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": cat })))
}

pub async fn delete_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM categories WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}
