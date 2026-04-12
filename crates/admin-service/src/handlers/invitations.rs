use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
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
pub struct InvitationRow {
    pub id: i64,
    pub user_id: i64,
    pub code: String,
    pub uses: i64,
    pub max_uses: i64,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/invitations
pub async fn list_invitations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;
    let limit = params.limit();
    let cursor = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, InvitationRow>(
        r#"SELECT id, user_id, code, uses, max_uses, is_active, created_at
           FROM invitation_links
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

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub max_uses: Option<i64>,
}

/// POST /v1/admin/invitations
pub async fn create_invitation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let code = uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string();
    let max_uses = req.max_uses.unwrap_or(100);

    let row = sqlx::query_as::<_, (i64,)>(
        r#"INSERT INTO invitation_links (user_id, code, max_uses, is_active)
           VALUES ($1, $2, $3, TRUE)
           RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(&code)
    .bind(max_uses)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": row.0, "code": code, "max_uses": max_uses } })))
}

/// DELETE /v1/admin/invitations/{id}
pub async fn delete_invitation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE invitation_links SET is_active = FALSE WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deactivated": true } })))
}
