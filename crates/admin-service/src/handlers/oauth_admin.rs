use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct OAuthAppRow {
    pub id: i64,
    pub user_id: i64,
    pub app_name: String,
    pub client_id: uuid::Uuid,
    pub redirect_uri: String,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/oauth-apps — list all registered OAuth developer apps
pub async fn list_oauth_apps(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;
    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, OAuthAppRow>(
        r#"SELECT id, user_id, app_name, client_id, redirect_uri, is_active, created_at
           FROM oauth_apps
           WHERE id < $1
           ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// POST /v1/admin/oauth-apps/{id}/toggle — activate/deactivate an OAuth app
pub async fn toggle_oauth_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let new_status = sqlx::query_scalar::<_, bool>(
        "UPDATE oauth_apps SET is_active = NOT is_active WHERE id = $1 RETURNING is_active",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("OAuth app not found".into()))?;

    Ok(Json(json!({ "data": { "id": id, "is_active": new_status } })))
}

/// DELETE /v1/admin/oauth-apps/{id} — permanently remove an OAuth app
pub async fn delete_oauth_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    // Cascade: delete tokens, codes, then app
    sqlx::query("DELETE FROM oauth_tokens WHERE app_id = $1").bind(id).execute(&state.db).await?;
    sqlx::query("DELETE FROM oauth_codes WHERE app_id = $1").bind(id).execute(&state.db).await?;
    sqlx::query("DELETE FROM oauth_apps WHERE id = $1").bind(id).execute(&state.db).await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}
