//! Post XHR-equivalent endpoints ported from Script/xhr/posts.php.
//!
//! Routes:
//! - POST   /v1/posts/preview-url                 — Open Graph + oEmbed URL preview
//! - DELETE /v1/posts/{id}/media/{media_id}       — remove one item from multi-image post
//! - PUT    /v1/posts/{id}/comments-status        — toggle comments enabled/disabled
//! - POST   /v1/posts/{id}/mark-sold              — mark a product-type post as sold
//! - POST   /v1/posts/{id}/notify-followers       — notify followers about a post (rate-limited)
//! - POST   /v1/posts/{id}/video-view             — increment video_views (deduped)
//! - POST   /v1/posts/{id}/wonder                 — toggle "wonder" reaction (separate from like)
//! - GET    /v1/posts/{id}/reactors?type=like|wonder|share
//!
//! Also:
//! - POST   /v1/posts/audio                       — upload an audio blob as a post

use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

// ═══════════════════════════════════════════════════════════════════
// URL preview (Open Graph + oEmbed for YouTube/Vimeo)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct PreviewUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct UrlPreview {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub site_name: Option<String>,
    pub embed_html: Option<String>,
}


fn is_private_ip(addr: &std::net::IpAddr) -> bool {
    match addr {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xff00 == 0xfc00)
                || (v6.segments()[0] & 0xffc0 == 0xfe80)
        }
    }
}

fn extract_host<'a>(url: &'a str) -> Option<&'a str> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    Some(rest.split(&['/', '?', '#', ':'][..]).next().unwrap_or(""))
}

async fn is_internal_host(url_str: &str) -> bool {
    let host = match extract_host(url_str) {
        Some(h) if !h.is_empty() => h,
        _ => return true,
    };

    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return is_private_ip(&ip);
    }

    let port = if url_str.starts_with("https://") { 443 } else { 80 };
    let addr_str = format!("{}:{}", host, port);

    match tokio::net::lookup_host(&addr_str).await {
        Ok(addrs) => {
            for addr in addrs {
                if is_private_ip(&addr.ip()) {
                    return true;
                }
            }
            false
        }
        Err(_) => false,
    }
}

pub async fn preview_url(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<PreviewUrlRequest>,
) -> Result<Json<Value>, ApiError> {
    let url = req.url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(ApiError::BadRequest(
            "url must start with http(s)://".into(),
        ));
    }

    // SSRF protection: block internal/private IP ranges
    if is_internal_host(url).await {
        return Err(ApiError::BadRequest(
            "url resolves to a private or internal address".into(),
        ));
    }

    let url_hash = {
        let mut h = Sha256::new();
        h.update(url.as_bytes());
        format!("{:x}", h.finalize())
    };

    // Cache lookup (24h TTL)
    type UrlPreviewRow = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let cached: Option<UrlPreviewRow> = sqlx::query_as(
        r#"SELECT url, title, description, image_url, site_name, embed_html
             FROM url_preview_cache
            WHERE url_hash = $1 AND fetched_at > NOW() - INTERVAL '24 hours'"#,
    )
    .bind(&url_hash)
    .fetch_optional(&state.db)
    .await?;

    if let Some((u, t, d, i, s, e)) = cached {
        return Ok(Json(json!({
            "data": UrlPreview { url: u, title: t, description: d, image_url: i, site_name: s, embed_html: e }
        })));
    }

    // Fetch with a timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("JungleBot/1.0 (+https://jungle.example)")
        .redirect(reqwest::redirect::Policy::limited(4))
        .build()
        .map_err(|e| ApiError::Internal(format!("http builder: {}", e)))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| ApiError::BadRequest(format!("fetch: {}", e)))?;
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if !status.is_success() || !content_type.contains("text/html") {
        return Err(ApiError::BadRequest(format!(
            "upstream returned {} ({})",
            status, content_type
        )));
    }

    let body = resp.text().await.unwrap_or_default();
    let mut preview = UrlPreview {
        url: url.to_string(),
        title: extract_meta(&body, &["og:title", "twitter:title"]).or_else(|| extract_title(&body)),
        description: extract_meta(
            &body,
            &["og:description", "description", "twitter:description"],
        ),
        image_url: extract_meta(&body, &["og:image", "twitter:image"]),
        site_name: extract_meta(&body, &["og:site_name"]),
        embed_html: None,
    };

    // YouTube / Vimeo oEmbed
    if is_youtube(url) {
        preview.embed_html = Some(youtube_embed(url));
    } else if is_vimeo(url) {
        preview.embed_html = Some(vimeo_embed(url));
    }

    // Upsert into cache
    let _ = sqlx::query(
        r#"INSERT INTO url_preview_cache
            (url_hash, url, title, description, image_url, site_name, embed_html)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (url_hash) DO UPDATE SET
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                image_url = EXCLUDED.image_url,
                site_name = EXCLUDED.site_name,
                embed_html = EXCLUDED.embed_html,
                fetched_at = NOW()"#,
    )
    .bind(&url_hash)
    .bind(&preview.url)
    .bind(&preview.title)
    .bind(&preview.description)
    .bind(&preview.image_url)
    .bind(&preview.site_name)
    .bind(&preview.embed_html)
    .execute(&state.db)
    .await;

    Ok(Json(json!({ "data": preview })))
}

