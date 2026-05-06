use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct StoryRow {
    pub id: i64,
    pub user_id: i64,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct StoryMediaRow {
    pub id: i64,
    pub story_id: i64,
    pub media_type: String,
    pub media_url: String,
    pub thumbnail_url: Option<String>,
    pub description: String,
    pub duration: Option<i32>,
    /// CSS filter string (e.g. "sepia(0.9)") applied by the viewer. Plan §3.3 — S3.
    pub filter_css: Option<String>,
    /// CSS color for the caption overlay. Plan §3.3 — S2.
    pub text_style_color: Option<String>,
    /// CSS font-family stack for the caption overlay. Plan §3.3 — S2.
    pub text_style_font: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct UploadedMediaRow {
    pub file_url: String,
    pub file_type: String,
    pub duration: Option<i32>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct StoryViewerRow {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub viewed_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct StoryResponse {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub media: Vec<StoryMediaRow>,
    pub view_count: i64,
    pub has_viewed: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct StoryUserRow {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn list_stories(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    // Get active stories from followed users + own stories
    let story_users = sqlx::query_as::<_, StoryUserRow>(
        r#"
        SELECT s.user_id, u.username, u.first_name, u.last_name, u.avatar
        FROM stories s
        JOIN users u ON u.id = s.user_id
        WHERE s.expires_at > NOW()
          AND (
              s.user_id = $1
              OR s.user_id IN (
                  SELECT following_id FROM follows WHERE follower_id = $1 AND status = 'active'
              )
          )
          AND s.user_id NOT IN (
              SELECT muted_id FROM mutes WHERE user_id = $1 AND mute_type = 'story'
          )
        GROUP BY s.user_id, u.username, u.first_name, u.last_name, u.avatar
        ORDER BY
            CASE WHEN s.user_id = $1 THEN 0 ELSE 1 END,
            MAX(s.created_at) DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mut result = Vec::with_capacity(story_users.len());
    for user in &story_users {
        let stories = sqlx::query_as::<_, StoryRow>(
            "SELECT id, user_id, created_at, expires_at FROM stories WHERE user_id = $1 AND expires_at > NOW() ORDER BY created_at",
        )
        .bind(user.user_id)
        .fetch_all(&state.db)
        .await?;

        let mut all_media = Vec::new();
        let mut total_views: i64 = 0;
        let mut has_viewed = true;

        for story in &stories {
            let media = sqlx::query_as::<_, StoryMediaRow>(
                "SELECT id, story_id, media_type, media_url, thumbnail_url, description, duration, filter_css, text_style_color, text_style_font, created_at FROM story_media WHERE story_id = $1 ORDER BY created_at",
            )
            .bind(story.id)
            .fetch_all(&state.db)
            .await?;

            for m in &media {
                let viewed = sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(SELECT 1 FROM story_views WHERE story_media_id = $1 AND user_id = $2)",
                )
                .bind(m.id)
                .bind(auth.user_id)
                .fetch_one(&state.db)
                .await?;

                if !viewed {
                    has_viewed = false;
                }

                let view_count = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM story_views WHERE story_media_id = $1",
                )
                .bind(m.id)
                .fetch_one(&state.db)
                .await?;

                total_views += view_count;
            }

            all_media.extend(media);
        }

        if let Some(first_story) = stories.first() {
            result.push(StoryResponse {
                id: first_story.id,
                user_id: user.user_id,
                username: user.username.clone(),
                first_name: user.first_name.clone(),
                last_name: user.last_name.clone(),
                avatar: user.avatar.clone(),
                created_at: first_story.created_at,
                expires_at: stories
                    .last()
                    .map(|s| s.expires_at)
                    .unwrap_or(first_story.expires_at),
                media: all_media,
                view_count: total_views,
                has_viewed,
            });
        }
    }

    Ok(Json(json!({ "data": result })))
}

pub async fn create_story(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut media_id: Option<i64> = None;
    let mut content_type = String::new();
    let mut description = String::new();
    let mut filter_css: Option<String> = None;
    let mut text_style_color: Option<String> = None;
    let mut text_style_font: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        match field.name() {
            Some("media") => {
                content_type = field.content_type().unwrap_or("image/jpeg").to_string();

                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;

                if data.len() > 20 * 1024 * 1024 {
                    return Err(ApiError::BadRequest(
                        "Story media must be under 20 MB".into(),
                    ));
                }
                file_data = Some(data.to_vec());
            }
            Some("description") => {
                description = field.text().await.unwrap_or_default();
            }
            Some("media_id") => {
                media_id = field
                    .text()
                    .await
                    .ok()
                    .and_then(|value| value.trim().parse::<i64>().ok());
            }
            Some("filter_css") => {
                let v = field.text().await.unwrap_or_default();
                // Sanity-limit and reject suspicious content. CSS filter values
                // are short; anything above 256 chars is almost certainly bogus.
                if !v.is_empty() && v.len() <= 256 && !v.contains(['<', '>', ';']) {
                    filter_css = Some(v);
                }
            }
            Some("text_style_color") => {
                let v = field.text().await.unwrap_or_default();
                // Accept `#rgb`, `#rrggbb`, or a few named colors to avoid CSS injection.
                if (v.starts_with('#') && v.len() <= 9)
                    || v.chars().all(|c| c.is_ascii_alphabetic())
                {
                    text_style_color = Some(v);
                }
            }
            Some("text_style_font") => {
                let v = field.text().await.unwrap_or_default();
                if !v.is_empty() && v.len() <= 128 && !v.contains([';', '{', '}']) {
                    text_style_font = Some(v);
                }
            }
            _ => {}
        }
    }

    let mut tx = state.db.begin().await?;

    // Find or create today's active story for this user
    let story_id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM stories WHERE user_id = $1 AND expires_at > NOW() ORDER BY created_at DESC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let story_id = if let Some(id) = story_id {
        id
    } else {
        sqlx::query_scalar::<_, i64>("INSERT INTO stories (user_id) VALUES ($1) RETURNING id")
            .bind(auth.user_id)
            .fetch_one(&mut *tx)
            .await?
    };

    let (media_type, media_url, thumbnail_url, duration) = if let Some(existing_media_id) = media_id
    {
        let media = sqlx::query_as::<_, UploadedMediaRow>(
            r#"
            SELECT file_url, file_type, duration, thumbnail_url
            FROM uploaded_media
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(existing_media_id)
        .bind(auth.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::NotFound("Uploaded media not found".into()))?;

        if media.file_type != "image" && media.file_type != "video" {
            return Err(ApiError::BadRequest(
                "Stories only support image or video media".into(),
            ));
        }

        let duration = if media.file_type == "video" {
            Some(media.duration.unwrap_or(15))
        } else {
            Some(5)
        };

        (
            media.file_type,
            media.file_url,
            media.thumbnail_url,
            duration,
        )
    } else {
        let data = file_data.ok_or_else(|| {
            ApiError::BadRequest("Provide either media or media_id when creating a story".into())
        })?;

        let media_type = if content_type.starts_with("video/") {
            "video".to_string()
        } else {
            "image".to_string()
        };

        let ext = mime_to_ext(&content_type);
        let unique_name = format!("stories/{}/{}.{}", auth.user_id, uuid::Uuid::new_v4(), ext);

        let storage = shared::storage::create_storage().await;
        let file_url = storage
            .upload(&unique_name, &data, &content_type)
            .await?;

        let duration = if media_type == "video" {
            Some(15)
        } else {
            Some(5)
        };

        (media_type, file_url, None, duration)
    };

    let story_media = sqlx::query_as::<_, StoryMediaRow>(
        r#"
        INSERT INTO story_media (
            story_id, media_type, media_url, thumbnail_url, description, duration,
            filter_css, text_style_color, text_style_font
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, story_id, media_type, media_url, thumbnail_url, description, duration,
                  filter_css, text_style_color, text_style_font, created_at
        "#,
    )
    .bind(story_id)
    .bind(&media_type)
    .bind(&media_url)
    .bind(&thumbnail_url)
    .bind(&description)
    .bind(duration)
    .bind(&filter_css)
    .bind(&text_style_color)
    .bind(&text_style_font)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let _ = state
        .event_bus
        .publish(&DomainEvent::StoryCreated {
            story_id,
            user_id: auth.user_id,
        })
        .await;

    Ok(Json(json!({
        "data": {
            "story_id": story_id,
            "media": story_media
        }
    })))
}

pub async fn get_story(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let story = sqlx::query_as::<_, StoryRow>(
        "SELECT id, user_id, created_at, expires_at FROM stories WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Story not found".into()))?;

    let media = sqlx::query_as::<_, StoryMediaRow>(
        "SELECT id, story_id, media_type, media_url, thumbnail_url, description, duration, filter_css, text_style_color, text_style_font, created_at FROM story_media WHERE story_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let user = sqlx::query_as::<_, StoryUserRow>(
        "SELECT id AS user_id, username, first_name, last_name, avatar FROM users WHERE id = $1",
    )
    .bind(story.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "id": story.id,
            "user": user,
            "created_at": story.created_at,
            "expires_at": story.expires_at,
            "media": media,
            "is_own": story.user_id == auth.user_id
        }
    })))
}

pub async fn delete_story(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let story = sqlx::query_as::<_, StoryRow>(
        "SELECT id, user_id, created_at, expires_at FROM stories WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Story not found".into()))?;

    if story.user_id != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    // Cascade deletes story_media and story_views
    sqlx::query("DELETE FROM stories WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn view_story(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // id here is story_media_id
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM story_media WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    if !exists {
        return Err(ApiError::NotFound("Story media not found".into()));
    }

    sqlx::query(
        "INSERT INTO story_views (story_media_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "viewed": true } })))
}

pub async fn get_viewers(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Only story owner can see viewers
    let owner_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT s.user_id FROM stories s
        JOIN story_media sm ON sm.story_id = s.id
        WHERE sm.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Story media not found".into()))?;

    if owner_id != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    let viewers = sqlx::query_as::<_, StoryViewerRow>(
        r#"
        SELECT sv.user_id, u.username, u.first_name, u.last_name, u.avatar, sv.viewed_at
        FROM story_views sv
        JOIN users u ON u.id = sv.user_id
        WHERE sv.story_media_id = $1
        ORDER BY sv.viewed_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": viewers })))
}

pub async fn my_stories(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let stories = sqlx::query_as::<_, StoryRow>(
        "SELECT id, user_id, created_at, expires_at FROM stories WHERE user_id = $1 AND expires_at > NOW() ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mut result = Vec::with_capacity(stories.len());
    for story in stories {
        let media = sqlx::query_as::<_, StoryMediaRow>(
            "SELECT id, story_id, media_type, media_url, thumbnail_url, description, duration, created_at FROM story_media WHERE story_id = $1 ORDER BY created_at",
        )
        .bind(story.id)
        .fetch_all(&state.db)
        .await?;

        result.push(json!({
            "id": story.id,
            "created_at": story.created_at,
            "expires_at": story.expires_at,
            "media": media
        }));
    }

    Ok(Json(json!({ "data": result })))
}

pub async fn archived_stories(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();
    let fetch_limit = limit + 1;

    let stories = sqlx::query_as::<_, StoryRow>(
        r#"
        SELECT id, user_id, created_at, expires_at
        FROM stories
        WHERE user_id = $1 AND expires_at <= NOW()
          AND ($2::bigint IS NULL OR id < $2)
        ORDER BY id DESC LIMIT $3
        "#,
    )
    .bind(auth.user_id)
    .bind(cursor)
    .bind(fetch_limit)
    .fetch_all(&state.db)
    .await?;

    let has_more = stories.len() as i64 > limit;
    let stories: Vec<_> = stories.into_iter().take(limit as usize).collect();
    let next_cursor = stories.last().map(|s| s.id.to_string());

    let mut result = Vec::with_capacity(stories.len());
    for story in stories {
        let media = sqlx::query_as::<_, StoryMediaRow>(
            "SELECT id, story_id, media_type, media_url, thumbnail_url, description, duration, created_at FROM story_media WHERE story_id = $1 ORDER BY created_at",
        )
        .bind(story.id)
        .fetch_all(&state.db)
        .await?;

        result.push(json!({
            "id": story.id,
            "created_at": story.created_at,
            "expires_at": story.expires_at,
            "media": media
        }));
    }

    Ok(Json(json!({
        "data": result,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ─── Story Reactions & Replies ───────────────────────────────────────────────

pub async fn react_to_story(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<StoryReactionRequest>,
) -> Result<Json<Value>, ApiError> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM story_media WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    if !exists {
        return Err(ApiError::NotFound("Story media not found".into()));
    }

    sqlx::query(
        r#"INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
        VALUES ($1, 'story', $2, $3)
        ON CONFLICT (user_id, target_type, target_id) DO UPDATE SET reaction_type = $3"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&req.reaction)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "reacted": true } })))
}

pub async fn list_story_reactions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Only story owner can see reactions
    let owner_id = sqlx::query_scalar::<_, i64>(
        "SELECT s.user_id FROM stories s JOIN story_media sm ON sm.story_id = s.id WHERE sm.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Story media not found".into()))?;

    if owner_id != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>, OffsetDateTime)>(
        r#"SELECT r.user_id, r.reaction_type, u.username, u.first_name, u.avatar, r.created_at
        FROM reactions r JOIN users u ON u.id = r.user_id
        WHERE r.target_type = 'story' AND r.target_id = $1
        ORDER BY r.created_at DESC"#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(
            |(uid, reaction, username, first_name, avatar, created_at)| {
                json!({
                    "user_id": uid, "reaction": reaction,
                    "username": username, "first_name": first_name,
                    "avatar": avatar, "created_at": created_at.to_string()
                })
            },
        )
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn reply_to_story(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<StoryReplyRequest>,
) -> Result<Json<Value>, ApiError> {
    // Verify story_media exists and get owner
    let owner_id = sqlx::query_scalar::<_, i64>(
        "SELECT s.user_id FROM stories s JOIN story_media sm ON sm.story_id = s.id WHERE sm.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Story media not found".into()))?;

    // Replies go as a DM to the story owner
    // Find or create a direct conversation between these two users
    let conv_id = sqlx::query_scalar::<_, i64>(
        r#"SELECT cm1.conversation_id
        FROM conversation_members cm1
        JOIN conversation_members cm2 ON cm1.conversation_id = cm2.conversation_id
        JOIN conversations c ON c.id = cm1.conversation_id
        WHERE cm1.user_id = $1 AND cm2.user_id = $2 AND c.type = 'direct'
        LIMIT 1"#,
    )
    .bind(auth.user_id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await?;

    let conv_id = if let Some(cid) = conv_id {
        cid
    } else {
        let mut tx = state.db.begin().await?;
        let cid = sqlx::query_scalar::<_, i64>(
            "INSERT INTO conversations (type) VALUES ('direct') RETURNING id",
        )
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO conversation_members (conversation_id, user_id, role) VALUES ($1, $2, 'member'), ($1, $3, 'member')")
            .bind(cid)
            .bind(auth.user_id)
            .bind(owner_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        cid
    };

    // Insert the reply as a story_reply message type.
    // Messages schema uses `content` (not `text`) and `media` JSONB.
    let msg_id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO messages (sender_id, conversation_id, content, message_type, media)
        VALUES ($1, $2, $3, 'story_reply', $4)
        RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(conv_id)
    .bind(&req.text)
    .bind(json!([{ "story_media_id": id }]))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(
        json!({ "data": { "message_id": msg_id, "conversation_id": conv_id } }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct StoryReactionRequest {
    pub reaction: String,
}

#[derive(Debug, Deserialize)]
pub struct StoryReplyRequest {
    pub text: String,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn mime_to_ext(mime: &str) -> &str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        _ => "bin",
    }
}
