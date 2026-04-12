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
use time::OffsetDateTime;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin { return Err(ApiError::Forbidden("".into())); }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct AnnouncementRow {
    pub id: i64,
    pub text: String,
    pub target: String,
    pub active: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateAnnouncementRequest {
    pub text: String,
    pub target: Option<String>,
}

pub async fn list_announcements(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, AnnouncementRow>(
        "SELECT * FROM announcements ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

pub async fn create_announcement(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateAnnouncementRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, AnnouncementRow>(
        "INSERT INTO announcements (text, target) VALUES ($1, $2) RETURNING *",
    )
    .bind(&req.text)
    .bind(req.target.as_deref().unwrap_or("all"))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": row })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateAnnouncementRequest {
    pub text: Option<String>,
    pub target: Option<String>,
    pub active: Option<bool>,
}

pub async fn update_announcement(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAnnouncementRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query(
        r#"UPDATE announcements SET
            text = COALESCE($2, text),
            target = COALESCE($3, target),
            active = COALESCE($4, active)
        WHERE id = $1"#,
    )
    .bind(id)
    .bind(&req.text)
    .bind(&req.target)
    .bind(req.active)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Announcement not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_announcement(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM announcements WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}
