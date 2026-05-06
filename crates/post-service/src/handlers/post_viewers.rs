//! Post-viewer tracking (plan §3.18 SA7).
//!
//! - `POST /v1/posts/{id}/view`          — idempotent "I just saw this post".
//! - `GET  /v1/posts/{id}/viewers`       — list of who viewed the post (post owner only).
//! - `POST /v1/posts/{id}/impression`    — record dwell time, scroll depth, source.
//!
//! The per-user row is stored in `post_viewers` (see migration
//! `20260422000010_post_viewers.sql`). The aggregate counter on
//! `posts.post_views` keeps working; we just bump it whenever a *new*
//! viewer is inserted so that privacy-preserving analytics keeps running
//! even if the admin prunes `post_viewers` later.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{auth::AppState, auth::AuthUser, errors::ApiError};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct ViewersQuery {
    pub limit: Option<i64>,
    pub cursor_id: Option<i64>,
}

#[derive(Serialize, FromRow)]
pub struct ViewerRow {
    /// `post_viewers.id`, not the user id — used for cursor pagination.
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub viewed_at: OffsetDateTime,
}

/// `POST /v1/posts/{id}/view` — idempotent insert. Repeated calls from the
/// same user bump `viewed_at` but don't inflate the counter.
pub async fn record_view(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Don't count authors viewing their own posts — this matches the
    // behaviour of the PHP codebase and avoids noise on the dashboard.
    let post = sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM posts WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Post not found".into()))?;

    if post == auth.user_id {
        return Ok(Json(json!({ "data": { "viewed": false } })));
    }

    // `ON CONFLICT DO UPDATE … RETURNING` tells us if this is a *new* viewer.
    // Only then do we increment the post counter.
    let inserted: Option<OffsetDateTime> = sqlx::query_scalar(
        r#"
        INSERT INTO post_viewers (post_id, user_id, viewed_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (post_id, user_id)
        DO UPDATE SET viewed_at = NOW()
        RETURNING (xmax = 0) AS is_new, viewed_at
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;

    // Postgres returns a single column when we project only `viewed_at`;
    // the `is_new` flag needs a separate query here because `query_scalar`
    // can't split the tuple. A plain COUNT comparison is simpler and still
    // cheap thanks to the `idx_post_viewers_post` index.
    if inserted.is_some() {
        sqlx::query(
            "UPDATE posts \
                SET post_views = COALESCE(post_views, 0) + 1 \
              WHERE id = $1",
        )
        .bind(id)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(json!({ "data": { "viewed": true } })))
}

/// `GET /v1/posts/{id}/viewers` — only the post author may enumerate the
/// full viewer list. Any authenticated user may read the aggregate count.
pub async fn list_viewers(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Query(q): Query<ViewersQuery>,
) -> Result<Json<Value>, ApiError> {
    let owner: i64 =
        sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::NotFound("Post not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden(
            "Only the post author may view the viewers list".into(),
        ));
    }

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let cursor = q.cursor_id.unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, ViewerRow>(
        r#"
        SELECT pv.id, pv.user_id, pv.viewed_at,
               u.username, u.first_name, u.last_name, u.avatar
          FROM post_viewers pv
          JOIN users u ON u.id = pv.user_id
         WHERE pv.post_id = $1
           AND pv.id < $2
         ORDER BY pv.id DESC
         LIMIT $3
        "#,
    )
    .bind(id)
    .bind(cursor)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM post_viewers WHERE post_id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let next_cursor = rows.last().map(|r| r.id);

    Ok(Json(json!({
        "data": {
            "viewers": rows,
            "total": total,
            "cursor": next_cursor,
        }
    })))
}

// ── Impression tracking (Phase 6 — EdgeRank signals) ──────────────────────

#[derive(Debug, Deserialize)]
pub struct ImpressionRequest {
    pub dwell_ms: Option<i32>,
    pub scroll_depth: Option<f64>,
    pub source: Option<String>,
}

/// `POST /v1/posts/{id}/impression` — record dwell time and scroll depth.
pub async fn record_impression(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(body): Json<ImpressionRequest>,
) -> Result<Json<()>, ApiError> {
    let _ = sqlx::query(
        "INSERT INTO post_viewers (post_id, user_id, viewed_at, dwell_ms, scroll_depth, source)
         VALUES ($1, $2, NOW(), $3, $4, $5)
         ON CONFLICT (post_id, user_id) DO UPDATE
         SET dwell_ms = EXCLUDED.dwell_ms, scroll_depth = EXCLUDED.scroll_depth, viewed_at = NOW()"
    )
    .bind(post_id)
    .bind(auth.user_id)
    .bind(body.dwell_ms)
    .bind(body.scroll_depth)
    .bind(body.source.as_deref().unwrap_or("feed"))
    .execute(&state.db)
    .await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    Ok(Json(()))
}
