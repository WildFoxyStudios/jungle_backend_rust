use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

#[derive(Debug, Serialize, FromRow)]
pub struct SectionRow {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ForumRow {
    pub id: i64,
    pub section_id: i64,
    pub name: String,
    pub description: String,
    pub thread_count: i32,
    pub last_post_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ThreadRow {
    pub id: i64,
    pub forum_id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub view_count: i32,
    pub reply_count: i32,
    pub last_reply_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub username: String,
    pub avatar: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ReplyRow {
    pub id: i64,
    pub thread_id: i64,
    pub user_id: i64,
    pub content: String,
    pub quoted_reply_id: Option<i64>,
    pub created_at: OffsetDateTime,
    pub username: String,
    pub avatar: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateThreadRequest {
    #[validate(length(min = 1, max = 300))]
    pub title: String,
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReplyRequest {
    #[validate(length(min = 1))]
    pub content: String,
    pub quoted_reply_id: Option<i64>,
}

pub async fn list_sections(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let sections = sqlx::query_as::<_, SectionRow>(
        "SELECT id, name, description FROM forum_sections ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    // Get forums per section
    let mut result = Vec::with_capacity(sections.len());
    for section in sections {
        let forums = sqlx::query_as::<_, ForumRow>(
            "SELECT id, section_id, name, description, thread_count, last_post_at FROM forums WHERE section_id = $1 ORDER BY id",
        )
        .bind(section.id)
        .fetch_all(&state.db)
        .await?;

        result.push(json!({
            "id": section.id,
            "name": section.name,
            "description": section.description,
            "forums": forums
        }));
    }

    Ok(Json(json!({ "data": result })))
}

pub async fn list_threads(
    State(state): State<AppState>,
    Path(forum_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let threads = sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.id, t.forum_id, t.user_id, t.title, t.content, t.view_count, t.reply_count,
            t.last_reply_at, t.created_at, u.username, u.avatar
        FROM forum_threads t JOIN users u ON u.id = t.user_id
        WHERE t.forum_id = $1 AND ($2::bigint IS NULL OR t.id < $2)
        ORDER BY t.id DESC LIMIT $3
        "#,
    )
    .bind(forum_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = threads.len() as i64 > limit;
    let threads: Vec<_> = threads.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": threads, "meta": { "has_more": has_more } }),
    ))
}

pub async fn create_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(forum_id): Path<i64>,
    Json(req): Json<CreateThreadRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let mut tx = state.db.begin().await?;

    let thread_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO forum_threads (forum_id, user_id, title, content) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(forum_id)
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.content)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE forums SET thread_count = thread_count + 1, last_post_at = NOW() WHERE id = $1",
    )
    .bind(forum_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let thread = sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.id, t.forum_id, t.user_id, t.title, t.content, t.view_count, t.reply_count,
            t.last_reply_at, t.created_at, u.username, u.avatar
        FROM forum_threads t JOIN users u ON u.id = t.user_id
        WHERE t.id = $1
        "#,
    )
    .bind(thread_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": thread })))
}

pub async fn get_thread(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE forum_threads SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    let thread = sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.id, t.forum_id, t.user_id, t.title, t.content, t.view_count, t.reply_count,
            t.last_reply_at, t.created_at, u.username, u.avatar
        FROM forum_threads t JOIN users u ON u.id = t.user_id
        WHERE t.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Thread not found".into()))?;

    Ok(Json(json!({ "data": thread })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateThreadRequest {
    #[validate(length(min = 1, max = 300))]
    pub title: Option<String>,
    #[validate(length(min = 1))]
    pub content: Option<String>,
}

/// PUT /v1/forums/threads/{id} — edit a forum thread
pub async fn update_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateThreadRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM forum_threads WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Thread not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("Not the thread owner".into()));
    }

    sqlx::query(
        r#"UPDATE forum_threads
           SET title   = COALESCE($1, title),
               content = COALESCE($2, content)
           WHERE id = $3"#,
    )
    .bind(req.title.as_deref())
    .bind(req.content.as_deref())
    .bind(id)
    .execute(&state.db)
    .await?;

    let thread = sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.id, t.forum_id, t.user_id, t.title, t.content, t.view_count, t.reply_count,
            t.last_reply_at, t.created_at, u.username, u.avatar
        FROM forum_threads t JOIN users u ON u.id = t.user_id
        WHERE t.id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": thread })))
}

pub async fn delete_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM forum_threads WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Thread not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let forum_id = sqlx::query_scalar::<_, i64>("SELECT forum_id FROM forum_threads WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    sqlx::query("DELETE FROM forum_threads WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE forums SET thread_count = GREATEST(thread_count - 1, 0) WHERE id = $1")
        .bind(forum_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn list_replies(
    State(state): State<AppState>,
    Path(thread_id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let replies = sqlx::query_as::<_, ReplyRow>(
        r#"
        SELECT r.id, r.thread_id, r.user_id, r.content, r.quoted_reply_id, r.created_at,
            u.username, u.avatar
        FROM forum_replies r JOIN users u ON u.id = r.user_id
        WHERE r.thread_id = $1 AND ($2::bigint IS NULL OR r.id > $2)
        ORDER BY r.id ASC LIMIT $3
        "#,
    )
    .bind(thread_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = replies.len() as i64 > limit;
    let replies: Vec<_> = replies.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": replies, "meta": { "has_more": has_more } }),
    ))
}

pub async fn create_reply(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(thread_id): Path<i64>,
    Json(req): Json<CreateReplyRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let mut tx = state.db.begin().await?;

    let reply = sqlx::query_as::<_, ReplyRow>(
        r#"
        WITH ins AS (
            INSERT INTO forum_replies (thread_id, user_id, content, quoted_reply_id)
            VALUES ($1, $2, $3, $4)
            RETURNING *
        )
        SELECT ins.id, ins.thread_id, ins.user_id, ins.content, ins.quoted_reply_id, ins.created_at,
            u.username, u.avatar
        FROM ins JOIN users u ON u.id = ins.user_id
        "#,
    )
    .bind(thread_id)
    .bind(auth.user_id)
    .bind(&req.content)
    .bind(req.quoted_reply_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE forum_threads SET reply_count = reply_count + 1, last_reply_at = NOW() WHERE id = $1")
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(json!({ "data": reply })))
}

/// PUT /v1/forums/replies/{id} — Edit own forum reply
pub async fn update_reply(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CreateReplyRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner: i64 = sqlx::query_scalar("SELECT user_id FROM forum_replies WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Reply not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("Not your reply".into()));
    }

    sqlx::query("UPDATE forum_replies SET content = $1 WHERE id = $2")
        .bind(req.content.trim())
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

/// POST /v1/forums/threads/{id}/share — Share forum thread as a post (PHP: thread_share.php, forum_share.php)
pub async fn share_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let thread = sqlx::query_as::<_, (String, i64)>(
        "SELECT title, forum_id FROM forum_threads WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Thread not found".into()))?;

    let share_text = format!("Shared forum thread: \"{}\"", thread.0);

    let post_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO posts (user_id, content, post_type, privacy, is_approved, search_vector)
           VALUES ($1, $2, 'forum_share', 'everyone', true, to_tsvector('simple', $2))
           RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(&share_text)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(
        json!({ "data": { "post_id": post_id, "shared": true } }),
    ))
}

pub async fn vote_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM reactions WHERE user_id=$1 AND target_type='forum_thread' AND target_id=$2)",
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    if exists {
        sqlx::query(
            "DELETE FROM reactions WHERE user_id=$1 AND target_type='forum_thread' AND target_id=$2",
        )
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;
        Ok(Json(json!({ "data": { "voted": false } })))
    } else {
        sqlx::query(
            "INSERT INTO reactions (user_id, target_type, target_id, reaction_type) VALUES ($1,'forum_thread',$2,'like') ON CONFLICT DO NOTHING",
        )
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;
        Ok(Json(json!({ "data": { "voted": true } })))
    }
}

pub async fn delete_reply(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM forum_replies WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Reply not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let thread_id =
        sqlx::query_scalar::<_, i64>("SELECT thread_id FROM forum_replies WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    sqlx::query("DELETE FROM forum_replies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    sqlx::query(
        "UPDATE forum_threads SET reply_count = GREATEST(reply_count - 1, 0) WHERE id = $1",
    )
    .bind(thread_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

impl SearchQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
    fn cursor_id(&self) -> Option<i64> {
        self.cursor.as_ref().and_then(|c| c.parse::<i64>().ok())
    }
}

/// GET /v1/forums/search?q=... — Full-text search across thread titles and content.
pub async fn search_threads(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let q = params.q.unwrap_or_default();
    let q_trimmed = q.trim();
    if q_trimmed.is_empty() {
        return Ok(Json(json!({ "data": [], "meta": { "has_more": false } })));
    }

    let like_pattern = format!("%{}%", q_trimmed);

    let threads = sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.id, t.forum_id, t.user_id, t.title, t.content, t.view_count, t.reply_count,
            t.last_reply_at, t.created_at, u.username, u.avatar
        FROM forum_threads t JOIN users u ON u.id = t.user_id
        WHERE (t.title ILIKE $1 OR t.content ILIKE $1)
          AND ($2::bigint IS NULL OR t.id < $2)
        ORDER BY t.id DESC LIMIT $3
        "#,
    )
    .bind(&like_pattern)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = threads.len() as i64 > limit;
    let threads: Vec<_> = threads.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": threads, "meta": { "has_more": has_more } }),
    ))
}

