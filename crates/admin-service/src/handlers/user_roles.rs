use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::{AppState, AuthUser}, errors::ApiError, permissions::Permission};

pub async fn make_admin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;
    let result = sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("User not found".into()));
    }

    Ok(Json(json!({ "data": { "is_admin": true, "user_id": user_id } })))
}

pub async fn remove_admin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;
    sqlx::query("UPDATE users SET is_admin = FALSE WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "is_admin": false, "user_id": user_id } })))
}

#[derive(Debug, Deserialize)]
pub struct MakeProRequest {
    pub plan_type: Option<String>,
    pub days: Option<i32>,
}

pub async fn make_pro(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<MakeProRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManagePro, &state).await?;
    let days = req.days.unwrap_or(30);
    let plan = req.plan_type.as_deref().unwrap_or("star");

    let result = sqlx::query(
        "UPDATE users SET is_pro = TRUE, pro_type = $1, pro_expires_at = NOW() + make_interval(days => $2) WHERE id = $3 AND deleted_at IS NULL",
    )
    .bind(plan)
    .bind(days)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("User not found".into()));
    }

    Ok(Json(json!({ "data": { "is_pro": true, "plan": plan, "days": days, "user_id": user_id } })))
}

pub async fn remove_pro(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManagePro, &state).await?;
    sqlx::query(
        "UPDATE users SET is_pro = FALSE, pro_type = NULL, pro_expires_at = NULL WHERE id = $1",
    )
    .bind(user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "is_pro": false, "user_id": user_id } })))
}
