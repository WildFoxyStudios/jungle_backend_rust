use std::collections::HashSet;
use std::time::Instant;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use redis::AsyncCommands;
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    events::DomainEvent,
    metrics::{REELS_FEED_DURATION_SECONDS, REELS_UNIQUE_VIEWS_TOTAL},
    pagination::PaginationParams,
};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use super::comments;

/// Parse a `site_config` text row; invalid/empty = feature allowed (default on).
fn parse_site_config_flag(opt: Option<String>) -> Option<bool> {
    let raw = opt?.trim().to_lowercase();
    if raw.is_empty() {
        return None;
    }
    match raw.as_str() {
        "false" | "0" | "no" | "off" => Some(false),
        "true" | "1" | "yes" | "on" => Some(true),
        _ => None,
    }
}

async fn require_reels_feature(state: &AppState) -> Result<(), ApiError> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT value FROM site_config WHERE category = 'features' AND key = 'reels' LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await?;

    if parse_site_config_flag(raw) == Some(false) {
        return Err(ApiError::Forbidden("Reels feature disabled".into()));
    }
    Ok(())
}

fn is_missing_reel_views_table(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db) = e
        && db.code().as_deref() == Some("42P01") {
            return db.message().to_lowercase().contains("reel_views");
        }
    false
}

#[derive(Debug, Deserialize)]
pub struct ReelsFeedQuery {
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
    /// `following` = reels only from followed accounts; default = ranked mix (For you).
    pub filter: Option<String>,
}

/// Row layout shared by `get_reels_feed` and `get_reel`. The JSON response
/// derived from this row matches the frontend `Reel` type (@jungle/api-client)
/// so the UI can consume the payload directly without normalization.
///
/// A struct is used instead of a tuple because sqlx only implements `FromRow`
/// on tuples up to 16 fields and we need more columns for reel audio metadata.
#[derive(Debug, FromRow)]
pub struct ReelRow {
    id: i64,
    user_id: i64,
    content: Option<String>,
    media: Option<Value>,
    like_count: i32,
    comment_count: i32,
    share_count: i32,
    view_count: i64,
    /// 0 = comments allowed, 1 = disabled
    comments_status: i16,
    uuid: Uuid,
    username: String,
    first_name: String,
    last_name: String,
    avatar: String,
    is_verified: bool,
    is_online: bool,
    is_pro: i16,
    my_reaction: Option<String>,
    /// Viewer follows the reel author (false for own reels / no session).
    is_following: bool,
    /// Viewer saved this post (reel) to bookmarks.
    is_saved: bool,
    audio_track_id: Option<i64>,
    audio_track_title: Option<String>,
    audio_track_artist: Option<String>,
    audio_track_source: Option<String>,
    remix_of_post_id: Option<i64>,
    allow_remix: bool,
    created_at: time::OffsetDateTime,
}

/// Build the public JSON payload for a single reel. Splits the JSONB `media`
/// array into a primary `video` MediaItem plus a `thumbnail` URL so the
/// frontend can render `<video src>` + poster directly.
pub fn reel_to_json(row: ReelRow) -> Value {
    let first_media = row
        .media
        .as_ref()
        .and_then(|m| m.as_array())
        .and_then(|a| a.first())
        .cloned();

    let thumbnail = first_media
        .as_ref()
        .and_then(|m| {
            m.get("thumbnail_url")
                .or_else(|| m.get("thumbnail"))
                .or_else(|| m.get("poster_url"))
        })
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let video = first_media.unwrap_or_else(|| {
        json!({
            "id": 0,
            "url": "",
            "type": "video",
            "mime": "video/mp4",
            "thumbnail": thumbnail,
        })
    });

    let audio = match row.audio_track_id {
        Some(tid) => json!({
            "id": tid,
            "title": row.audio_track_title.as_deref().unwrap_or(""),
            "artist_label": row.audio_track_artist.as_deref().unwrap_or(""),
            "source": row.audio_track_source.as_deref().unwrap_or(""),
        }),
        None => Value::Null,
    };

    json!({
        "id": row.id,
        "user_id": row.user_id,
        "caption": row.content.unwrap_or_default(),
        "video": video,
        "thumbnail": thumbnail,
        "like_count": row.like_count,
        "comment_count": row.comment_count,
        "share_count": row.share_count,
        "view_count": row.view_count,
        "comments_status": row.comments_status,
        "my_reaction": row.my_reaction,
        "is_following": row.is_following,
        "is_saved": row.is_saved,
        "audio": audio,
        "remix_of_post_id": row.remix_of_post_id,
        "allow_remix": row.allow_remix,
        "publisher": {
            "id": row.user_id,
            "uuid": row.uuid.to_string(),
            "username": row.username,
            "first_name": row.first_name,
            "last_name": row.last_name,
            "avatar": row.avatar,
            "is_verified": row.is_verified,
            "is_online": row.is_online,
            "is_pro": row.is_pro,
        },
        "created_at": row.created_at.to_string(),
    })
}

