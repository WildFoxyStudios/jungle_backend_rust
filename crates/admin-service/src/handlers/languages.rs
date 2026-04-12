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

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin { return Err(ApiError::Forbidden("".into())); }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct LanguageRow {
    pub id: i64,
    pub name: String,
    pub iso_code: String,
    pub direction: String,
    pub flag_image: String,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateLanguageRequest {
    pub name: String,
    pub iso_code: String,
    pub direction: Option<String>,
    pub flag_image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLanguageRequest {
    pub name: Option<String>,
    pub iso_code: Option<String>,
    pub direction: Option<String>,
    pub flag_image: Option<String>,
    pub active: Option<bool>,
}

pub async fn list_languages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let langs = sqlx::query_as::<_, LanguageRow>("SELECT * FROM languages ORDER BY name")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(json!({ "data": langs })))
}

pub async fn create_language(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateLanguageRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let lang = sqlx::query_as::<_, LanguageRow>(
        "INSERT INTO languages (name, iso_code, direction, flag_image) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(&req.name)
    .bind(&req.iso_code)
    .bind(req.direction.as_deref().unwrap_or("ltr"))
    .bind(req.flag_image.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": lang })))
}

pub async fn update_language(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateLanguageRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let lang = sqlx::query_as::<_, LanguageRow>(
        r#"
        UPDATE languages SET
            name = COALESCE($2, name),
            iso_code = COALESCE($3, iso_code),
            direction = COALESCE($4, direction),
            flag_image = COALESCE($5, flag_image),
            active = COALESCE($6, active)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.iso_code)
    .bind(&req.direction)
    .bind(&req.flag_image)
    .bind(req.active)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": lang })))
}

pub async fn delete_language(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM languages WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}