fn extract_meta(html: &str, names: &[&str]) -> Option<String> {
    let lower = html.to_lowercase();
    for name in names {
        let needle = format!(r#"property="{}""#, name);
        let needle2 = format!(r#"name="{}""#, name);
        for n in [&needle, &needle2] {
            if let Some(idx) = lower.find(n.as_str()) {
                // Look for content=".." within the same tag (< .. >)
                let start = html[..idx].rfind('<').unwrap_or(0);
                let end = html[idx..].find('>').map(|e| idx + e).unwrap_or(html.len());
                let tag = &html[start..end];
                if let Some(content) = extract_content_attr(tag) {
                    return Some(content);
                }
            }
        }
    }
    None
}

fn extract_content_attr(tag: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let key = "content=";
    let idx = lower.find(key)?;
    let rest = &tag[idx + key.len()..];
    let bytes = rest.as_bytes();
    if bytes.first() == Some(&b'"') {
        let end = rest[1..].find('"')?;
        Some(rest[1..=end].to_string())
    } else if bytes.first() == Some(&b'\'') {
        let end = rest[1..].find('\'')?;
        Some(rest[1..=end].to_string())
    } else {
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(rest.len());
        Some(rest[..end].to_string())
    }
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let s = lower.find("<title>")? + 7;
    let e = lower[s..].find("</title>")? + s;
    Some(html[s..e].trim().to_string())
}

fn is_youtube(url: &str) -> bool {
    url.contains("youtube.com/watch") || url.contains("youtu.be/")
}

fn is_vimeo(url: &str) -> bool {
    url.contains("vimeo.com/")
}

fn youtube_embed(url: &str) -> String {
    // Extract v= param or the path after youtu.be/
    let id = if let Some(pos) = url.find("v=") {
        url[pos + 2..].split(&['&', '#'][..]).next().unwrap_or("")
    } else if let Some(pos) = url.find("youtu.be/") {
        url[pos + 9..]
            .split(&['?', '#', '/'][..])
            .next()
            .unwrap_or("")
    } else {
        ""
    };
    format!(
        r#"<iframe width="560" height="315" src="https://www.youtube.com/embed/{}" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>"#,
        id
    )
}

fn vimeo_embed(url: &str) -> String {
    let id = url.rsplit('/').next().unwrap_or("");
    format!(
        r#"<iframe src="https://player.vimeo.com/video/{}" width="640" height="360" frameborder="0" allow="autoplay; fullscreen" allowfullscreen></iframe>"#,
        id
    )
}

// ═══════════════════════════════════════════════════════════════════
// DELETE /v1/posts/{id}/media/{media_id} — remove one item from multi-image post
// ═══════════════════════════════════════════════════════════════════

pub async fn delete_post_media(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((post_id, media_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    // Verify ownership
    let owner: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1 AND deleted_at IS NULL")
            .bind(post_id)
            .fetch_optional(&state.db)
            .await?;

    let owner = owner.ok_or(ApiError::NotFound("post not found".into()))?;
    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("not post owner".into()));
    }

    let result = sqlx::query("DELETE FROM post_media WHERE id = $1 AND post_id = $2")
        .bind(media_id)
        .bind(post_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("media not found in post".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ═══════════════════════════════════════════════════════════════════
// PUT /v1/posts/{id}/comments-status — enable or disable comments
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CommentsStatusRequest {
    pub enabled: bool,
}

pub async fn set_comments_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(req): Json<CommentsStatusRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1 AND deleted_at IS NULL")
            .bind(post_id)
            .fetch_optional(&state.db)
            .await?;

    let owner = owner.ok_or(ApiError::NotFound("post not found".into()))?;
    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("not post owner".into()));
    }

    // 0 = enabled, 1 = disabled (preserves the PHP convention)
    let status: i16 = if req.enabled { 0 } else { 1 };
    sqlx::query("UPDATE posts SET comments_status = $1 WHERE id = $2")
        .bind(status)
        .bind(post_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "enabled": req.enabled } })))
}