/// GET /v1/reels — trending/random reels feed
#[tracing::instrument(skip(state, auth, q), err)]
pub async fn get_reels_feed(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ReelsFeedQuery>,
) -> Result<Json<Value>, ApiError> {
    let feed_start = Instant::now();
    require_reels_feature(&state).await?;
    let limit = q.limit.unwrap_or(20).clamp(1, 50);

    let follow_clause = if q.filter.as_deref() == Some("following") {
        "AND p.user_id IN (SELECT following_id FROM follows WHERE follower_id = $1 AND status = 'active')"
    } else {
        ""
    };

    let base_where = format!(
        r#"
        p.is_reel = TRUE
          AND p.deleted_at IS NULL
          AND p.is_approved = TRUE
          AND (
            p.privacy = 'everyone'
            OR (p.privacy = 'followers' AND EXISTS (
                 SELECT 1 FROM follows f
                 WHERE f.following_id = p.user_id AND f.follower_id = $1 AND f.status = 'active'))
            OR (p.privacy = 'only_me' AND p.user_id = $1)
          )
          AND p.user_id NOT IN (
              SELECT blocked_id FROM blocks WHERE blocker_id = $1
              UNION
              SELECT blocker_id FROM blocks WHERE blocked_id = $1
          )
          {follow_clause}
          AND ($2::bigint IS NULL OR p.id < $2)"#,
        follow_clause = follow_clause
    );

    let sql_unseen = format!(
        r#"
        SELECT p.id, p.user_id, p.content, p.media,
               p.like_count, p.comment_count, p.share_count, p.view_count, p.comments_status,
               u.uuid, u.username, u.first_name, u.last_name, u.avatar,
               u.is_verified, u.is_online, u.is_pro,
               r.reaction_type AS my_reaction,
               EXISTS(SELECT 1 FROM follows f WHERE f.follower_id = $1 AND f.following_id = p.user_id AND f.status = 'active' AND p.user_id <> $1) AS is_following,
               EXISTS(SELECT 1 FROM saved_posts sp WHERE sp.user_id = $1 AND sp.post_id = p.id) AS is_saved,
               p.audio_track_id, at.title AS audio_track_title, at.artist_label AS audio_track_artist, at.source AS audio_track_source, p.remix_of_post_id, p.allow_remix,
               p.created_at
        FROM posts p
        JOIN users u ON u.id = p.user_id
        LEFT JOIN reel_audio_tracks at ON at.id = p.audio_track_id
        LEFT JOIN reactions r
               ON r.target_type = 'post'
              AND r.target_id = p.id
              AND r.user_id = $1
        WHERE {base_where}
          AND p.id NOT IN (SELECT post_id FROM reel_views WHERE user_id = $1)
        ORDER BY (p.like_count + p.comment_count + p.view_count) DESC, p.created_at DESC
        LIMIT $3
        "#
    );

    let sql_all = format!(
        r#"
        SELECT p.id, p.user_id, p.content, p.media,
               p.like_count, p.comment_count, p.share_count, p.view_count, p.comments_status,
               u.uuid, u.username, u.first_name, u.last_name, u.avatar,
               u.is_verified, u.is_online, u.is_pro,
               r.reaction_type AS my_reaction,
               EXISTS(SELECT 1 FROM follows f WHERE f.follower_id = $1 AND f.following_id = p.user_id AND f.status = 'active' AND p.user_id <> $1) AS is_following,
               EXISTS(SELECT 1 FROM saved_posts sp WHERE sp.user_id = $1 AND sp.post_id = p.id) AS is_saved,
               p.audio_track_id, at.title AS audio_track_title, at.artist_label AS audio_track_artist, at.source AS audio_track_source, p.remix_of_post_id, p.allow_remix,
               p.created_at
        FROM posts p
        JOIN users u ON u.id = p.user_id
        LEFT JOIN reel_audio_tracks at ON at.id = p.audio_track_id
        LEFT JOIN reactions r
               ON r.target_type = 'post'
              AND r.target_id = p.id
              AND r.user_id = $1
        WHERE {base_where}
        ORDER BY (p.like_count + p.comment_count + p.view_count) DESC, p.created_at DESC
        LIMIT $3
        "#
    );

    let mut rows = match sqlx::query_as::<_, ReelRow>(&sql_unseen)
        .bind(auth.user_id)
        .bind(q.cursor)
        .bind(limit + 1)
        .fetch_all(&state.db)
        .await
    {
        Ok(r) => r,
        Err(e) if is_missing_reel_views_table(&e) => {
            tracing::warn!(
                "reel_views not found (apply migration 20260428000002_reel_views). Using full reel list without unseen filter."
            );
            sqlx::query_as::<_, ReelRow>(&sql_all)
                .bind(auth.user_id)
                .bind(q.cursor)
                .bind(limit + 1)
                .fetch_all(&state.db)
                .await?
        }
        Err(e) => return Err(e.into()),
    };

    if rows.is_empty() && q.cursor.is_none() {
        rows = sqlx::query_as::<_, ReelRow>(&sql_all)
            .bind(auth.user_id)
            .bind(q.cursor)
            .bind(limit + 1)
            .fetch_all(&state.db)
            .await?;
    }

    let has_more = rows.len() as i64 > limit;
    let data: Vec<Value> = rows
        .into_iter()
        .take(limit as usize)
        .map(reel_to_json)
        .collect();

    let next_cursor = data.last().and_then(|d| d["id"].as_i64());

    REELS_FEED_DURATION_SECONDS.observe(feed_start.elapsed().as_secs_f64());
    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// POST /v1/reels/{id}/view — unique view per (user, reel); requires auth
#[tracing::instrument(skip(state, auth), err)]
pub async fn view_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1 AND is_reel = TRUE AND deleted_at IS NULL)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if !exists {
        return Err(ApiError::NotFound("Reel not found".into()));
    }

    let vkey = format!("reelv:{}:{}", auth.user_id, id);
    let mut r = state.redis.clone();
    let set_ok: Option<String> = redis::cmd("SET")
        .arg(&vkey)
        .arg(1i32)
        .arg("EX")
        .arg(1i32)
        .arg("NX")
        .query_async(&mut r)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if set_ok.is_none() {
        return Err(ApiError::RateLimited);
    }

    let mut tx = state.db.begin().await?;

    let first_time = sqlx::query_scalar::<_, bool>(
        r#"
        INSERT INTO reel_views (post_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (post_id, user_id) DO NOTHING
        RETURNING true
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .is_some();

    if first_time {
        sqlx::query("UPDATE posts SET view_count = COALESCE(view_count, 0) + 1 WHERE id = $1 AND is_reel = TRUE")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        REELS_UNIQUE_VIEWS_TOTAL.inc();
    }

    tx.commit().await?;

    Ok(Json(json!({ "data": { "viewed": true, "counted": first_time } })))
}

/// DELETE /v1/reels/views — clear this user's reel view history (re-show unseen in feed)
#[tracing::instrument(skip(state, auth), err)]
pub async fn clear_reel_views(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let res = sqlx::query("DELETE FROM reel_views WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "data": { "deleted": res.rows_affected() }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReelRequest {
    pub content: Option<String>,
    pub media: Option<Value>,
    pub privacy: Option<String>,
    /// 0 = comments allowed, 1 = disabled
    pub comments_status: Option<i16>,
    pub audio_track_id: Option<i64>,
    pub remix_of_post_id: Option<i64>,
    pub template_key: Option<String>,
    /// When false, others cannot create reels that reference this one as remix source.
    #[serde(default)]
    pub allow_remix: Option<bool>,
}

async fn link_hashtags_from_caption(
    state: &AppState,
    post_id: i64,
    text: &str,
) -> Result<(), ApiError> {
    let mut seen = HashSet::new();
    for word in text.split_whitespace() {
        let w = word
            .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let Some(stripped) = w.strip_prefix('#') else {
            continue;
        };
        if stripped.is_empty() || !stripped.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let tag = (if stripped.len() > 200 {
            stripped[..200].to_string()
        } else {
            stripped.to_string()
        })
        .to_lowercase();
        if !seen.insert(tag.clone()) {
            continue;
        }
        let hid: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO hashtags (tag, use_count, last_used_at)
            VALUES ($1, 1, NOW())
            ON CONFLICT (tag) DO UPDATE SET use_count = hashtags.use_count + 1, last_used_at = NOW()
            RETURNING id
            "#,
        )
        .bind(&tag)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
        let _ = sqlx::query(
            "INSERT INTO post_hashtags (post_id, hashtag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(post_id)
        .bind(hid)
        .execute(&state.db)
        .await;
    }
    Ok(())
}

/// Deduplicated lowercase usernames from `@handle` tokens in a caption.
fn mention_user_keys_from_caption(text: &str) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let w = word.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if let Some(tail) = w.strip_prefix('@') {
            let uname: String = tail
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if uname.is_empty() || uname.len() > 64 {
                continue;
            }
            let key = uname.to_lowercase();
            if seen.insert(key.clone()) {
                out.push(key);
            }
        }
    }
    out
}

