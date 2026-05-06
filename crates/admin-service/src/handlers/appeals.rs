use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    permissions::Permission,
};
use sqlx::Row;

// ── User-facing: appeal a moderation decision ─────────────────

#[derive(Deserialize)]
pub struct CreateAppealRequest {
    pub report_id: i64,
    pub reason: String,
}

pub async fn create_appeal(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateAppealRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("Appeal reason is required".into()));
    }

    let row = sqlx::query(
        "INSERT INTO report_appeals (report_id, user_id, reason, status, created_at)
         VALUES ($1, $2, $3, 'pending', NOW())
         ON CONFLICT (report_id, user_id) DO UPDATE SET reason = $3, status = 'pending'
         RETURNING id, status, created_at",
    )
    .bind(body.report_id)
    .bind(auth.user_id)
    .bind(&body.reason)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "status": row.get::<String, _>("status"),
        "created_at": row.get::<String, _>("created_at"),
    })))
}

// ── Admin: list appeals ────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListAppealsParams {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_appeals(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListAppealsParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::ModerateReports, &state).await?;

    let rows = sqlx::query(
        "SELECT a.id, a.report_id, a.user_id, a.reason, a.status, a.created_at, a.resolved_at,
                u.username, u.first_name, u.last_name
         FROM report_appeals a
         JOIN users u ON u.id = a.user_id
         WHERE ($1::text IS NULL OR a.status = $1)
         ORDER BY a.created_at DESC
         LIMIT $2",
    )
    .bind(&params.status)
    .bind(params.limit.unwrap_or(50))
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "report_id": r.get::<i64, _>("report_id"),
                "user_id": r.get::<i64, _>("user_id"),
                "username": r.get::<String, _>("username"),
                "first_name": r.get::<String, _>("first_name"),
                "last_name": r.get::<String, _>("last_name"),
                "reason": r.get::<String, _>("reason"),
                "status": r.get::<String, _>("status"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "items": items, "total": items.len() })))
}

// ── Admin: decide on an appeal ─────────────────────────────────

#[derive(Deserialize)]
pub struct DecideAppealRequest {
    pub approved: bool,
    pub notes: Option<String>,
}

pub async fn decide_appeal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(appeal_id): Path<i64>,
    Json(body): Json<DecideAppealRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::ModerateReports, &state).await?;

    let new_status = if body.approved {
        "approved"
    } else {
        "rejected"
    };

    let row = sqlx::query(
        "UPDATE report_appeals SET status = $1, resolved_at = NOW() WHERE id = $2
         RETURNING id, status, resolved_at",
    )
    .bind(new_status)
    .bind(appeal_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?
    .ok_or(ApiError::NotFound("Appeal not found".into()))?;

    if let Some(ref notes) = body.notes {
        tracing::info!(appeal_id, notes, "Appeal decided with admin notes");
    }

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "status": row.get::<String, _>("status"),
        "resolved_at": row.get::<String, _>("resolved_at"),
    })))
}

// ── Strikes: assign a strike to a user ─────────────────────────

#[derive(Deserialize)]
pub struct AssignStrikeRequest {
    pub user_id: i64,
    pub reason: String,
}

pub async fn assign_strike(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<AssignStrikeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth.require_permission(Permission::AssignStrikes, &state).await?;

    let row = sqlx::query(
        "INSERT INTO user_strikes (user_id, reason, created_at)
         VALUES ($1, $2, NOW())
         RETURNING id",
    )
    .bind(body.user_id)
    .bind(&body.reason)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    // Count total strikes and auto-ban if >= 3
    let strike_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_strikes WHERE user_id = $1",
    )
    .bind(body.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    if strike_count >= 3 {
        sqlx::query("UPDATE users SET active = FALSE WHERE id = $1")
            .bind(body.user_id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e);
                ApiError::Internal("DB error".into())
            })?;
    }

    Ok(Json(serde_json::json!({
        "id": row.get::<i64, _>("id"),
        "user_id": body.user_id,
        "strike_count": strike_count,
    })))
}
