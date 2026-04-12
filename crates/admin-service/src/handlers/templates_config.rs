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

// ── Colored Post Templates ──────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ColoredPostTemplateRow {
    pub id: i64,
    pub color_1: String,
    pub color_2: String,
    pub text_color: String,
    pub image: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateColoredPostTemplateRequest {
    pub color_1: String,
    pub color_2: String,
    pub text_color: Option<String>,
    pub image: Option<String>,
}

/// GET /v1/admin/colored-posts — list colored post templates
pub async fn list_colored_post_templates(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let templates = sqlx::query_as::<_, ColoredPostTemplateRow>(
        "SELECT id, color_1, color_2, text_color, image FROM colored_post_templates ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": templates })))
}

/// POST /v1/admin/colored-posts — create a colored post template
pub async fn create_colored_post_template(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CreateColoredPostTemplateRequest>,
) -> Result<Json<Value>, ApiError> {
    let template = sqlx::query_as::<_, ColoredPostTemplateRow>(
        r#"
        INSERT INTO colored_post_templates (color_1, color_2, text_color, image)
        VALUES ($1, $2, $3, $4)
        RETURNING id, color_1, color_2, text_color, image
        "#,
    )
    .bind(&req.color_1)
    .bind(&req.color_2)
    .bind(req.text_color.as_deref().unwrap_or("#ffffff"))
    .bind(req.image.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": template })))
}

/// PUT /v1/admin/colored-posts/{id} — update a template
pub async fn update_colored_post_template(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CreateColoredPostTemplateRequest>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        r#"
        UPDATE colored_post_templates
        SET color_1 = $1, color_2 = $2,
            text_color = COALESCE($3, text_color),
            image = COALESCE($4, image)
        WHERE id = $5
        "#,
    )
    .bind(&req.color_1)
    .bind(&req.color_2)
    .bind(req.text_color.as_deref())
    .bind(req.image.as_deref())
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Template not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

/// DELETE /v1/admin/colored-posts/{id} — delete a template
pub async fn delete_colored_post_template(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM colored_post_templates WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Template not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Reaction Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ReactionTypeRow {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateReactionTypeRequest {
    pub name: String,
    pub icon: String,
    pub is_active: Option<bool>,
}

/// GET /v1/admin/reaction-types — list reaction types
pub async fn list_reaction_types(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let types = sqlx::query_as::<_, ReactionTypeRow>(
        "SELECT id, name, icon, is_active FROM reaction_types ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": types })))
}

/// POST /v1/admin/reaction-types — create a reaction type
pub async fn create_reaction_type(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CreateReactionTypeRequest>,
) -> Result<Json<Value>, ApiError> {
    let rt = sqlx::query_as::<_, ReactionTypeRow>(
        r#"
        INSERT INTO reaction_types (name, icon, is_active)
        VALUES ($1, $2, $3)
        RETURNING id, name, icon, is_active
        "#,
    )
    .bind(&req.name)
    .bind(&req.icon)
    .bind(req.is_active.unwrap_or(true))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": rt })))
}

/// PUT /v1/admin/reaction-types/{id} — update a reaction type
pub async fn update_reaction_type(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CreateReactionTypeRequest>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE reaction_types SET name = $1, icon = $2, is_active = $3 WHERE id = $4",
    )
    .bind(&req.name)
    .bind(&req.icon)
    .bind(req.is_active.unwrap_or(true))
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Reaction type not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

/// DELETE /v1/admin/reaction-types/{id} — delete a reaction type
pub async fn delete_reaction_type(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM reaction_types WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Reaction type not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}