/// Resolve `@handle` in caption; publishes `UserMentionedInPost` (deduplicated) for existing users.
async fn link_mentions_from_caption(
    state: &AppState,
    post_id: i64,
    author_id: i64,
    text: &str,
) -> Result<(), ApiError> {
    for key in mention_user_keys_from_caption(text) {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL LIMIT 1",
        )
        .bind(&key)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
        if let Some((mentioned_id,)) = row {
            if mentioned_id == author_id {
                continue;
            }
            let _ = state
                .event_bus
                .publish(&DomainEvent::UserMentionedInPost {
                    post_id,
                    mentioner_id: author_id,
                    mentioned_user_id: mentioned_id,
                })
                .await;
        }
    }
    Ok(())
}

/// POST /v1/reels — create a reel (a post with is_reel = true)
#[tracing::instrument(skip(state, auth, req), err)]
pub async fn create_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateReelRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    require_reels_feature(&state).await?;
    req.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let mut redis = state.redis.clone();
    let key = format!("reel_hr:{}", auth.user_id);
    let n: u64 = redis
        .incr(&key, 1)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if n == 1u64 {
        let _: () = redis.expire(&key, 3600i64).await.unwrap_or(());
    }
    if n > 5 {
        return Err(ApiError::RateLimited);
    }

    let privacy = req.privacy.as_deref().unwrap_or("everyone");
    if !["everyone", "followers", "only_me"].contains(&privacy) {
        return Err(ApiError::BadRequest("Invalid privacy".into()));
    }
    let comments_st = req.comments_status.unwrap_or(0).clamp(0, 1);
    let allow_remix = req.allow_remix.unwrap_or(true);

    if let Some(tid) = req.audio_track_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM reel_audio_tracks WHERE id = $1)",
        )
        .bind(tid)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);
        if !ok {
            return Err(ApiError::BadRequest("Invalid audio_track_id".into()));
        }
    }

    if let Some(src_id) = req.remix_of_post_id {
        let row: Option<(i64, bool, bool)> = sqlx::query_as(
            "SELECT user_id, is_reel, allow_remix FROM posts WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(src_id)
        .fetch_optional(&state.db)
        .await?;
        let Some((owner_id, is_reel, src_allow)) = row else {
            return Err(ApiError::BadRequest("Invalid remix_of_post_id".into()));
        };
        if !is_reel {
            return Err(ApiError::BadRequest("remix_of_post_id must be a reel".into()));
        }
        if !src_allow && owner_id != auth.user_id {
            return Err(ApiError::Forbidden(
                "This reel does not allow remixes".into(),
            ));
        }
    }

    let need_approval: bool = sqlx::query_scalar(
        "SELECT COALESCE((SELECT value::boolean FROM site_config WHERE category = 'features' AND key = 'post_approval' LIMIT 1), false)",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let is_approved = !need_approval;

    let template = req
        .template_key
        .as_deref()
        .map(|s| s.chars().take(64).collect::<String>());
    let row = sqlx::query_as::<_, (i64,)>(
        r#"INSERT INTO posts (user_id, content, media, is_reel, post_type, privacy, is_approved, comments_status, audio_track_id, remix_of_post_id, template_key, allow_remix)
           VALUES ($1, $2, $3, TRUE, 'reel', $4, $5, $6, $7, $8, $9, $10)
           RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(&req.content)
    .bind(&req.media)
    .bind(privacy)
    .bind(is_approved)
    .bind(comments_st)
    .bind(req.audio_track_id)
    .bind(req.remix_of_post_id)
    .bind(&template)
    .bind(allow_remix)
    .fetch_one(&state.db)
    .await?;

    if let Some(tid) = req.audio_track_id {
        let _ = sqlx::query(
            "UPDATE reel_audio_tracks SET use_count = use_count + 1 WHERE id = $1",
        )
        .bind(tid)
        .execute(&state.db)
        .await;
    }

    let cap = req.content.as_deref().unwrap_or("");
    if !cap.is_empty() {
        let _ = link_hashtags_from_caption(&state, row.0, cap).await;
        let _ = link_mentions_from_caption(&state, row.0, auth.user_id, cap).await;
    }

    if need_approval {
        let _ = sqlx::query(
            r#"
            INSERT INTO moderation_queue (target_type, target_id, submitted_by_user_id, status)
            VALUES ('post', $1, $2, 'pending')
            ON CONFLICT (target_type, target_id) DO UPDATE SET
              submitted_by_user_id = EXCLUDED.submitted_by_user_id,
              status = 'pending'
            "#,
        )
        .bind(row.0)
        .bind(auth.user_id)
        .execute(&state.db)
        .await;
    }

    let _ = state
        .event_bus
        .publish(&DomainEvent::PostCreated {
            post_id: row.0,
            user_id: auth.user_id,
            group_id: None,
            page_id: None,
        })
        .await;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": { "id": row.0 } })),
    ))
}

