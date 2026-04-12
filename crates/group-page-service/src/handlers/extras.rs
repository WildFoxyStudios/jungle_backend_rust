use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

// ── Post Row (shared across group/page/event posts) ────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct PostRow {
    pub id: i64,
    pub uuid: Uuid,
    pub content: String,
    pub post_type: String,
    pub media: serde_json::Value,
    pub like_count: i32,
    pub comment_count: i32,
    pub created_at: OffsetDateTime,
}

// ── Page Posts ──────────────────────────────────────────────────────

/// GET /v1/pages/{id}/posts
pub async fn page_posts(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let posts = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, content, post_type, media, like_count, comment_count, created_at
           FROM posts
           WHERE page_id = $1 AND deleted_at IS NULL AND is_approved = TRUE AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = posts.len() as i64 > limit;
    let data: Vec<_> = posts.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ── Group Posts ─────────────────────────────────────────────────────

/// GET /v1/groups/{id}/posts
pub async fn group_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify membership
    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2 AND status = 'active')",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !is_member {
        // Check if group is public
        let is_public: bool = sqlx::query_scalar(
            "SELECT privacy = 'public' FROM groups WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);

        if !is_public {
            return Err(ApiError::Forbidden("Must be a member to view group posts".into()));
        }
    }

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let posts = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, content, post_type, media, like_count, comment_count, created_at
           FROM posts
           WHERE group_id = $1 AND deleted_at IS NULL AND is_approved = TRUE AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = posts.len() as i64 > limit;
    let data: Vec<_> = posts.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ── Event Posts ─────────────────────────────────────────────────────

/// GET /v1/events/{id}/posts
pub async fn event_posts(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let posts = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, content, post_type, media, like_count, comment_count, created_at
           FROM posts
           WHERE event_id = $1 AND deleted_at IS NULL AND is_approved = TRUE AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = posts.len() as i64 > limit;
    let data: Vec<_> = posts.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ── Page Invite ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub user_ids: Vec<i64>,
}

/// POST /v1/pages/{id}/invite — invite users to like a page
pub async fn invite_page_like(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut invited = 0i64;
    for uid in &req.user_ids {
        let result = sqlx::query(
            r#"INSERT INTO notifications (recipient_id, sender_id, type, target_type, target_id, text)
               VALUES ($1, $2, 'page_invite', 'page', $3, 'invited you to like a page')
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uid)
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await;

        if result.is_ok() {
            invited += 1;
        }
    }

    Ok(Json(json!({ "data": { "invited": invited } })))
}

/// POST /v1/groups/{id}/invite — invite users to join a group
pub async fn invite_group_join(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut invited = 0i64;
    for uid in &req.user_ids {
        let result = sqlx::query(
            r#"INSERT INTO notifications (recipient_id, sender_id, type, target_type, target_id, text)
               VALUES ($1, $2, 'group_invite', 'group', $3, 'invited you to join a group')
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uid)
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await;

        if result.is_ok() {
            invited += 1;
        }
    }

    Ok(Json(json!({ "data": { "invited": invited } })))
}

// ── Page Avatar/Cover ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AvatarCoverRequest {
    pub url: String,
}

/// PUT /v1/pages/{id}/avatar
pub async fn update_page_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AvatarCoverRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE pages SET avatar = $3 WHERE page_id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.url)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Page not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "avatar": req.url } })))
}

/// PUT /v1/pages/{id}/cover
pub async fn update_page_cover(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AvatarCoverRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE pages SET cover = $3 WHERE page_id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.url)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Page not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "cover": req.url } })))
}

/// PUT /v1/groups/{id}/avatar
pub async fn update_group_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AvatarCoverRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Check ownership or admin role
    let is_admin: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM group_members
            WHERE group_id = $1 AND user_id = $2 AND role IN ('admin', 'owner')
        )"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !is_admin {
        return Err(ApiError::Forbidden("Must be group admin".into()));
    }

    sqlx::query("UPDATE groups SET avatar = $2 WHERE id = $1")
        .bind(id)
        .bind(&req.url)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "avatar": req.url } })))
}

/// PUT /v1/groups/{id}/cover
pub async fn update_group_cover(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AvatarCoverRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let is_admin: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM group_members
            WHERE group_id = $1 AND user_id = $2 AND role IN ('admin', 'owner')
        )"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !is_admin {
        return Err(ApiError::Forbidden("Must be group admin".into()));
    }

    sqlx::query("UPDATE groups SET cover = $2 WHERE id = $1")
        .bind(id)
        .bind(&req.url)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "cover": req.url } })))
}

/// PUT /v1/events/{id}/cover
pub async fn update_event_cover(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AvatarCoverRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE events SET cover = $3 WHERE id = $1 AND organizer_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.url)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Event not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "cover": req.url } })))
}

// ── Page Boost & Verify ────────────────────────────────────────────

/// POST /v1/pages/{id}/boost — pro feature
pub async fn boost_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let is_pro = sqlx::query_scalar::<_, i32>(
        "SELECT is_pro FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if is_pro == 0 {
        return Err(ApiError::Forbidden("Pro subscription required".into()));
    }

    let result = sqlx::query(
        "UPDATE pages SET is_boosted = TRUE WHERE page_id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Page not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "boosted": true } })))
}

