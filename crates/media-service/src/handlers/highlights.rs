//! Story Highlights — permanent story collections pinned to a user profile.
//!
//! Ports the PHP `api/highlight` endpoint (subtypes: create, delete,
//! add_story, get_highlight, get_highlight_stories).
//!
//! Routes:
//!   POST   /v1/story-highlights                        — create
//!   GET    /v1/story-highlights/my                     — list my own
//!   GET    /v1/users/{user_id}/story-highlights        — list a user's
//!   GET    /v1/story-highlights/{id}                   — get one + items
//!   PUT    /v1/story-highlights/{id}                   — rename / re-cover
//!   DELETE /v1/story-highlights/{id}                   — delete highlight
//!   POST   /v1/story-highlights/{id}/stories           — add story_media to highlight
//!   DELETE /v1/story-highlights/{id}/stories/{sid}     — remove item from highlight

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
use validator::Validate;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct HighlightRow {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub cover_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct HighlightWithCount {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub cover_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub item_count: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct HighlightItemRow {
    pub item_id: i64,
    pub story_media_id: i64,
    pub media_type: String,
    pub media_url: String,
    pub thumbnail_url: Option<String>,
    pub description: String,
    pub duration: Option<i32>,
    pub added_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateHighlightRequest {
    #[validate(length(min = 1, max = 60))]
    pub title: String,
    pub cover_url: Option<String>,
    #[serde(default)]
    pub story_media_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateHighlightRequest {
    #[validate(length(min = 1, max = 60))]
    pub title: Option<String>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddStoriesRequest {
    pub story_media_ids: Vec<i64>,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /v1/story-highlights
pub async fn create_highlight(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateHighlightRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let mut tx = state.db.begin().await?;

    let highlight = sqlx::query_as::<_, HighlightRow>(
        r#"
        INSERT INTO story_highlights (user_id, title, cover_url)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, title, cover_url, created_at, updated_at
        "#,
    )
    .bind(auth.user_id)
    .bind(req.title.trim())
    .bind(req.cover_url.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    if !req.story_media_ids.is_empty() {
        let owned_ids = filter_owned_story_media(
            &mut *tx,
            auth.user_id,
            &req.story_media_ids,
        )
        .await?;

        for (idx, sm_id) in owned_ids.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO story_highlight_items (highlight_id, story_media_id, sort_order)
                   VALUES ($1, $2, $3)
                   ON CONFLICT (highlight_id, story_media_id) DO NOTHING"#,
            )
            .bind(highlight.id)
            .bind(sm_id)
            .bind(idx as i32)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(Json(json!({ "data": highlight })))
}

/// GET /v1/story-highlights/my
pub async fn my_highlights(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    list_highlights_for(&state, auth.user_id, &params).await
}

/// GET /v1/users/{user_id}/story-highlights
pub async fn user_highlights(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(user_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    list_highlights_for(&state, user_id, &params).await
}

async fn list_highlights_for(
    state: &AppState,
    user_id: i64,
    params: &PaginationParams,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let rows = sqlx::query_as::<_, HighlightWithCount>(
        r#"
        SELECT h.id, h.user_id, h.title, h.cover_url, h.created_at, h.updated_at,
               COALESCE(COUNT(i.id), 0) AS item_count
          FROM story_highlights h
          LEFT JOIN story_highlight_items i ON i.highlight_id = h.id
         WHERE h.user_id = $1
           AND ($2::bigint IS NULL OR h.id < $2)
         GROUP BY h.id
         ORDER BY h.id DESC
         LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.id.to_string());

    Ok(Json(json!({
        "data": rows,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// GET /v1/story-highlights/{id}
pub async fn get_highlight(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let highlight = sqlx::query_as::<_, HighlightRow>(
        "SELECT id, user_id, title, cover_url, created_at, updated_at FROM story_highlights WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Highlight not found".into()))?;

    let limit = params.limit();
    let cursor = params.cursor_id();

    let items = sqlx::query_as::<_, HighlightItemRow>(
        r#"
        SELECT i.id AS item_id, sm.id AS story_media_id,
               sm.media_type, sm.media_url, sm.thumbnail_url,
               sm.description, sm.duration, i.added_at
          FROM story_highlight_items i
          JOIN story_media sm ON sm.id = i.story_media_id
         WHERE i.highlight_id = $1
           AND ($2::bigint IS NULL OR i.id < $2)
         ORDER BY i.sort_order ASC, i.id DESC
         LIMIT $3
        "#,
    )
    .bind(id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = items.len() as i64 > limit;
    let items: Vec<_> = items.into_iter().take(limit as usize).collect();
    let next_cursor = items.last().map(|r| r.item_id.to_string());

    Ok(Json(json!({
        "data": {
            "highlight": highlight,
            "items": items,
        },
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// PUT /v1/story-highlights/{id}
pub async fn update_highlight(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateHighlightRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;
    verify_owner(&state, id, auth.user_id).await?;

    let updated = sqlx::query_as::<_, HighlightRow>(
        r#"
        UPDATE story_highlights
           SET title     = COALESCE($2, title),
               cover_url = COALESCE($3, cover_url),
               updated_at = NOW()
         WHERE id = $1
         RETURNING id, user_id, title, cover_url, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(req.title.as_deref().map(str::trim))
    .bind(req.cover_url.as_deref())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": updated })))
}

/// DELETE /v1/story-highlights/{id}
pub async fn delete_highlight(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    sqlx::query("DELETE FROM story_highlights WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

/// POST /v1/story-highlights/{id}/stories
pub async fn add_stories_to_highlight(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AddStoriesRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.story_media_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "story_media_ids must not be empty".into(),
        ));
    }
    verify_owner(&state, id, auth.user_id).await?;

    let mut tx = state.db.begin().await?;
    let owned_ids = filter_owned_story_media(&mut *tx, auth.user_id, &req.story_media_ids).await?;

    let current_max: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) FROM story_highlight_items WHERE highlight_id = $1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(-1);

    let mut added = 0i64;
    for (offset, sm_id) in owned_ids.iter().enumerate() {
        let res = sqlx::query(
            r#"INSERT INTO story_highlight_items (highlight_id, story_media_id, sort_order)
               VALUES ($1, $2, $3)
               ON CONFLICT (highlight_id, story_media_id) DO NOTHING"#,
        )
        .bind(id)
        .bind(sm_id)
        .bind(current_max + 1 + offset as i32)
        .execute(&mut *tx)
        .await?;
        added += res.rows_affected() as i64;
    }

    sqlx::query("UPDATE story_highlights SET updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "added": added } })))
}

/// DELETE /v1/story-highlights/{id}/stories/{sid}
pub async fn remove_story_from_highlight(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, story_media_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    verify_owner(&state, id, auth.user_id).await?;

    let res = sqlx::query(
        "DELETE FROM story_highlight_items WHERE highlight_id = $1 AND story_media_id = $2",
    )
    .bind(id)
    .bind(story_media_id)
    .execute(&state.db)
    .await?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("Highlight item not found".into()));
    }

    sqlx::query("UPDATE story_highlights SET updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "removed": true } })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn verify_owner(state: &AppState, highlight_id: i64, user_id: i64) -> Result<(), ApiError> {
    let owner: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM story_highlights WHERE id = $1")
            .bind(highlight_id)
            .fetch_optional(&state.db)
            .await?;

    match owner {
        None => Err(ApiError::NotFound("Highlight not found".into())),
        Some(uid) if uid != user_id => Err(ApiError::Forbidden("".into())),
        Some(_) => Ok(()),
    }
}

/// Return the subset of `ids` that reference story_media rows owned by `user_id`.
async fn filter_owned_story_media<'c, E>(
    executor: E,
    user_id: i64,
    ids: &[i64],
) -> Result<Vec<i64>, ApiError>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<i64> = sqlx::query_scalar(
        r#"
        SELECT sm.id
          FROM story_media sm
          JOIN stories s ON s.id = sm.story_id
         WHERE sm.id = ANY($1::bigint[])
           AND s.user_id = $2
        "#,
    )
    .bind(ids)
    .bind(user_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}