/// GET /v1/reels/{id}
#[tracing::instrument(skip(state, auth), err)]
pub async fn get_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let row = sqlx::query_as::<_, ReelRow>(
        r#"SELECT p.id, p.user_id, p.content, p.media,
                  p.like_count, p.comment_count, p.share_count, p.view_count, p.comments_status,
                  u.uuid, u.username, u.first_name, u.last_name, u.avatar,
                  u.is_verified, u.is_online, u.is_pro,
                  r.reaction_type AS my_reaction,
                  EXISTS(SELECT 1 FROM follows f WHERE f.follower_id = $2 AND f.following_id = p.user_id AND f.status = 'active' AND p.user_id <> $2) AS is_following,
                  EXISTS(SELECT 1 FROM saved_posts sp WHERE sp.user_id = $2 AND sp.post_id = p.id) AS is_saved,
                  p.audio_track_id, at.title AS audio_track_title, at.artist_label AS audio_track_artist, at.source AS audio_track_source, p.remix_of_post_id, p.allow_remix,
                  p.created_at
           FROM posts p
           JOIN users u ON u.id = p.user_id
           LEFT JOIN reel_audio_tracks at ON at.id = p.audio_track_id
           LEFT JOIN reactions r
                  ON r.target_type = 'post'
                 AND r.target_id = p.id
                 AND r.user_id = $2
           WHERE p.id = $1 AND p.is_reel = TRUE AND p.deleted_at IS NULL
             AND (
               p.privacy = 'everyone'
               OR (p.privacy = 'followers' AND EXISTS (
                    SELECT 1 FROM follows f
                    WHERE f.following_id = p.user_id AND f.follower_id = $2 AND f.status = 'active'))
               OR (p.privacy = 'only_me' AND p.user_id = $2)
             )"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Reel not found".into()))?;

    Ok(Json(json!({ "data": reel_to_json(row) })))
}

