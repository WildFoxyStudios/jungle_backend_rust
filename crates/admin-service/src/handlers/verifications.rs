use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::{AppState, AuthUser}, errors::ApiError, permissions::Permission};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_verification_requests(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::VerifyUsers, &state).await?;
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let offset = (q.page.unwrap_or(1) - 1).max(0) * limit;
    let status = q.status.unwrap_or_else(|| "pending".into());

    let rows = sqlx::query_as::<_, (i64, i64, Option<String>, Option<String>, String, time::OffsetDateTime)>(
        r#"SELECT vr.id, vr.user_id, vr.full_name, vr.document_url, vr.status, vr.created_at
        FROM verification_requests vr
        WHERE vr.status = $1
        ORDER BY vr.created_at ASC
        LIMIT $2 OFFSET $3"#,
    )
    .bind(&status)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, user_id, full_name, doc_url, status, created_at)| {
            json!({
                "id": id,
                "user_id": user_id,
                "full_name": full_name,
                "document_url": doc_url,
                "status": status,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub admin_note: Option<String>,
}

pub async fn approve_verification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReviewRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::VerifyUsers, &state).await?;
    let user_id = sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM verification_requests WHERE id = $1 AND status = 'pending'",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Request not found".into()))?;

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "UPDATE verification_requests SET status = 'approved', admin_note = $1, reviewed_at = NOW() WHERE id = $2",
    )
    .bind(&req.admin_note)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE users SET is_verified = TRUE WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "approved": true, "user_id": user_id } })))
}

pub async fn reject_verification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReviewRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::VerifyUsers, &state).await?;
    let result = sqlx::query(
        "UPDATE verification_requests SET status = 'rejected', admin_note = $1, reviewed_at = NOW() WHERE id = $2",
    )
    .bind(&req.admin_note)
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Request not found".into()));
    }

    Ok(Json(json!({ "data": { "rejected": true } })))
}
