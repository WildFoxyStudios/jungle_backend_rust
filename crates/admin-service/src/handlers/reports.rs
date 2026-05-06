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
    permissions::Permission,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct ReportRow {
    pub id: i64,
    pub reporter_id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub reason: String,
    pub description: String,
    pub status: String,
    pub created_at: OffsetDateTime,
}

pub async fn list_reports(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ModerateReports, &state).await?;
    let limit = params.limit();
    let cursor = params.cursor_id();

    let reports = sqlx::query_as::<_, ReportRow>(
        "SELECT * FROM reports WHERE ($1::bigint IS NULL OR id < $1) ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = reports.len() as i64 > limit;
    let reports: Vec<_> = reports.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": reports, "meta": { "has_more": has_more } })))
}

pub async fn get_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ModerateReports, &state).await?;

    let report = sqlx::query_as::<_, ReportRow>("SELECT * FROM reports WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;

    Ok(Json(json!({ "data": report })))
}

pub async fn resolve_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ModerateReports, &state).await?;

    sqlx::query("UPDATE reports SET status = 'resolved' WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "resolved": true } })))
}

pub async fn dismiss_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ModerateReports, &state).await?;

    sqlx::query("UPDATE reports SET status = 'dismissed' WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "dismissed": true } })))
}
