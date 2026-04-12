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

#[derive(Debug, Deserialize)]
pub struct UserSearchQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Deserialize)]
pub struct AdminUpdateUserRequest {
    pub is_admin: Option<bool>,
    pub is_pro: Option<bool>,
    pub balance: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AdminUserRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_admin: bool,
    pub is_pro: bool,
    pub is_verified: bool,
    pub active: bool,
    pub created_at: OffsetDateTime,
    pub last_login_at: Option<OffsetDateTime>,
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<UserSearchQuery>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;
    let limit = q.pagination.limit();
    let cursor = q.pagination.cursor_id();

    // Build status filter
    let status_filter: Option<bool> = match q.status.as_deref() {
        Some("active") => Some(true),
        Some("banned") => Some(false),
        _ => None,
    };
    let pro_filter: Option<bool> = match q.status.as_deref() {
        Some("pro") => Some(true),
        _ => None,
    };

    let users = if let Some(ref search) = q.q {
        let ilike = format!("%{}%", search);
        sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT id, uuid, username, email, first_name, last_name, avatar, is_admin, is_pro, is_verified, active, created_at, last_login_at
            FROM users
            WHERE (username ILIKE $1 OR email ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1)
              AND ($2::bigint IS NULL OR id < $2)
              AND ($4::bool IS NULL OR active = $4)
              AND ($5::bool IS NULL OR is_pro = $5)
            ORDER BY id DESC LIMIT $3
            "#,
        )
        .bind(&ilike)
        .bind(cursor)
        .bind(limit + 1)
        .bind(status_filter)
        .bind(pro_filter)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT id, uuid, username, email, first_name, last_name, avatar, is_admin, is_pro, is_verified, active, created_at, last_login_at
            FROM users
            WHERE ($1::bigint IS NULL OR id < $1)
              AND ($3::bool IS NULL OR active = $3)
              AND ($4::bool IS NULL OR is_pro = $4)
            ORDER BY id DESC LIMIT $2
            "#,
        )
        .bind(cursor)
        .bind(limit + 1)
        .bind(status_filter)
        .bind(pro_filter)
        .fetch_all(&state.db)
        .await?
    };

    let has_more = users.len() as i64 > limit;
    let users: Vec<_> = users.into_iter().take(limit as usize).collect();
    let next_cursor = users.last().map(|u| u.id.to_string());

    Ok(Json(json!({ "data": users, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn get_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let user = sqlx::query_as::<_, AdminUserRow>(
        "SELECT id, uuid, username, email, first_name, last_name, avatar, is_admin, is_pro, is_verified, active, created_at, last_login_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    Ok(Json(json!({ "data": user })))
}

pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AdminUpdateUserRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query(
        r#"
        UPDATE users SET
            is_admin = COALESCE($2, is_admin),
            is_pro = COALESCE($3, is_pro),
            balance = COALESCE($4::decimal, balance),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(req.is_admin)
    .bind(req.is_pro)
    .bind(&req.balance)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn ban_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE users SET active = FALSE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "banned": true } })))
}

pub async fn unban_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE users SET active = TRUE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "unbanned": true } })))
}

pub async fn verify_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE users SET is_verified = TRUE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "verified": true } })))
}

pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    // Soft delete — deactivate
    sqlx::query("UPDATE users SET active = FALSE, email = CONCAT('deleted_', id, '@deleted.local'), updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}
