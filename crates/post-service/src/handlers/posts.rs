use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    events::DomainEvent,
};
use sqlx::{FromRow, Row};
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
    pub view_count: i64,
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

    // Enqueue for AI moderation (fire-and-forget — non-blocking)
    if !content.is_empty() {
        let db = state.db.clone();
        let user_id = auth.user_id;
        let post_id = post.id;
        let content_clone = content.clone();
        tokio::spawn(async move {
            let _ = shared::moderation::enqueue_moderation(
                &db, "post", post_id, user_id, &content_clone,
            )
            .await;
        });
    }

    // Publish event for notification/realtime services
    let _ = state
        .event_bus
        .publish(&DomainEvent::PostCreated {
            post_id: post.id,
            user_id: auth.user_id,
            group_id: req.group_id,
            page_id: req.page_id,
        })
        .await;

    // Inform any subscribed feed (home / group / page) that there is a new
    // post available, so the web client can show a "X new posts" banner.
    let feed_scope = match (req.group_id, req.page_id) {
        (Some(gid), _) => format!("group:{}", gid),
        (_, Some(pid)) => format!("page:{}", pid),
        _ => "home".to_string(),
    };
    let _ = state
        .event_bus
        .publish(&DomainEvent::NewPostsAvailable {
            feed_scope,
            count: 1,
        })
        .await;

    Ok(Json(serde_json::json!({ "data": post })))
}

pub async fn get_post(
    State(state): State<AppState>,
    auth: OptionalAuth,
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

    // Block access to only_me posts from non-owners
    let viewer_id = auth.0.as_ref().map(|u| u.user_id);
    if post.privacy == "only_me" && viewer_id != Some(post.user_id) {
        return Err(ApiError::NotFound("Post not found".into()));
    }

    // Increment view count (non-critical, log on failure)
    if let Err(e) = sqlx::query("UPDATE posts SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        tracing::warn!(error = %e, "Failed to increment view count");
    }

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

    // Re-enqueue for moderation if content changed
    if let Some(ref new_content) = req.content
        && !new_content.is_empty() {
            let db = state.db.clone();
            let user_id = auth.user_id;
            let post_id = post.id;
            let content_clone = new_content.clone();
            tokio::spawn(async move {
                let _ = shared::moderation::enqueue_moderation(
                    &db, "post", post_id, user_id, &content_clone,
                )
                .await;
            });
        }

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

    let _ = state
        .event_bus
        .publish(&DomainEvent::PostDeleted { post_id: id })
        .await;

    Ok(Json(
        serde_json::json!({ "data": { "message": "Post deleted" } }),
    ))
}

pub async fn save_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "INSERT INTO saved_posts (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(
        serde_json::json!({ "data": { "message": "Post saved" } }),
    ))
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

    Ok(Json(
        serde_json::json!({ "data": { "message": "Post unsaved" } }),
    ))
}

pub async fn get_saved_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<shared::pagination::PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit.unwrap_or(20).min(50);
    let cursor = params.cursor.and_then(|c| c.parse::<i64>().ok());

    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            String,
            String,
            i32,
            i32,
            i32,
            i64,
            String,
            String,
            String,
            bool,
            String,
            String,
        ),
    >(
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
    let items: Vec<_> = rows
        .into_iter()
        .take(limit as usize)
        .map(|r| {
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
        })
        .collect();

    let next_cursor = if has_more {
        items
            .last()
            .and_then(|i| i["id"].as_i64())
            .map(|id| id.to_string())
    } else {
        None
    };

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
    sqlx::query(
        "INSERT INTO hidden_posts (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(
        serde_json::json!({ "data": { "message": "Post hidden" } }),
    ))
}

// ── Co-Authors ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct InviteCoauthorRequest {
    pub user_id: i64,
}

pub async fn invite_coauthor(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(body): Json<InviteCoauthorRequest>,
) -> Result<Json<()>, ApiError> {
    // Verify the requester is the post owner
    let owner = sqlx::query("SELECT user_id FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(post_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("DB error".into())
        })?
        .ok_or(ApiError::NotFound("Post not found".into()))?;
    let owner_id: i64 = owner.get("user_id");
    if owner_id != auth.user_id {
        return Err(ApiError::Forbidden(
            "Only the post author can invite co-authors".into(),
        ));
    }

    sqlx::query(
        "INSERT INTO post_coauthors (post_id, user_id, status) VALUES ($1, $2, 'pending') ON CONFLICT DO NOTHING",
    )
    .bind(post_id)
    .bind(body.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    // Mark post as collaborative
    sqlx::query("UPDATE posts SET is_collaborative = TRUE WHERE id = $1")
        .bind(post_id)
        .execute(&state.db)
        .await?;

    Ok(Json(()))
}