/// DELETE /v1/reels/{id}
#[tracing::instrument(skip(state, auth), err)]
pub async fn delete_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let result = sqlx::query(
        "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND user_id = $2 AND is_reel = TRUE AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Reel not found or not owned".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct ReelReactRequest {
    pub reaction: String,
}

/// POST /v1/reels/{id}/react
pub async fn react_to_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReelReactRequest>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;

    if req.reaction.trim().is_empty() {
        sqlx::query(
            "DELETE FROM reactions WHERE user_id = $1 AND target_type = 'post' AND target_id = $2",
        )
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(
            r#"INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
               VALUES ($1, 'post', $2, $3)
               ON CONFLICT (user_id, target_type, target_id) DO UPDATE SET reaction_type = $3"#,
        )
        .bind(auth.user_id)
        .bind(id)
        .bind(&req.reaction)
        .execute(&state.db)
        .await?;
    }

    sqlx::query("UPDATE posts SET like_count = (SELECT COUNT(*)::int FROM reactions WHERE target_type = 'post' AND target_id = $1) WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if !req.reaction.trim().is_empty() {
        let author_id: i64 = sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
        if author_id != auth.user_id {
            let _ = state
                .event_bus
                .publish(&DomainEvent::PostLiked {
                    post_id: id,
                    user_id: auth.user_id,
                    author_id,
                    reaction_type: req.reaction.trim().to_string(),
                })
                .await;
        }
    }

    let out_reaction: Option<String> = if req.reaction.trim().is_empty() {
        None
    } else {
        Some(req.reaction)
    };
    Ok(Json(json!({ "data": { "reaction": out_reaction } })))
}

/// GET /v1/reels/{id}/comments — same shape as /v1/posts/{id}/comments
#[tracing::instrument(skip(state, auth, q), err)]
pub async fn reel_comments(
    state: State<AppState>,
    auth: OptionalAuth,
    Path(id): Path<i64>,
    q: Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let is: Option<bool> = sqlx::query_scalar(
        "SELECT is_reel FROM posts WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    if is != Some(true) {
        return Err(ApiError::NotFound("Reel not found".into()));
    }
    comments::get_comments(state, auth, Path(id), q).await
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReelCommentRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: String,
    pub parent_id: Option<i64>,
}

/// POST /v1/reels/{id}/comments
#[tracing::instrument(skip(state, auth, req), err)]
pub async fn add_reel_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReelCommentRequest>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    req.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let is: Option<bool> = sqlx::query_scalar(
        "SELECT is_reel FROM posts WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    if is != Some(true) {
        return Err(ApiError::NotFound("Reel not found".into()));
    }

    let comments_open: i16 =
        sqlx::query_scalar("SELECT COALESCE(comments_status, 0) FROM posts WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    if comments_open != 0 {
        return Err(ApiError::Forbidden("Comments are disabled for this reel".into()));
    }

    let creq = comments::CreateCommentRequest {
        content: Some(req.content),
        media: None,
        parent_id: req.parent_id,
    };

    comments::create_comment(State(state), auth, Path(id), Json(creq)).await
}

/// DELETE /v1/reels/{reel_id}/comments/{comment_id}
#[tracing::instrument(skip(state, auth), err)]
pub async fn delete_reel_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((reel_id, comment_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM comments c JOIN posts p ON c.post_id = p.id
         WHERE c.id = $1 AND c.post_id = $2 AND p.is_reel = TRUE)",
    )
    .bind(comment_id)
    .bind(reel_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);
    if !ok {
        return Err(ApiError::NotFound("Comment not found".into()));
    }
    comments::delete_comment(State(state), auth, Path(comment_id)).await
}

/// POST /v1/reels/{id}/share
#[tracing::instrument(skip(state, auth), err)]
pub async fn share_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let n = sqlx::query("UPDATE posts SET share_count = share_count + 1 WHERE id = $1 AND is_reel = TRUE AND deleted_at IS NULL")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound("Reel not found".into()));
    }
    Ok(Json(json!({ "data": { "shared": true, "sharer_id": auth.user_id } })))
}

