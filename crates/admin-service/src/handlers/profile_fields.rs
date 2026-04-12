use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

pub async fn list_fields(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, String, Value, bool, i32, bool)>(
        "SELECT id, name, field_type, options, is_required, sort_order, is_active FROM profile_fields ORDER BY sort_order ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, ftype, options, required, sort, active)| {
            json!({
                "id": id, "name": name, "field_type": ftype,
                "options": options, "is_required": required,
                "sort_order": sort, "is_active": active
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct CreateFieldRequest {
    pub name: String,
    pub field_type: Option<String>,
    pub options: Option<Value>,
    pub is_required: Option<bool>,
    pub sort_order: Option<i32>,
}

pub async fn create_field(
    State(state): State<AppState>,
    Json(req): Json<CreateFieldRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO profile_fields (name, field_type, options, is_required, sort_order)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id"#,
    )
    .bind(&req.name)
    .bind(req.field_type.as_deref().unwrap_or("text"))
    .bind(req.options.as_ref().unwrap_or(&json!([])))
    .bind(req.is_required.unwrap_or(false))
    .bind(req.sort_order.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateFieldRequest {
    pub name: Option<String>,
    pub field_type: Option<String>,
    pub options: Option<Value>,
    pub is_required: Option<bool>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

pub async fn update_field(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateFieldRequest>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        r#"UPDATE profile_fields SET
            name = COALESCE($1, name),
            field_type = COALESCE($2, field_type),
            options = COALESCE($3, options),
            is_required = COALESCE($4, is_required),
            sort_order = COALESCE($5, sort_order),
            is_active = COALESCE($6, is_active)
        WHERE id = $7"#,
    )
    .bind(&req.name)
    .bind(&req.field_type)
    .bind(&req.options)
    .bind(req.is_required)
    .bind(req.sort_order)
    .bind(req.is_active)
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Field not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_field(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM profile_fields WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}
