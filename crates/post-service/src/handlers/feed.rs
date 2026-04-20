use axum::{
    extract::{Query, State},
    Json,
};
use redis::AsyncCommands;
use serde::Deserialize;
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    models::user::PublicUserRow,
    pagination::PaginationParams,
};
use std::collections::HashMap;

use super::posts::PostRow;

#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    pub filter: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// GET /v1/feed — fan-out on read with Redis cache (60s TTL)
pub async fn get_feed(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<FeedQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.pagination.limit();
    let cursor_id = params.pagination.cursor_id().unwrap_or(i64::MAX);
    let filter = params.filter.as_deref().unwrap_or("all");

    // Try Redis cache for post IDs
    let cache_key = format!("feed:{}:{}:{}", auth.user_id, cursor_id, filter);
    let mut redis = state.redis.clone();
    let cached: Option<String> = redis.get(&cache_key).await.ok().flatten();

    let posts = if let Some(ids_json) = cached {
        // Cache hit — batch load by IDs
        let ids: Vec<i64> = serde_json::from_str(&ids_json).unwrap_or_default();
        if ids.is_empty() {
            vec![]
        } else {
            batch_load_posts(&state.db, &ids).await?
        }
    } else {
        // Cache miss — query DB, cache the IDs
        // Map filter to post_type condition
        let type_filter = match filter {
            "photos" => Some("photo"),
            "videos" => Some("video"),
            "links" => Some("link"),
            "polls" => Some("poll"),
            "live" => Some("live"),
            _ => None,
        };

        let rows = sqlx::query_as::<_, PostRow>(
            r#"SELECT p.id, p.uuid, p.user_id, p.parent_id, p.content, p.post_type, p.media,
                      p.privacy, p.feeling, p.location, p.is_pinned, p.is_boosted, p.is_reel,
                      p.like_count, p.comment_count, p.share_count, p.view_count,
                      p.created_at, p.updated_at
               FROM posts p
               WHERE p.deleted_at IS NULL
                 AND p.is_approved = TRUE
                 AND p.is_reel = FALSE
                 AND p.id < $2
                 AND (p.privacy != 'only_me' OR p.user_id = $1)
                 AND p.user_id NOT IN (SELECT blocked_id FROM blocks WHERE blocker_id = $1)
                 AND p.id NOT IN (SELECT post_id FROM hidden_posts WHERE user_id = $1)
                 AND ($4::text IS NULL OR p.post_type = $4)
                 AND (
                     p.user_id = $1
                     OR p.user_id IN (SELECT following_id FROM follows WHERE follower_id = $1 AND status = 'active')
                     OR p.page_id IN (SELECT page_id FROM page_likes WHERE user_id = $1)
                     OR p.group_id IN (SELECT group_id FROM group_members WHERE user_id = $1 AND status = 'active')
                 )
               ORDER BY p.is_pinned DESC, p.is_boosted DESC,
                        (p.like_count + p.comment_count * 3 + p.share_count * 5)::float
                        / GREATEST(EXTRACT(EPOCH FROM (NOW() - p.created_at)) / 3600, 1) DESC,
                        p.id DESC
               LIMIT $3"#,
        )
        .bind(auth.user_id)
        .bind(cursor_id)
        .bind(limit + 1)
        .bind(type_filter)
        .fetch_all(&state.db)
        .await?;

        // Cache post IDs for 60 seconds
        let ids: Vec<i64> = rows.iter().map(|p| p.id).collect();
        let _: Result<(), _> = redis
            .set_ex(&cache_key, serde_json::to_string(&ids).unwrap_or_default(), 60)
            .await;

        rows
    };

    let has_more = posts.len() as i64 > limit;
    let data: Vec<_> = posts.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    // Batch load publisher info for all posts
    let user_ids: Vec<i64> = data.iter().map(|p| p.user_id).collect::<std::collections::HashSet<_>>().into_iter().collect();
    let publishers: HashMap<i64, PublicUserRow> = if !user_ids.is_empty() {
        let rows = sqlx::query_as::<_, PublicUserRow>(
            r#"SELECT uuid, username, first_name, last_name, avatar, cover, about, is_verified, is_pro
               FROM users WHERE id = ANY($1)"#,
        )
        .bind(&user_ids)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        // Re-query with id to build the map
        let id_rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, username FROM users WHERE id = ANY($1)",
        )
        .bind(&user_ids)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        let username_map: HashMap<String, PublicUserRow> = rows.into_iter().map(|r| (r.username.clone(), r)).collect();
        id_rows.into_iter().filter_map(|(id, uname)| {
            username_map.get(&uname).cloned().map(|p| (id, p))
        }).collect()
    } else {
        HashMap::new()
    };

    // Inject publisher + feed ad every 5 posts
    let mut result: Vec<serde_json::Value> = Vec::with_capacity(data.len() + 4);
    for (i, post) in data.iter().enumerate() {
        let mut val = serde_json::to_value(post).unwrap_or_default();
        if let Some(obj) = val.as_object_mut()
            && let Some(pub_row) = publishers.get(&post.user_id)
        {
            obj.insert(
                "publisher".into(),
                serde_json::to_value(pub_row).unwrap_or_default(),
            );
        }
        result.push(val);
        if (i + 1) % 5 == 0
            && let Ok(Some(ad)) = get_feed_ad(&state.db, auth.user_id).await {
                result.push(ad);
            }
    }

    Ok(Json(json!({
        "data": result,
        "meta": {
            "cursor": next_cursor,
            "has_more": has_more,
        }
    })))
}