// ═══════════════════════════════════════════════════════════════════
// POST /v1/posts/{id}/mark-sold
// ═══════════════════════════════════════════════════════════════════

pub async fn mark_sold(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT user_id, post_type FROM posts WHERE id = $1 AND deleted_at IS NULL")
            .bind(post_id)
            .fetch_optional(&state.db)
            .await?;

    let (owner, post_type) = row.ok_or(ApiError::NotFound("post not found".into()))?;
    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("not post owner".into()));
    }

    // Only product-type posts can be marked sold
    let is_product = matches!(
        post_type.as_deref(),
        Some("product") | Some("sale") | Some("marketplace")
    );
    if !is_product {
        return Err(ApiError::BadRequest(
            "only product posts can be marked sold".into(),
        ));
    }

    sqlx::query("UPDATE posts SET is_sold = TRUE WHERE id = $1")
        .bind(post_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "is_sold": true } })))
}

// ═══════════════════════════════════════════════════════════════════
// POST /v1/posts/{id}/notify-followers
// ═══════════════════════════════════════════════════════════════════

pub async fn notify_followers(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1 AND deleted_at IS NULL")
            .bind(post_id)
            .fetch_optional(&state.db)
            .await?;

    let owner = owner.ok_or(ApiError::NotFound("post not found".into()))?;
    if owner != auth.user_id {
        return Err(ApiError::Forbidden("not post owner".into()));
    }

    // Rate limit: only one notify per post
    let already: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(SELECT 1 FROM notifications
                          WHERE sender_id = $1
                            AND type = 'NotifyPost'
                            AND text LIKE '%post/' || $2::text || '%')"#,
    )
    .bind(auth.user_id)
    .bind(post_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if already {
        return Err(ApiError::Conflict(
            "followers already notified for this post".into(),
        ));
    }

    let affected = sqlx::query(
        r#"INSERT INTO notifications (recipient_id, sender_id, type, text)
           SELECT f.follower_id, $1, 'NotifyPost',
                  'New post from a user you follow. See post/' || $2::text
             FROM follows f
            WHERE f.following_id = $1 AND f.status = 'active'"#,
    )
    .bind(auth.user_id)
    .bind(post_id)
    .execute(&state.db)
    .await?
    .rows_affected();

    Ok(Json(json!({ "data": { "notified": affected } })))
}

// ═══════════════════════════════════════════════════════════════════
// POST /v1/posts/{id}/video-view — dedup per user+IP per day
// ═══════════════════════════════════════════════════════════════════

pub async fn video_view(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let ip_hash = {
        let mut h = Sha256::new();
        h.update(ip.as_bytes());
        format!("{:x}", h.finalize())
    };

    let inserted = sqlx::query(
        r#"INSERT INTO video_views_dedup (post_id, user_id, ip_hash)
           VALUES ($1, $2, $3)
           ON CONFLICT (post_id, user_id, ip_hash) DO NOTHING"#,
    )
    .bind(post_id)
    .bind(auth.user_id)
    .bind(&ip_hash)
    .execute(&state.db)
    .await?
    .rows_affected();

    if inserted > 0 {
        sqlx::query("UPDATE posts SET video_views = video_views + 1 WHERE id = $1")
            .bind(post_id)
            .execute(&state.db)
            .await?;
    }

    let total: i32 = sqlx::query_scalar("SELECT video_views FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(
        json!({ "data": { "counted": inserted > 0, "total_views": total } }),
    ))
}

// ═══════════════════════════════════════════════════════════════════
// POST /v1/posts/{id}/wonder — separate "wonder" reaction from like
// ═══════════════════════════════════════════════════════════════════