/// POST /v1/pages/{id}/verify — request page verification
pub async fn request_page_verification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify ownership
    let owned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pages WHERE page_id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !owned {
        return Err(ApiError::Forbidden("Must be page owner".into()));
    }

    sqlx::query(
        r#"INSERT INTO verification_requests (user_id, target_type, target_id, status)
           VALUES ($1, 'page', $2, 'pending')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "verification_requested": true } })))
}

// ── Boosted Pages ──────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct BoostedPageRow {
    pub page_id: i64,
    pub page_name: String,
    pub page_title: String,
    pub avatar: String,
    pub like_count: i32,
}

/// GET /v1/boosted/pages
pub async fn my_boosted_pages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pages = sqlx::query_as::<_, BoostedPageRow>(
        r#"SELECT page_id, page_name, page_title, avatar, like_count
           FROM pages
           WHERE user_id = $1 AND is_boosted = TRUE AND active = TRUE
           ORDER BY page_id DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": pages })))
}

/// GET /v1/groups/{id}/non-members — Users NOT in this group (for invite modal) (PHP: not_in_group_member.php)
pub async fn group_non_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let followers = sqlx::query_as::<_, (i64, String, String, String, String, bool, i16)>(
        r#"SELECT u.id, u.username, u.first_name, u.last_name, u.avatar, u.is_verified, u.is_pro
           FROM follows f
           JOIN users u ON u.id = f.following_id
           WHERE f.follower_id = $1
             AND f.status = 'active'
             AND u.deleted_at IS NULL
             AND u.id NOT IN (
               SELECT user_id FROM group_members WHERE group_id = $2
             )
           ORDER BY u.first_name LIMIT 50"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<_> = followers.into_iter().map(|(uid, username, first_name, last_name, avatar, is_verified, is_pro)| {
        json!({ "id": uid, "username": username, "first_name": first_name, "last_name": last_name, "avatar": avatar, "is_verified": is_verified, "is_pro": is_pro })
    }).collect();

    Ok(Json(json!({ "data": data })))
}

/// GET /v1/pages/{id}/non-likes — Users who haven't liked this page (for invite modal) (PHP: not_in_page_member.php)
pub async fn page_non_likers(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let followers = sqlx::query_as::<_, (i64, String, String, String, String, bool, i16)>(
        r#"SELECT u.id, u.username, u.first_name, u.last_name, u.avatar, u.is_verified, u.is_pro
           FROM follows f
           JOIN users u ON u.id = f.following_id
           WHERE f.follower_id = $1
             AND f.status = 'active'
             AND u.deleted_at IS NULL
             AND u.id NOT IN (
               SELECT user_id FROM page_likes WHERE page_id = $2
             )
           ORDER BY u.first_name LIMIT 50"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<_> = followers.into_iter().map(|(uid, username, first_name, last_name, avatar, is_verified, is_pro)| {
        json!({ "id": uid, "username": username, "first_name": first_name, "last_name": last_name, "avatar": avatar, "is_verified": is_verified, "is_pro": is_pro })
    }).collect();

    Ok(Json(json!({ "data": data })))
}

/// GET /v1/pages/{id}/ratings — List page ratings (PHP: page_reviews.php)
pub async fn list_page_ratings(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, (i64, i64, String, String, String, i16, String, time::OffsetDateTime)>(
        r#"SELECT pr.id, pr.user_id, u.username, u.first_name, u.avatar, pr.rating, pr.review, pr.created_at
           FROM page_ratings pr
           JOIN users u ON u.id = pr.user_id
           WHERE pr.page_id = $1 AND pr.id < $2
           ORDER BY pr.id DESC LIMIT $3"#,
    )
    .bind(id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let last_id = rows.get(rows.len().saturating_sub(2)).map(|r| r.0);
    let data: Vec<_> = rows.into_iter().take(limit as usize).map(|(rid, uid, username, first_name, avatar, rating, review, created_at)| {
        json!({ "id": rid, "user_id": uid, "username": username, "first_name": first_name, "avatar": avatar, "rating": rating, "review": review, "created_at": created_at })
    }).collect();
    let next_cursor = if has_more { last_id.map(|i| i.to_string()) } else { None };

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// GET /v1/groups/check-name?name=mygroup&group_id=5 — Check group name availability (PHP: check_groupname.php)
pub async fn check_group_name(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<CheckNameParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let name = params.name.trim().to_lowercase();

    if name.len() < 5 {
        return Ok(Json(json!({ "data": { "available": false, "reason": "min_length_5" } })));
    }

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM groups WHERE LOWER(group_name) = $1 AND id != $2)",
    )
    .bind(&name)
    .bind(params.current_id.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "available": !exists } })))
}

/// GET /v1/pages/check-name?name=mypage&page_id=5 — Check page name availability (PHP: check_pagename.php)
pub async fn check_page_name(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<CheckNameParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let name = params.name.trim().to_lowercase();

    if name.len() < 5 {
        return Ok(Json(json!({ "data": { "available": false, "reason": "min_length_5" } })));
    }

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pages WHERE LOWER(page_name) = $1 AND page_id != $2)",
    )
    .bind(&name)
    .bind(params.current_id.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "available": !exists } })))
}

#[derive(Debug, Deserialize)]
pub struct CheckNameParams {
    pub name: String,
    pub current_id: Option<i64>,
}