/// Batch load posts by IDs (avoids N+1)
async fn batch_load_posts(db: &sqlx::PgPool, ids: &[i64]) -> Result<Vec<PostRow>, ApiError> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let posts = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, uuid, user_id, parent_id, content, post_type, media,
                  privacy, feeling, location, is_pinned, is_boosted, is_reel,
                  like_count, comment_count, share_count, view_count,
                  created_at, updated_at
           FROM posts WHERE id = ANY($1) AND deleted_at IS NULL
           ORDER BY is_pinned DESC, is_boosted DESC, id DESC"#,
    )
    .bind(ids)
    .fetch_all(db)
    .await?;

    Ok(posts)
}

/// Select a random active ad with remaining budget and track impression
async fn get_feed_ad(
    db: &sqlx::PgPool,
    _viewer_id: i64,
) -> Result<Option<serde_json::Value>, ApiError> {
    let ad = sqlx::query_as::<_, (i64, i64, String)>(
        r#"SELECT ua.id, ua.post_id, ua.audience
        FROM user_ads ua
        WHERE ua.status = 'active' AND ua.budget > 0
        ORDER BY RANDOM() LIMIT 1"#,
    )
    .fetch_optional(db)
    .await?;

    if let Some((ad_id, post_id, _audience)) = ad {
        // Increment impressions, deduct from budget
        sqlx::query(
            "UPDATE user_ads SET impressions = impressions + 1, budget = GREATEST(budget - 0.001, 0) WHERE id = $1",
        )
        .bind(ad_id)
        .execute(db)
        .await?;

        let post = sqlx::query_as::<_, PostRow>(
            r#"SELECT id, uuid, user_id, parent_id, content, post_type, media,
                      privacy, feeling, location, is_pinned, is_boosted, is_reel,
                      like_count, comment_count, share_count, view_count,
                      created_at, updated_at
               FROM posts WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(post_id)
        .fetch_optional(db)
        .await?;

        if let Some(p) = post {
            let mut val = serde_json::to_value(&p).unwrap_or_default();
            if let Some(obj) = val.as_object_mut() {
                obj.insert("is_ad".into(), json!(true));
                obj.insert("ad_id".into(), json!(ad_id));
            }
            return Ok(Some(val));
        }
    }
    Ok(None)
}
