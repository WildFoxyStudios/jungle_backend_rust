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
pub struct ConfigRow {
    pub id: i64,
    pub category: String,
    pub key: String,
    pub value: String,
    pub value_type: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub items: Vec<ConfigItem>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigItem {
    pub category: String,
    pub key: String,
    pub value: String,
}

pub async fn list_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, ConfigRow>(
        "SELECT id, category, key, value, value_type FROM site_config ORDER BY category, key",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

pub async fn get_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, ConfigRow>(
        "SELECT id, category, key, value, value_type FROM site_config WHERE category = $1 ORDER BY key",
    )
    .bind(&category)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

pub async fn update_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let mut updated = 0i64;
    for item in &req.items {
        let result = sqlx::query(
            r#"
            INSERT INTO site_config (category, key, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (category, key) DO UPDATE SET value = EXCLUDED.value
            "#,
        )
        .bind(&item.category)
        .bind(&item.key)
        .bind(&item.value)
        .execute(&state.db)
        .await?;
        updated += result.rows_affected() as i64;
    }

    Ok(Json(json!({ "data": { "updated": updated } })))
}
