use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct AnnouncementRow {
    pub id: i64,
    pub text: String,
    pub target: String,
    pub created_at: OffsetDateTime,
}

/// GET /v1/announcements — List active announcements the user hasn't dismissed
pub async fn list_active_announcements(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, AnnouncementRow>(
        r#"SELECT a.id, a.text, a.target, a.created_at
        FROM announcements a
        WHERE a.active = true
          AND NOT EXISTS (
              SELECT 1 FROM announcement_views av
              WHERE av.announcement_id = a.id AND av.user_id = $1
          )
        ORDER BY a.created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

/// POST /v1/announcements/{id}/dismiss — Mark an announcement as viewed/dismissed
pub async fn dismiss_announcement(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        r#"INSERT INTO announcement_views (announcement_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (announcement_id, user_id) DO NOTHING"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "dismissed": true } })))
}
