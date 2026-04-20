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

use super::posts::PostRow;

// ── Poll Vote ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PollVoteRequest {
    pub option_index: i32,
}

/// POST /v1/posts/{id}/poll/vote
pub async fn vote_poll(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(req): Json<PollVoteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify that the post exists and has a poll
    let poll = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM polls WHERE post_id = $1",
    )
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Poll not found for this post".into()))?;

    let poll_id = poll.0;

    // Check if user already voted
    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM poll_votes WHERE poll_id = $1 AND user_id = $2",
    )
    .bind(poll_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;

    if existing.is_some() {
        return Err(ApiError::BadRequest("You already voted on this poll".into()));
    }

    sqlx::query(
        "INSERT INTO poll_votes (poll_id, user_id, option_index) VALUES ($1, $2, $3)",
    )
    .bind(poll_id)
    .bind(auth.user_id)
    .bind(req.option_index)
    .execute(&state.db)
    .await?;

    // Return updated vote counts
    let votes = sqlx::query_as::<_, (i32, i64)>(
        "SELECT option_index, COUNT(*) as cnt FROM poll_votes WHERE poll_id = $1 GROUP BY option_index ORDER BY option_index",
    )
    .bind(poll_id)
    .fetch_all(&state.db)
    .await?;

    let vote_counts: Vec<serde_json::Value> = votes
        .into_iter()
        .map(|(idx, cnt)| json!({ "option_index": idx, "count": cnt }))
        .collect();

    Ok(Json(json!({ "data": { "voted": req.option_index, "results": vote_counts } })))
}

// ── Pin Post ───────────────────────────────────────────────────────

/// POST /v1/posts/{id}/pin
pub async fn pin_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE posts SET is_pinned = TRUE WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "pinned": true } })))
}

/// DELETE /v1/posts/{id}/pin
pub async fn unpin_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE posts SET is_pinned = FALSE WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "pinned": false } })))
}

// ── Boost Post ─────────────────────────────────────────────────────

/// POST /v1/posts/{id}/boost — requires Pro subscription
pub async fn boost_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify user is pro
    let is_pro = sqlx::query_scalar::<_, i32>(
        "SELECT is_pro FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if is_pro == 0 {
        return Err(ApiError::Forbidden("Pro subscription required to boost posts".into()));
    }

    let result = sqlx::query(
        "UPDATE posts SET is_boosted = TRUE WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "boosted": true } })))
}

/// DELETE /v1/posts/{id}/boost — stop boosting a post. Always allowed for
/// the owner even if their Pro subscription has lapsed, so they can turn off
/// a boost they no longer want.
pub async fn unboost_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE posts SET is_boosted = FALSE WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "boosted": false } })))
}

// ── Report Post ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    pub reason: String,
    pub description: Option<String>,
}

/// POST /v1/posts/{id}/report
pub async fn report_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReportRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Prevent duplicate reports
    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM reports WHERE reporter_id = $1 AND target_type = 'post' AND target_id = $2",
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    if existing.is_some() {
        return Err(ApiError::BadRequest("You already reported this post".into()));
    }

    sqlx::query(
        r#"INSERT INTO reports (reporter_id, target_type, target_id, reason, description)
           VALUES ($1, 'post', $2, $3, $4)"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&req.reason)
    .bind(&req.description)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "reported": true } })))
}

// ── Explore Feed ───────────────────────────────────────────────────

/// GET /v1/feed/explore — popular public posts (trending)
pub async fn explore_feed(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let posts = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, user_id, parent_id, content, post_type, media,
                  privacy, feeling, location, is_pinned, is_boosted, is_reel,
                  like_count, comment_count, share_count, view_count,
                  created_at, updated_at
           FROM posts
           WHERE deleted_at IS NULL
             AND is_approved = TRUE
             AND is_reel = FALSE
             AND privacy = 'everyone'
             AND id < $1
             AND created_at > NOW() - INTERVAL '7 days'
           ORDER BY (like_count + comment_count * 2 + share_count * 3) DESC, id DESC
           LIMIT $2"#,
    )
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

// ── Memories ───────────────────────────────────────────────────────

/// GET /v1/memories — "On This Day" posts from previous years
pub async fn get_memories(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let posts = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, user_id, parent_id, content, post_type, media,
                  privacy, feeling, location, is_pinned, is_boosted, is_reel,
                  like_count, comment_count, share_count, view_count,
                  created_at, updated_at
           FROM posts
           WHERE user_id = $1
             AND deleted_at IS NULL
             AND EXTRACT(MONTH FROM created_at) = EXTRACT(MONTH FROM NOW())
             AND EXTRACT(DAY FROM created_at) = EXTRACT(DAY FROM NOW())
             AND created_at < NOW() - INTERVAL '1 year'
           ORDER BY created_at DESC
           LIMIT 50"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": posts })))
}

// ── Create Reply (convenience) ─────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateReplyRequest {
    pub content: String,
    pub media: Option<serde_json::Value>,
}