/// GET /v1/reels/trending
#[tracing::instrument(skip(state, auth, q), err)]
pub async fn reels_trending(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ReelsFeedQuery>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let limit = q.limit.unwrap_or(50).clamp(1, 50);
    let rows = sqlx::query_as::<_, ReelRow>(
        r#"
        SELECT p.id, p.user_id, p.content, p.media,
               p.like_count, p.comment_count, p.share_count, p.view_count, p.comments_status,
               u.uuid, u.username, u.first_name, u.last_name, u.avatar,
               u.is_verified, u.is_online, u.is_pro,
               r.reaction_type AS my_reaction,
               EXISTS(SELECT 1 FROM follows f WHERE f.follower_id = $1 AND f.following_id = p.user_id AND f.status = 'active' AND p.user_id <> $1) AS is_following,
               EXISTS(SELECT 1 FROM saved_posts sp WHERE sp.user_id = $1 AND sp.post_id = p.id) AS is_saved,
               p.audio_track_id, at.title AS audio_track_title, at.artist_label AS audio_track_artist, at.source AS audio_track_source, p.remix_of_post_id, p.allow_remix,
               p.created_at
        FROM posts p
        JOIN users u ON u.id = p.user_id
        LEFT JOIN reel_audio_tracks at ON at.id = p.audio_track_id
        LEFT JOIN reactions r
               ON r.target_type = 'post' AND r.target_id = p.id AND r.user_id = $1
        WHERE p.is_reel = TRUE
          AND p.deleted_at IS NULL
          AND p.is_approved = TRUE
          AND (
            p.privacy = 'everyone'
            OR (p.privacy = 'followers' AND EXISTS (
                 SELECT 1 FROM follows f
                 WHERE f.following_id = p.user_id AND f.follower_id = $1 AND f.status = 'active'))
            OR (p.privacy = 'only_me' AND p.user_id = $1)
          )
          AND p.created_at > NOW() - INTERVAL '24 hours'
        ORDER BY
          ((p.like_count + p.comment_count * 2 + p.share_count * 2)::float + p.view_count::float / 10.0)
          / GREATEST(EXTRACT(EPOCH FROM (NOW() - p.created_at)) / 3600.0, 0.25) DESC
        LIMIT $2
        "#,
    )
    .bind(auth.user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows.into_iter().map(reel_to_json).collect();
    Ok(Json(json!({ "data": data, "meta": { "has_more": false, "cursor": null } })))
}

/// GET /v1/reels/user/{username} — public reels for profile grid
#[tracing::instrument(skip(state, auth, q), err)]
pub async fn reels_by_username(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(username): Path<String>,
    Query(q): Query<ReelsFeedQuery>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let rows = sqlx::query_as::<_, ReelRow>(
        r#"
        SELECT p.id, p.user_id, p.content, p.media,
               p.like_count, p.comment_count, p.share_count, p.view_count, p.comments_status,
               u.uuid, u.username, u.first_name, u.last_name, u.avatar,
               u.is_verified, u.is_online, u.is_pro,
               r.reaction_type AS my_reaction,
               EXISTS(SELECT 1 FROM follows f WHERE f.follower_id = $1 AND f.following_id = p.user_id AND f.status = 'active' AND p.user_id <> $1) AS is_following,
               EXISTS(SELECT 1 FROM saved_posts sp WHERE sp.user_id = $1 AND sp.post_id = p.id) AS is_saved,
               p.audio_track_id, at.title AS audio_track_title, at.artist_label AS audio_track_artist, at.source AS audio_track_source, p.remix_of_post_id, p.allow_remix,
               p.created_at
        FROM posts p
        JOIN users u ON u.id = p.user_id
        LEFT JOIN reel_audio_tracks at ON at.id = p.audio_track_id
        LEFT JOIN reactions r
               ON r.target_type = 'post' AND r.target_id = p.id AND r.user_id = $1
        WHERE p.is_reel = TRUE
          AND u.username = $2
          AND p.deleted_at IS NULL
          AND p.is_approved = TRUE
          AND (
            p.privacy = 'everyone'
            OR (p.privacy = 'followers' AND EXISTS (
                 SELECT 1 FROM follows f
                 WHERE f.following_id = p.user_id AND f.follower_id = $1 AND f.status = 'active'))
            OR (p.privacy = 'only_me' AND p.user_id = $1)
          )
          AND ($3::bigint IS NULL OR p.id < $3)
        ORDER BY p.created_at DESC
        LIMIT $4
        "#,
    )
    .bind(auth.user_id)
    .bind(&username)
    .bind(q.cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let data: Vec<Value> = rows
        .into_iter()
        .take(limit as usize)
        .map(reel_to_json)
        .collect();
    let next_cursor = data.last().and_then(|d| d["id"].as_i64());
    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[derive(Debug, Deserialize)]
pub struct ReelAudioSearchQuery {
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReelAudioTrackRequest {
    pub title: String,
    pub artist_label: Option<String>,
    pub source: String,
    pub uploaded_media_id: Option<i64>,
    pub source_post_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ReelInsightRequest {
    pub bucket_sec: i16,
}

/// GET /v1/reels/explore — newest reels the viewer can see (discovery rail).
#[tracing::instrument(skip(state, auth, q), err)]
pub async fn reels_explore(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ReelsFeedQuery>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let limit = q.limit.unwrap_or(30).clamp(1, 50);
    let base_where = r#"
        p.is_reel = TRUE
          AND p.deleted_at IS NULL
          AND p.is_approved = TRUE
          AND (
            p.privacy = 'everyone'
            OR (p.privacy = 'followers' AND EXISTS (
                 SELECT 1 FROM follows f
                 WHERE f.following_id = p.user_id AND f.follower_id = $1 AND f.status = 'active'))
            OR (p.privacy = 'only_me' AND p.user_id = $1)
          )
          AND p.user_id NOT IN (
              SELECT blocked_id FROM blocks WHERE blocker_id = $1
              UNION
              SELECT blocker_id FROM blocks WHERE blocked_id = $1
          )
          AND ($2::bigint IS NULL OR p.id < $2)"#;
    let sql = format!(
        r#"
        SELECT p.id, p.user_id, p.content, p.media,
               p.like_count, p.comment_count, p.share_count, p.view_count, p.comments_status,
               u.uuid, u.username, u.first_name, u.last_name, u.avatar,
               u.is_verified, u.is_online, u.is_pro,
               r.reaction_type AS my_reaction,
               EXISTS(SELECT 1 FROM follows f WHERE f.follower_id = $1 AND f.following_id = p.user_id AND f.status = 'active' AND p.user_id <> $1) AS is_following,
               EXISTS(SELECT 1 FROM saved_posts sp WHERE sp.user_id = $1 AND sp.post_id = p.id) AS is_saved,
               p.audio_track_id, at.title AS audio_track_title, at.artist_label AS audio_track_artist, at.source AS audio_track_source, p.remix_of_post_id, p.allow_remix,
               p.created_at
        FROM posts p
        JOIN users u ON u.id = p.user_id
        LEFT JOIN reel_audio_tracks at ON at.id = p.audio_track_id
        LEFT JOIN reactions r
               ON r.target_type = 'post' AND r.target_id = p.id AND r.user_id = $1
        WHERE {base_where}
        ORDER BY p.created_at DESC
        LIMIT $3
        "#,
        base_where = base_where
    );
    let rows = sqlx::query_as::<_, ReelRow>(&sql)
        .bind(auth.user_id)
        .bind(q.cursor)
        .bind(limit + 1)
        .fetch_all(&state.db)
        .await?;
    let has_more = rows.len() as i64 > limit;
    let data: Vec<Value> = rows
        .into_iter()
        .take(limit as usize)
        .map(reel_to_json)
        .collect();
    let next_cursor = data.last().and_then(|d| d["id"].as_i64());
    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// GET /v1/reels/audio/trending
#[tracing::instrument(skip(state, _auth), err)]
pub async fn reels_audio_trending(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let rows: Vec<(i64, String, String, i64, String)> = sqlx::query_as(
        r#"SELECT id, title, artist_label, use_count, source
           FROM reel_audio_tracks
           ORDER BY use_count DESC, created_at DESC
           LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, artist_label, use_count, source)| {
            json!({
                "id": id,
                "title": title,
                "artist_label": artist_label,
                "use_count": use_count,
                "source": source,
            })
        })
        .collect();
    Ok(Json(json!({ "data": data })))
}

/// GET /v1/reels/audio/search?q=
#[tracing::instrument(skip(state, _auth, q), err)]
pub async fn reels_audio_search(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<ReelAudioSearchQuery>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    let needle = q.q.as_deref().unwrap_or("").trim();
    if needle.is_empty() {
        return Ok(Json(json!({ "data": [] })));
    }
    let pat = format!("%{}%", needle.replace('%', "\\%").replace('_', "\\_"));
    let rows: Vec<(i64, String, String, i64, String)> = sqlx::query_as(
        r#"SELECT id, title, artist_label, use_count, source
           FROM reel_audio_tracks
           WHERE title ILIKE $1 OR artist_label ILIKE $1
           ORDER BY use_count DESC, created_at DESC
           LIMIT 30"#,
    )
    .bind(&pat)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, artist_label, use_count, source)| {
            json!({
                "id": id,
                "title": title,
                "artist_label": artist_label,
                "use_count": use_count,
                "source": source,
            })
        })
        .collect();
    Ok(Json(json!({ "data": data })))
}

/// POST /v1/reels/audio — register a reusable audio track.
#[tracing::instrument(skip(state, auth, req), err)]
pub async fn create_reel_audio_track(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateReelAudioTrackRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    require_reels_feature(&state).await?;
    let src = req.source.trim().to_lowercase();
    if !["user_upload", "from_reel", "catalog"].contains(&src.as_str()) {
        return Err(ApiError::BadRequest(
            "source must be user_upload, from_reel, or catalog".into(),
        ));
    }
    if src == "from_reel" && req.source_post_id.is_none() {
        return Err(ApiError::BadRequest(
            "source_post_id required when source is from_reel".into(),
        ));
    }
    if let Some(pid) = req.source_post_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1 AND is_reel = TRUE AND deleted_at IS NULL)",
        )
        .bind(pid)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);
        if !ok {
            return Err(ApiError::BadRequest("Invalid source_post_id".into()));
        }
    }
    if let Some(mid) = req.uploaded_media_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM uploaded_media WHERE id = $1 AND user_id = $2)",
        )
        .bind(mid)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);
        if !ok {
            return Err(ApiError::BadRequest("Invalid uploaded_media_id".into()));
        }
    }
    let title: String = req.title.chars().take(300).collect();
    let artist: String = req
        .artist_label
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(300)
        .collect();
    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO reel_audio_tracks (title, artist_label, source, uploaded_media_id, source_post_id, created_by_user_id)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id"#,
    )
    .bind(&title)
    .bind(&artist)
    .bind(&src)
    .bind(req.uploaded_media_id)
    .bind(req.source_post_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": { "id": id } })),
    ))
}

