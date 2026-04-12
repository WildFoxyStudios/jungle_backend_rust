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