pub async fn list_coauthors(
    State(state): State<AppState>,
    Path(post_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT pc.user_id, pc.status, pc.added_at, u.username, u.first_name, u.last_name, u.avatar
         FROM post_coauthors pc JOIN users u ON u.id = pc.user_id WHERE pc.post_id = $1",
    )
    .bind(post_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "user_id": r.get::<i64, _>("user_id"),
                "username": r.get::<String, _>("username"),
                "first_name": r.get::<String, _>("first_name"),
                "last_name": r.get::<String, _>("last_name"),
                "avatar": r.get::<Option<String>, _>("avatar"),
                "status": r.get::<String, _>("status"),
                "added_at": r.get::<String, _>("added_at"),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "data": items })))
}

pub async fn accept_coauthor_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let result = sqlx::query(
        "UPDATE post_coauthors SET status = 'accepted' WHERE post_id = $1 AND user_id = $2 AND status = 'pending'",
    )
    .bind(post_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(
            "No pending co-author invitation found".into(),
        ));
    }
    Ok(Json(()))
}

pub async fn remove_coauthor(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((post_id, user_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    // Only the post owner can remove co-authors
    let owner = sqlx::query("SELECT user_id FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(post_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("DB error".into())
        })?
        .ok_or(ApiError::NotFound("Post not found".into()))?;
    let owner_id: i64 = owner.get("user_id");
    if owner_id != auth.user_id {
        return Err(ApiError::Forbidden(
            "Only the post author can remove co-authors".into(),
        ));
    }

    sqlx::query("DELETE FROM post_coauthors WHERE post_id = $1 AND user_id = $2")
        .bind(post_id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("DB error".into())
        })?;
    Ok(Json(()))
}

// ── Post Translation ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct TranslatePostRequest {
    pub target_lang: String,
}

pub async fn translate_post(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(body): Json<TranslatePostRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validate language code against ISO 639-1/2 pattern before any use
    if body.target_lang.len() < 2
        || body.target_lang.len() > 5
        || !body.target_lang.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
    {
        return Err(ApiError::BadRequest("Invalid language code".into()));
    }

    // Check cache first
    let cached = sqlx::query(
        "SELECT translated_text FROM post_translations WHERE post_id = $1 AND lang = $2",
    )
    .bind(post_id)
    .bind(&body.target_lang)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    if let Some(row) = cached {
        return Ok(Json(serde_json::json!({
            "data": {
                "post_id": post_id,
                "lang": body.target_lang,
                "translated_text": row.get::<String, _>("translated_text"),
                "cached": true,
            }
        })));
    }

    // Get original content
    let post = sqlx::query("SELECT content FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("DB error".into())
        })?
        .ok_or(ApiError::NotFound("Post not found".into()))?;

    let content: String = post.get("content");
    if content.is_empty() {
        return Ok(Json(serde_json::json!({ "data": { "translated_text": "", "cached": false } })));
    }

    // Call OpenAI for translation
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err(ApiError::Internal(
            "Translation service not configured".into(),
        ));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "system", "content": format!("Translate the following text to {}. Only return the translation, nothing else.", body.target_lang)},
                {"role": "user", "content": content}
            ],
            "max_tokens": 1000,
            "temperature": 0.3,
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("Translation API error".into())
        })?;

    let json: serde_json::Value = resp.json().await.map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("Translation parse error".into())
    })?;

    let translated = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or(&content)
        .to_string();

    // Cache the translation (non-critical, log on failure)
    if let Err(e) = sqlx::query(
        "INSERT INTO post_translations (post_id, lang, translated_text) VALUES ($1, $2, $3) ON CONFLICT (post_id, lang) DO UPDATE SET translated_text = $3",
    )
    .bind(post_id)
    .bind(&body.target_lang)
    .bind(&translated)
    .execute(&state.db)
    .await
    {
        tracing::warn!(error = %e, "Failed to cache translation");
    }

    Ok(Json(serde_json::json!({
        "data": {
            "post_id": post_id,
            "lang": body.target_lang,
            "translated_text": translated,
            "cached": false,
        }
    })))
}