/// POST /v1/reels/{id}/insights — sample retention bucket (aggregated offline).
#[tracing::instrument(skip(state, auth, req), err)]
pub async fn reel_insight_sample(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReelInsightRequest>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;
    if req.bucket_sec < 0 || req.bucket_sec > 119 {
        return Err(ApiError::BadRequest("bucket_sec must be 0..=119".into()));
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1 AND is_reel = TRUE AND deleted_at IS NULL)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if !exists {
        return Err(ApiError::NotFound("Reel not found".into()));
    }
    let mut redis = state.redis.clone();
    let key = format!("reel_insight:{}:{}", auth.user_id, id);
    let n: u64 = redis
        .incr(&key, 1)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if n == 1u64 {
        let _: () = redis.expire(&key, 60i64).await.unwrap_or(());
    }
    if n > 60 {
        return Err(ApiError::RateLimited);
    }
    sqlx::query(
        "INSERT INTO reel_insight_samples (user_id, post_id, bucket_sec) VALUES ($1, $2, $3)",
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(req.bucket_sec)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(json!({ "data": { "recorded": true } })))
}

// ── Reel Bonuses (Creator Monetization) ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BonusHistoryQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

/// GET /v1/reels/bonuses — current user's reel earnings summary
#[tracing::instrument(skip(state, auth), err)]
pub async fn get_reel_bonuses(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;

    let current: Option<(f64, f64)> = sqlx::query_as(
        "SELECT COALESCE(SUM(earnings_amount), 0) as pending,
                COALESCE(SUM(earnings_amount) FILTER (WHERE status = 'paid'), 0) as paid
         FROM reel_earnings WHERE user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .map(|(p, d): (f64, f64)| (p, d));

    let (total_earnings, paid_earnings) = current.unwrap_or((0.0, 0.0));
    let pending_earnings = total_earnings - paid_earnings;

    let total_views: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(views_count), 0) FROM reel_earnings WHERE user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "total_earnings": total_earnings,
            "pending_earnings": pending_earnings,
            "paid_earnings": paid_earnings,
            "total_views": total_views,
            "currency": "USD"
        }
    })))
}

