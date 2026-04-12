use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, String, String, bool)>(
        "SELECT id, name, subject, body, is_active FROM email_templates ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, subject, body, active)| {
            json!({ "id": id, "name": name, "subject": subject, "body": body, "is_active": active })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct UpsertTemplateRequest {
    pub name: String,
    pub subject: String,
    pub body: String,
    pub is_active: Option<bool>,
}

pub async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<UpsertTemplateRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO email_templates (name, subject, body, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&req.name)
    .bind(&req.subject)
    .bind(&req.body)
    .bind(req.is_active.unwrap_or(true))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpsertTemplateRequest>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE email_templates SET name = $1, subject = $2, body = $3, is_active = COALESCE($4, is_active) WHERE id = $5",
    )
    .bind(&req.name)
    .bind(&req.subject)
    .bind(&req.body)
    .bind(req.is_active)
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Template not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM email_templates WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}