#[derive(Debug, Serialize, FromRow)]
struct TopPosterRow {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub thread_count: i64,
    pub reply_count: i64,
}

/// GET /v1/forums/members — Top posters leaderboard (ranked by thread_count + reply_count).
pub async fn list_top_posters(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, TopPosterRow>(
        r#"
        WITH thread_counts AS (
            SELECT user_id, COUNT(*)::bigint AS cnt FROM forum_threads GROUP BY user_id
        ),
        reply_counts AS (
            SELECT user_id, COUNT(*)::bigint AS cnt FROM forum_replies GROUP BY user_id
        )
        SELECT u.id AS user_id, u.username, u.first_name, u.last_name, u.avatar, u.is_verified,
            COALESCE(tc.cnt, 0) AS thread_count,
            COALESCE(rc.cnt, 0) AS reply_count
        FROM users u
        LEFT JOIN thread_counts tc ON tc.user_id = u.id
        LEFT JOIN reply_counts rc  ON rc.user_id = u.id
        WHERE COALESCE(tc.cnt, 0) + COALESCE(rc.cnt, 0) > 0
        ORDER BY (COALESCE(tc.cnt, 0) + COALESCE(rc.cnt, 0)) DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "user": {
                    "id": r.user_id,
                    "username": r.username,
                    "first_name": r.first_name,
                    "last_name": r.last_name,
                    "avatar": r.avatar,
                    "is_verified": r.is_verified,
                },
                "thread_count": r.thread_count,
                "reply_count": r.reply_count,
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

/// GET /v1/forums/my/threads — Current user's forum threads.
pub async fn my_threads(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let threads = sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.id, t.forum_id, t.user_id, t.title, t.content, t.view_count, t.reply_count,
            t.last_reply_at, t.created_at, u.username, u.avatar
        FROM forum_threads t JOIN users u ON u.id = t.user_id
        WHERE t.user_id = $1
          AND ($2::bigint IS NULL OR t.id < $2)
        ORDER BY t.id DESC LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = threads.len() as i64 > limit;
    let threads: Vec<_> = threads.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": threads, "meta": { "has_more": has_more } }),
    ))
}

/// GET /v1/forums/my/replies — Current user's forum replies.
pub async fn my_replies(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let replies = sqlx::query_as::<_, ReplyRow>(
        r#"
        SELECT r.id, r.thread_id, r.user_id, r.content, r.quoted_reply_id, r.created_at,
            u.username, u.avatar
        FROM forum_replies r JOIN users u ON u.id = r.user_id
        WHERE r.user_id = $1
          AND ($2::bigint IS NULL OR r.id < $2)
        ORDER BY r.id DESC LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = replies.len() as i64 > limit;
    let replies: Vec<_> = replies.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": replies, "meta": { "has_more": has_more } }),
    ))
}