/// GET /v1/reels/bonuses/history — paginated bonus payment history
#[tracing::instrument(skip(state, auth, q), err)]
pub async fn get_bonus_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<BonusHistoryQuery>,
) -> Result<Json<Value>, ApiError> {
    require_reels_feature(&state).await?;

    let limit = q.limit.unwrap_or(20).min(50);
    let cursor = q.cursor.and_then(|c| c.parse::<i64>().ok());

    let rows = sqlx::query_as::<_, (i64, f64, i64, String, String, String)>(
        r#"SELECT re.id, re.earnings_amount, re.views_count,
                  re.status, re.created_at::text, COALESCE(re.paid_at::text, '')
           FROM reel_earnings re
           WHERE re.user_id = $1
           AND ($2::bigint IS NULL OR re.id < $2)
           ORDER BY re.created_at DESC
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
        .map(|(id, amount, views, status, created, paid)| {
            json!({
                "id": id, "amount": amount, "views": views,
                "status": status, "created_at": created,
                "paid_at": if paid.is_empty() { None } else { Some(paid) }
            })
        })
        .collect();

    let next_cursor = if has_more {
        items.last().and_then(|i| i["id"].as_i64()).map(|id| id.to_string())
    } else {
        None
    };

    Ok(Json(json!({
        "data": items,
        "meta": { "has_more": has_more, "cursor": next_cursor }
    })))
}

#[cfg(test)]
mod mention_tests {
    use super::mention_user_keys_from_caption;

    #[test]
    fn mention_keys_dedup_and_lower() {
        let t = "yo @Foo and @foo also @bar_nice!";
        let k = mention_user_keys_from_caption(t);
        assert_eq!(k, vec!["foo".to_string(), "bar_nice".to_string()]);
    }

    #[test]
    fn mention_strips_punctuation() {
        let t = "Hi @Zed!";
        assert_eq!(mention_user_keys_from_caption(t), vec!["zed".to_string()]);
    }
}