pub async fn toggle_wonder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let existed: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
               SELECT 1 FROM post_reactions
                WHERE post_id = $1 AND user_id = $2 AND reaction_type = 'wonder'
           )"#,
    )
    .bind(post_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if existed {
        sqlx::query(
            r#"DELETE FROM post_reactions
                WHERE post_id = $1 AND user_id = $2 AND reaction_type = 'wonder'"#,
        )
        .bind(post_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;
        sqlx::query("UPDATE posts SET wonder_count = GREATEST(wonder_count - 1, 0) WHERE id = $1")
            .bind(post_id)
            .execute(&state.db)
            .await?;
    } else {
        sqlx::query(
            r#"INSERT INTO post_reactions (post_id, user_id, reaction_type)
               VALUES ($1, $2, 'wonder')
               ON CONFLICT (post_id, user_id, reaction_type) DO NOTHING"#,
        )
        .bind(post_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;
        sqlx::query("UPDATE posts SET wonder_count = wonder_count + 1 WHERE id = $1")
            .bind(post_id)
            .execute(&state.db)
            .await?;
    }

    let total: i32 = sqlx::query_scalar("SELECT wonder_count FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(
        json!({ "data": { "wondered": !existed, "wonder_count": total } }),
    ))
}

// ═══════════════════════════════════════════════════════════════════
// GET /v1/posts/{id}/reactors?type=like|wonder|share
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ReactorsQuery {
    #[serde(rename = "type")]
    pub reaction_type: Option<String>,
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_reactors(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(post_id): Path<i64>,
    Query(q): Query<ReactorsQuery>,
) -> Result<Json<Value>, ApiError> {
    let rtype = q.reaction_type.as_deref().unwrap_or("like");
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let cursor = q.cursor;

    type ReactorRow = (
        i64,
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
    );
    let rows: Vec<ReactorRow> = if rtype == "share" {
        sqlx::query_as(
            r#"SELECT ps.id, u.id, u.username, u.first_name, u.last_name, u.avatar, ps.created_at::text
                 FROM post_shares ps
                 JOIN users u ON u.id = ps.user_id
                WHERE ps.post_id = $1
                  AND ($2::bigint IS NULL OR ps.id < $2)
             ORDER BY ps.id DESC
                LIMIT $3"#,
        )
        .bind(post_id)
        .bind(cursor)
        .bind(limit + 1)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT pr.id, u.id, u.username, u.first_name, u.last_name, u.avatar, pr.created_at::text
                 FROM post_reactions pr
                 JOIN users u ON u.id = pr.user_id
                WHERE pr.post_id = $1 AND pr.reaction_type = $2
                  AND ($3::bigint IS NULL OR pr.id < $3)
             ORDER BY pr.id DESC
                LIMIT $4"#,
        )
        .bind(post_id)
        .bind(rtype)
        .bind(cursor)
        .bind(limit + 1)
        .fetch_all(&state.db)
        .await?
    };

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.0.to_string());

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(_id, user_id, username, fn_, ln, avatar, ts)| {
            json!({
                "id": user_id,
                "username": username,
                "first_name": fn_,
                "last_name": ln,
                "avatar": avatar,
                "reaction": rtype,
                "reacted_at": ts,
            })
        })
        .collect();

    Ok(Json(json!({
        "data": data,
        "meta": { "has_more": has_more, "cursor": next_cursor }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// POST /v1/posts/audio — create an audio-type post with a blob upload
// ═══════════════════════════════════════════════════════════════════

pub async fn create_audio_post(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    const MAX_BYTES: usize = 10 * 1024 * 1024; // 10 MB

    let mut text: Option<String> = None;
    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut audio_mime: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "text" {
            text = Some(
                field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?,
            );
        } else if name == "audio" {
            audio_mime = field.content_type().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            if data.len() > MAX_BYTES {
                return Err(ApiError::BadRequest("audio > 10 MB".into()));
            }
            audio_bytes = Some(data.to_vec());
        }
    }

    let audio_bytes = audio_bytes.ok_or(ApiError::BadRequest("audio field required".into()))?;
    let mime = audio_mime.unwrap_or_else(|| "audio/webm".into());
    if !mime.starts_with("audio/") {
        return Err(ApiError::BadRequest("audio content-type required".into()));
    }

    // Persist the blob via the shared storage abstraction
    let storage = shared::storage::create_storage().await;
    let key = format!("audio/{}/{}.bin", auth.user_id, uuid::Uuid::new_v4());
    let url = storage
        .upload(&key, &audio_bytes, &mime)
        .await
        .map_err(|e| ApiError::Internal(format!("upload failed: {}", e)))?;

    // Create the post row (post_type = 'audio')
    let post_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO posts (user_id, text, post_type, media_url, published_at)
           VALUES ($1, $2, 'audio', $3, NOW())
           RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(text.as_deref().unwrap_or(""))
    .bind(&url)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "id": post_id,
            "media_url": url,
            "mime": mime,
        }
    })))
}