/// POST /v1/comments/{id}/replies — convenience route to create a reply
pub async fn create_reply(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(parent_id): Path<i64>,
    Json(req): Json<CreateReplyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get the parent comment's post_id
    let parent = sqlx::query_as::<_, (i64,)>(
        "SELECT post_id FROM comments WHERE id = $1",
    )
    .bind(parent_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Parent comment not found".into()))?;

    let media = req.media.unwrap_or(json!([]));

    let reply = sqlx::query_as::<_, super::comments::CommentRow>(
        r#"INSERT INTO comments (user_id, post_id, parent_id, content, media)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, user_id, post_id, parent_id, content, media, like_count, reply_count, created_at"#,
    )
    .bind(auth.user_id)
    .bind(parent.0)
    .bind(parent_id)
    .bind(&req.content)
    .bind(&media)
    .fetch_one(&state.db)
    .await?;

    // Update denormalized counts
    let _ = sqlx::query("UPDATE posts SET comment_count = comment_count + 1 WHERE id = $1")
        .bind(parent.0)
        .execute(&state.db)
        .await;
    let _ = sqlx::query("UPDATE comments SET reply_count = reply_count + 1 WHERE id = $1")
        .bind(parent_id)
        .execute(&state.db)
        .await;

    Ok(Json(json!({ "data": reply })))
}

// ── Update Ad ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateAdRequest {
    pub audience: Option<String>,
    pub budget: Option<f64>,
    pub status: Option<String>,
}

/// PUT /v1/ads/{id}
pub async fn update_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAdRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        r#"UPDATE user_ads SET
            audience = COALESCE($3, audience),
            budget = COALESCE($4, budget),
            status = COALESCE($5, status)
        WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.audience)
    .bind(req.budget)
    .bind(&req.status)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Ad not found or access denied".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Boosted Content ────────────────────────────────────────────────

/// GET /v1/boosted/posts — my boosted posts
pub async fn my_boosted_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let posts = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, user_id, parent_id, content, post_type, media,
                  privacy, feeling, location, is_pinned, is_boosted, is_reel,
                  like_count, comment_count, share_count, view_count,
                  created_at, updated_at
           FROM posts
           WHERE user_id = $1 AND is_boosted = TRUE AND deleted_at IS NULL AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(auth.user_id)
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

// ── Public: Colored Post Templates & Reaction Types ──────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ColoredPostTemplateRow {
    pub id: i64,
    pub color_1: String,
    pub color_2: String,
    pub text_color: String,
    pub image: String,
}

/// GET /v1/posts/colored-templates — list colored post templates (public)
pub async fn list_colored_templates(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let templates = sqlx::query_as::<_, ColoredPostTemplateRow>(
        "SELECT id, color_1, color_2, text_color, image FROM colored_post_templates ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": templates })))
}

#[derive(Debug, Serialize, FromRow)]
pub struct ReactionTypeRow {
    pub id: i64,
    pub name: String,
    pub icon: String,
}

/// GET /v1/posts/most-liked — trending most-liked posts
pub async fn most_liked_posts(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, user_id, parent_id, content, post_type, media, privacy, feeling,
                  location, is_pinned, is_boosted, is_reel, like_count, comment_count, share_count, view_count, created_at, updated_at
           FROM posts
           WHERE deleted_at IS NULL AND id < $1
           ORDER BY like_count DESC, id DESC LIMIT $2"#,
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());
    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// GET /v1/posts/most-watched — trending video posts by view count
pub async fn most_watched_posts(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, user_id, parent_id, content, post_type, media, privacy, feeling,
                  location, is_pinned, is_boosted, is_reel, like_count, comment_count, share_count, view_count, created_at, updated_at
           FROM posts
           WHERE deleted_at IS NULL AND post_type IN ('video', 'youtube')
             AND id < $1
           ORDER BY view_count DESC, like_count DESC, id DESC LIMIT $2"#,
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());
    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// GET /v1/posts/reaction-types — list active reaction types (public)
pub async fn list_reaction_types(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let types = sqlx::query_as::<_, ReactionTypeRow>(
        "SELECT id, name, icon FROM reaction_types WHERE is_active = true ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": types })))
}

// ═══════════════════════════════════════════════════════════════════
// GET /v1/posts/open-to-work — feed of "open to work" posts
// ═══════════════════════════════════════════════════════════════════

pub async fn open_to_work_feed(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    type Row = (i64, i64, String, Option<String>, Option<String>, String, Option<String>, time::OffsetDateTime);
    let rows: Vec<Row> = sqlx::query_as(
        r#"SELECT p.id, p.user_id, u.username, u.first_name, u.last_name,
                  COALESCE(p.text, ''), u.avatar, p.created_at
             FROM posts p
             JOIN users u ON u.id = p.user_id
            WHERE p.post_type = 'open_to_work'
              AND p.deleted_at IS NULL
              AND p.published_at IS NOT NULL
              AND ($1::bigint IS NULL OR p.id < $1)
         ORDER BY p.id DESC
            LIMIT $2"#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<Row> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.0);

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, uid, username, fn_, ln, text, avatar, created)| {
            json!({
                "id": id,
                "user_id": uid,
                "username": username,
                "first_name": fn_,
                "last_name": ln,
                "text": text,
                "avatar": avatar,
                "created_at": created.to_string(),
            })
        })
        .collect();

    Ok(Json(json!({
        "data": data,
        "meta": { "has_more": has_more, "next_cursor": next_cursor }
    })))
}
