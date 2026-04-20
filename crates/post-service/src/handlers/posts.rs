use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    events::DomainEvent,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, FromRow)]
pub struct PostRow {
    pub id: i64,
    pub uuid: Uuid,
    pub user_id: i64,
    pub parent_id: Option<i64>,
    pub content: String,
    pub post_type: String,
    pub media: serde_json::Value,
    pub privacy: String,
    pub feeling: String,
    pub location: String,
    pub is_pinned: bool,
    pub is_boosted: bool,
    pub is_reel: bool,
    pub like_count: i32,
    pub comment_count: i32,
    pub share_count: i32,
    pub view_count: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(max = 63206))]
    pub content: Option<String>,
    pub privacy: Option<String>,
    pub media: Option<serde_json::Value>,
    pub feeling: Option<String>,
    pub location: Option<String>,
    pub colored_post: Option<serde_json::Value>,
    pub page_id: Option<i64>,
    pub group_id: Option<i64>,
    pub event_id: Option<i64>,
    pub is_reel: Option<bool>,
}

pub async fn create_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let content = req.content.unwrap_or_default();
    let privacy = req.privacy.as_deref().unwrap_or("everyone");
    let media = req.media.unwrap_or(serde_json::json!([]));
    let feeling = req.feeling.unwrap_or_default();
    let location = req.location.unwrap_or_default();
    let is_reel = req.is_reel.unwrap_or(false);

    let post_type = if is_reel {
        "reel"
    } else if media.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
        "media"
    } else {
        "text"
    };

    let post = sqlx::query_as::<_, PostRow>(
        r#"INSERT INTO posts (user_id, content, post_type, media, privacy, feeling, location,
                              page_id, group_id, event_id, colored_post, is_reel)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           RETURNING id, uuid, user_id, parent_id, content, post_type, media, privacy, feeling,
                     location, is_pinned, is_boosted, is_reel, like_count, comment_count,
                     share_count, view_count, created_at, updated_at"#,
    )
    .bind(auth.user_id)
    .bind(&content)
    .bind(post_type)
    .bind(&media)
    .bind(privacy)
    .bind(&feeling)
    .bind(&location)
    .bind(req.page_id)
    .bind(req.group_id)
    .bind(req.event_id)
    .bind(&req.colored_post)
    .bind(is_reel)
    .fetch_one(&state.db)
    .await?;

    // Publish event for notification/realtime services
    let _ = state.event_bus.publish(&DomainEvent::PostCreated {
        post_id: post.id,
        user_id: auth.user_id,
        group_id: req.group_id,
        page_id: req.page_id,
    }).await;

    Ok(Json(serde_json::json!({ "data": post })))
}

pub async fn get_post(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let post = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, user_id, parent_id, content, post_type, media, privacy, feeling,
                  location, is_pinned, is_boosted, is_reel, like_count, comment_count,
                  share_count, view_count, created_at, updated_at
           FROM posts WHERE id = $1 AND deleted_at IS NULL"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Post not found".into()))?;

    // Increment view count
    let _ = sqlx::query("UPDATE posts SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await;

    // Load publisher info
    let publisher = sqlx::query_as::<_, shared::models::user::PublicUserRow>(
        "SELECT uuid, username, first_name, last_name, avatar, cover, about, is_verified, is_pro FROM users WHERE id = $1",
    )
    .bind(post.user_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "post": post,
            "publisher": publisher,
        }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePostRequest {
    #[validate(length(max = 63206))]
    pub content: Option<String>,
    pub privacy: Option<String>,
}

pub async fn update_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePostRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    let post = sqlx::query_as::<_, PostRow>(
        r#"UPDATE posts SET
            content = COALESCE($3, content),
            privacy = COALESCE($4, privacy),
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL
        RETURNING id, uuid, user_id, parent_id, content, post_type, media, privacy, feeling,
                  location, is_pinned, is_boosted, is_reel, like_count, comment_count,
                  share_count, view_count, created_at, updated_at"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.content)
    .bind(&req.privacy)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Post not found or access denied".into()))?;

    Ok(Json(serde_json::json!({ "data": post })))
}

pub async fn delete_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Post not found or access denied".into()));
    }

    let _ = state.event_bus.publish(&DomainEvent::PostDeleted { post_id: id }).await;

    Ok(Json(serde_json::json!({ "data": { "message": "Post deleted" } })))
}

pub async fn save_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("INSERT INTO saved_posts (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "data": { "message": "Post saved" } })))
}

pub async fn unsave_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM saved_posts WHERE user_id = $1 AND post_id = $2")
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "data": { "message": "Post unsaved" } })))
}

pub async fn get_saved_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<shared::pagination::PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit.unwrap_or(20).min(50);
    let cursor = params.cursor.and_then(|c| c.parse::<i64>().ok());

    let rows = sqlx::query_as::<_, (i64, String, String, String, String, i32, i32, i32, i64, String, String, String, bool, String, String)>(
        r#"SELECT p.id, p.uuid, p.content, p.post_type, p.privacy,
            p.like_count, p.comment_count, p.share_count,
            u.id as user_id, u.username, u.first_name, u.last_name, u.is_verified,
            u.avatar, p.created_at::text
        FROM saved_posts sp
        JOIN posts p ON sp.post_id = p.id
        JOIN users u ON p.user_id = u.id
        WHERE sp.user_id = $1 AND p.deleted_at IS NULL
        AND ($2::bigint IS NULL OR sp.post_id < $2)
        ORDER BY sp.created_at DESC
        LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).map(|r| {
        serde_json::json!({
            "id": r.0, "uuid": r.1, "content": r.2, "post_type": r.3,
            "privacy": r.4, "like_count": r.5, "comment_count": r.6,
            "share_count": r.7, "is_saved": true,
            "publisher": {
                "id": r.8, "username": r.9, "first_name": r.10,
                "last_name": r.11, "is_verified": r.12, "avatar": r.13
            },
            "created_at": r.14
        })
    }).collect();

    let next_cursor = if has_more { items.last().and_then(|i| i["id"].as_i64()).map(|id| id.to_string()) } else { None };

    Ok(Json(serde_json::json!({
        "data": items,
        "meta": { "has_more": has_more, "cursor": next_cursor }
    })))
}

pub async fn hide_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("INSERT INTO hidden_posts (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "data": { "message": "Post hidden" } })))
}
