use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
pub struct FollowUser {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub is_pro: i16,
}

// ==================== FOLLOW ====================

pub async fn follow_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if auth.user_id == user_id {
        return Err(ApiError::BadRequest("Cannot follow yourself".into()));
    }

    // Check user exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND deleted_at IS NULL)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);

    if !exists {
        return Err(ApiError::NotFound("User not found".into()));
    }

    // Check not blocked
    let blocked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE (blocker_id = $1 AND blocked_id = $2) OR (blocker_id = $2 AND blocked_id = $1))",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if blocked {
        return Err(ApiError::Forbidden("".into()));
    }

    // Check if user requires follow confirmation
    let needs_confirm: bool = sqlx::query_scalar(
        "SELECT COALESCE(privacy_settings->>'confirm_followers', 'false')::boolean FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let status = if needs_confirm { "pending" } else { "active" };

    sqlx::query(
        "INSERT INTO follows (follower_id, following_id, status) VALUES ($1, $2, $3) ON CONFLICT (follower_id, following_id) DO UPDATE SET status = $3",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .bind(status)
    .execute(&state.db)
    .await?;

    if status == "active" {
        let _ = state.event_bus.publish(&DomainEvent::FollowCreated {
            follower_id: auth.user_id,
            following_id: user_id,
        }).await;
    }

    Ok(Json(serde_json::json!({
        "data": { "status": status, "following_id": user_id }
    })))
}

pub async fn unfollow_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM follows WHERE follower_id = $1 AND following_id = $2")
        .bind(auth.user_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() > 0 {
        let _ = state.event_bus.publish(&DomainEvent::FollowDeleted {
            follower_id: auth.user_id,
            following_id: user_id,
        }).await;
    }

    Ok(Json(serde_json::json!({
        "data": { "message": "Unfollowed" }
    })))
}

pub async fn get_followers(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(username): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL",
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let followers = sqlx::query_as::<_, FollowUser>(
        r#"SELECT u.id, u.uuid, u.username, u.first_name, u.last_name, u.avatar, u.is_verified, u.is_pro
           FROM follows f
           JOIN users u ON u.id = f.follower_id
           WHERE f.following_id = $1 AND f.status = 'active' AND f.id < $2
             AND u.deleted_at IS NULL
           ORDER BY f.id DESC
           LIMIT $3"#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = followers.len() as i64 > limit;
    let data: Vec<_> = followers.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|u| u.id.to_string());

    Ok(Json(serde_json::json!({
        "data": data,
        "meta": {
            "cursor": next_cursor,
            "has_more": has_more,
        }
    })))
}

pub async fn get_following(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(username): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL",
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let following = sqlx::query_as::<_, FollowUser>(
        r#"SELECT u.id, u.uuid, u.username, u.first_name, u.last_name, u.avatar, u.is_verified, u.is_pro
           FROM follows f
           JOIN users u ON u.id = f.following_id
           WHERE f.follower_id = $1 AND f.status = 'active' AND f.id < $2
             AND u.deleted_at IS NULL
           ORDER BY f.id DESC
           LIMIT $3"#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = following.len() as i64 > limit;
    let data: Vec<_> = following.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|u| u.id.to_string());

    Ok(Json(serde_json::json!({
        "data": data,
        "meta": {
            "cursor": next_cursor,
            "has_more": has_more,
        }
    })))
}

// ==================== BLOCK ====================

pub async fn block_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if auth.user_id == user_id {
        return Err(ApiError::BadRequest("Cannot block yourself".into()));
    }

    // Remove any follow relationship in both directions
    sqlx::query("DELETE FROM follows WHERE (follower_id = $1 AND following_id = $2) OR (follower_id = $2 AND following_id = $1)")
        .bind(auth.user_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    sqlx::query(
        "INSERT INTO blocks (blocker_id, blocked_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    let _ = state.event_bus.publish(&DomainEvent::UserBlocked {
        blocker_id: auth.user_id,
        blocked_id: user_id,
    }).await;

    Ok(Json(serde_json::json!({
        "data": { "message": "User blocked" }
    })))
}

pub async fn unblock_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM blocks WHERE blocker_id = $1 AND blocked_id = $2")
        .bind(auth.user_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "message": "User unblocked" }
    })))
}

// ==================== POKE ====================

pub async fn poke_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if auth.user_id == user_id {
        return Err(ApiError::BadRequest("Cannot poke yourself".into()));
    }

    sqlx::query(
        "INSERT INTO pokes (poker_id, poked_id) VALUES ($1, $2) ON CONFLICT (poker_id, poked_id) DO UPDATE SET created_at = NOW()",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "data": { "message": "Poked!" }
    })))
}

/// GET /v1/social/pokes — List received pokes with cursor pagination
pub async fn list_pokes(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<shared::pagination::PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit.unwrap_or(20).min(50);
    let cursor = params.cursor.and_then(|c| c.parse::<i64>().ok());

    let rows = sqlx::query_as::<_, (i64, i64, String, String, String, bool, String)>(
        r#"SELECT p.id, u.id, u.username, u.first_name, u.last_name, u.is_verified, u.avatar
        FROM pokes p JOIN users u ON p.poker_id = u.id
        WHERE p.poked_id = $1 AND ($2::bigint IS NULL OR p.id < $2)
        ORDER BY p.created_at DESC LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).map(|r| {
        serde_json::json!({
            "id": r.0,
            "user": { "id": r.1, "username": r.2, "first_name": r.3, "last_name": r.4, "is_verified": r.5, "avatar": r.6 }
        })
    }).collect();
    let next_cursor = if has_more { items.last().and_then(|i| i["id"].as_i64()).map(|id| id.to_string()) } else { None };

    Ok(Json(serde_json::json!({
        "data": items,
        "meta": { "has_more": has_more, "cursor": next_cursor }
    })))
}

// ==================== MUTE ====================

pub async fn mute_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "INSERT INTO mutes (user_id, muted_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "data": { "message": "User muted" }
    })))
}

pub async fn unmute_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM mutes WHERE user_id = $1 AND muted_id = $2")
        .bind(auth.user_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "message": "User unmuted" }
    })))
}

/// GET /v1/social/blocked — List users blocked by current user (PHP: get-blocked-users.php)
pub async fn list_blocked_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, FollowUser>(
        r#"SELECT u.id, u.uuid, u.username, u.first_name, u.last_name, u.avatar, u.is_verified, u.is_pro
           FROM blocks b
           JOIN users u ON u.id = b.blocked_id
           WHERE b.blocker_id = $1 AND u.id < $2 AND u.deleted_at IS NULL
           ORDER BY u.id DESC LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(serde_json::json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}
