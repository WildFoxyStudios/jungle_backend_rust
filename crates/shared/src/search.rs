use axum::Json;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::errors::ApiError;

/// Optional narrowing for `type=user` and for the **`users`** bucket in aggregate `"all"` search.
#[derive(Clone, Debug, Default)]
pub struct SearchUserFilters {
    gender: Option<String>,
    verified_only: bool,
    require_photo: bool,
    apply_age: bool,
    age_min: i32,
    age_max: i32,
}

impl SearchUserFilters {
    #[must_use]
    pub fn from_optional(
        gender: Option<String>,
        verified_only: Option<bool>,
        has_photo: Option<bool>,
        age_min: Option<i32>,
        age_max: Option<i32>,
    ) -> Self {
        let mut out = Self::default();
        if let Some(g) = gender {
            let g = g.trim().to_lowercase();
            if g == "male" || g == "female" {
                out.gender = Some(g);
            }
        }
        out.verified_only = matches!(verified_only, Some(true));
        out.require_photo = matches!(has_photo, Some(true));
        if let (Some(a_min), Some(a_max)) = (age_min, age_max) {
            let lo = a_min.clamp(13, 120);
            let hi = a_max.clamp(13, 120).max(lo);
            out.apply_age = true;
            out.age_min = lo;
            out.age_max = hi;
        }
        out
    }
}

async fn search_users(
    db: &PgPool,
    ilike: &str,
    prefix: &str,
    limit: i64,
    viewer_id: Option<i64>,
    filters: &SearchUserFilters,
) -> Result<Vec<SearchResult>, ApiError> {
    let rows = sqlx::query_as::<_, SearchResult>(
        r#"
        SELECT id, username AS name, avatar AS image, 'user' AS result_type,
            CASE WHEN username ILIKE $2 THEN 0 ELSE 1 END AS rank
        FROM users
        WHERE deleted_at IS NULL AND is_active = TRUE
            AND (username ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1)
            AND ($3::bigint IS NULL OR id NOT IN (
                SELECT blocked_id FROM blocks WHERE blocker_id = $3
                UNION ALL
                SELECT blocker_id FROM blocks WHERE blocked_id = $3
            ))
            AND ($4::text IS NULL OR gender = $4)
            AND (NOT $5::bool OR is_verified = TRUE)
            AND (NOT $6::bool OR (avatar IS NOT NULL AND TRIM(avatar) <> ''))
            AND (
                NOT $7::bool OR (
                    birthday IS NOT NULL
                    AND EXTRACT(YEAR FROM AGE(CURRENT_DATE::date, birthday))::integer >= $8
                    AND EXTRACT(YEAR FROM AGE(CURRENT_DATE::date, birthday))::integer <= $9
                )
            )
        ORDER BY rank, is_verified DESC
        LIMIT $10
        "#,
    )
    .bind(ilike)
    .bind(prefix)
    .bind(viewer_id)
    .bind(filters.gender.clone())
    .bind(filters.verified_only)
    .bind(filters.require_photo)
    .bind(filters.apply_age)
    .bind(filters.age_min)
    .bind(filters.age_max)
    .bind(limit.clamp(1, 50))
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Global search across all entity types
pub async fn search_all(
    db: &PgPool,
    query: &str,
    search_type: Option<&str>,
    viewer_id: Option<i64>,
    limit: i64,
    user_filters: SearchUserFilters,
) -> Result<Json<Value>, ApiError> {
    let tsquery = to_tsquery(query);
    let ilike = format!("%{}%", crate::sanitize::sanitize_search_query(query));
    let prefix = format!("{}%", crate::sanitize::sanitize_search_query(query));
    let limit = limit.clamp(1, 50);

    match search_type.unwrap_or("all") {
        "user" => {
            let users = search_users(db, &ilike, &prefix, limit, viewer_id, &user_filters).await?;

            Ok(Json(json!({ "data": users, "type": "users" })))
        }

        "post" => {
            let posts = search_posts_ts(db, &tsquery, limit).await?;

            Ok(Json(json!({ "data": posts, "type": "posts" })))
        }

        "page" => {
            let pages = sqlx::query_as::<_, SearchResult>(
                r#"
                SELECT page_id AS id, page_title AS name, avatar AS image, 'page' AS result_type, 0::real AS rank
                FROM pages WHERE active = TRUE
                    AND (page_name ILIKE $1 OR page_title ILIKE $1)
                ORDER BY like_count DESC
                LIMIT $2
                "#,
            )
            .bind(&ilike)
            .bind(limit)
            .fetch_all(db)
            .await?;

            Ok(Json(json!({ "data": pages, "type": "pages" })))
        }

        "group" => {
            let groups = sqlx::query_as::<_, SearchResult>(
                r#"
                SELECT id, group_title AS name, avatar AS image, 'group' AS result_type, 0::real AS rank
                FROM groups WHERE active = TRUE
                    AND (group_name ILIKE $1 OR group_title ILIKE $1)
                ORDER BY member_count DESC
                LIMIT $2
                "#,
            )
            .bind(&ilike)
            .bind(limit)
            .fetch_all(db)
            .await?;

            Ok(Json(json!({ "data": groups, "type": "groups" })))
        }

        "hashtag" => {
            let tags = sqlx::query_as::<_, SearchResult>(
                r#"
                SELECT id, tag AS name, NULL AS image, 'hashtag' AS result_type, 0::real AS rank
                FROM hashtags WHERE tag ILIKE $1
                ORDER BY use_count DESC
                LIMIT $2
                "#,
            )
            .bind(&ilike)
            .bind(limit)
            .fetch_all(db)
            .await?;

            Ok(Json(json!({ "data": tags, "type": "hashtags" })))
        }

        "blog" => {
            let blogs = search_blogs_ts(db, &tsquery, limit).await?;

            Ok(Json(json!({ "data": blogs, "type": "blogs" })))
        }

        "product" => {
            let products = search_products_ilike(db, &ilike, limit).await?;

            Ok(Json(json!({ "data": products, "type": "products" })))
        }

        "event" => {
            let events = search_events_ilike(db, &ilike, limit).await?;

            Ok(Json(json!({ "data": events, "type": "events" })))
        }

        "reel" => {
            let reels = search_reels_ts(db, &tsquery, limit).await?;

            Ok(Json(json!({ "data": reels, "type": "reels" })))
        }

        // "all" — search everything in parallel
        _ => {
            let (users, pages, groups, hashtags, posts, reels, blogs, products, events) =
                tokio::join!(
                    search_type_internal(SearchTypeParams {
                        db,
                        search_type: "user",
                        ilike: &ilike,
                        prefix: &prefix,
                        _tsquery: &tsquery,
                        limit: 5,
                        viewer_id,
                        user_filters: &user_filters,
                    }),
                    search_type_internal(SearchTypeParams { db, search_type: "page", ilike: &ilike, prefix: &prefix, _tsquery: &tsquery, limit: 5, viewer_id, user_filters: &user_filters }),
                    search_type_internal(SearchTypeParams { db, search_type: "group", ilike: &ilike, prefix: &prefix, _tsquery: &tsquery, limit: 5, viewer_id, user_filters: &user_filters }),
                    search_type_internal(SearchTypeParams { db, search_type: "hashtag", ilike: &ilike, prefix: &prefix, _tsquery: &tsquery, limit: 5, viewer_id, user_filters: &user_filters }),
                    search_posts_ts(db, &tsquery, 5),
                    search_reels_ts(db, &tsquery, 5),
                    search_blogs_ts(db, &tsquery, 5),
                    search_products_ilike(db, &ilike, 5),
                    search_events_ilike(db, &ilike, 5),
                );
            Ok(Json(json!({
                "data": {
                    "users": users.unwrap_or_default(),
                    "pages": pages.unwrap_or_default(),
                    "groups": groups.unwrap_or_default(),
                    "hashtags": hashtags.unwrap_or_default(),
                    "posts": posts.unwrap_or_default(),
                    "reels": reels.unwrap_or_default(),
                    "blogs": blogs.unwrap_or_default(),
                    "products": products.unwrap_or_default(),
                    "events": events.unwrap_or_default(),
                }
            })))
        }
    }
}

/// Full-text post hits (shared by `type=post` and aggregate `all`).
async fn search_posts_ts(
    db: &PgPool,
    tsquery: &str,
    limit: i64,
) -> Result<Vec<SearchResult>, ApiError> {
    if tsquery.trim().is_empty() {
        return Ok(vec![]);
    }
    let rows = sqlx::query_as::<_, SearchResult>(
        r#"
        SELECT p.id, LEFT(p.content, 200) AS name, NULL AS image, 'post' AS result_type,
            ts_rank(p.search_vector, to_tsquery('simple', $1))::real AS rank
        FROM posts p
        WHERE p.deleted_at IS NULL AND p.is_approved = TRUE AND p.privacy = 'everyone'
            AND COALESCE(p.is_reel, false) = false
            AND p.search_vector @@ to_tsquery('simple', $1)
        ORDER BY rank DESC
        LIMIT $2
        "#,
    )
    .bind(tsquery)
    .bind(limit.clamp(1, 50))
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Reel posts (`is_reel`) — FTS on `posts.search_vector`; excluded from generic `post` hits.
async fn search_reels_ts(
    db: &PgPool,
    tsquery: &str,
    limit: i64,
) -> Result<Vec<SearchResult>, ApiError> {
    if tsquery.trim().is_empty() {
        return Ok(vec![]);
    }
    let rows = sqlx::query_as::<_, SearchResult>(
        r#"
        SELECT p.id, LEFT(p.content, 200) AS name, NULL AS image, 'reel' AS result_type,
            ts_rank(p.search_vector, to_tsquery('simple', $1))::real AS rank
        FROM posts p
        WHERE p.deleted_at IS NULL AND p.is_approved = TRUE AND p.privacy = 'everyone'
            AND COALESCE(p.is_reel, false) = true
            AND p.search_vector @@ to_tsquery('simple', $1)
        ORDER BY rank DESC
        LIMIT $2
        "#,
    )
    .bind(tsquery)
    .bind(limit.clamp(1, 50))
    .fetch_all(db)
    .await?;
    Ok(rows)
}

async fn search_blogs_ts(
    db: &PgPool,
    tsquery: &str,
    limit: i64,
) -> Result<Vec<SearchResult>, ApiError> {
    if tsquery.trim().is_empty() {
        return Ok(vec![]);
    }
    let rows = sqlx::query_as::<_, SearchResult>(
        r#"
        SELECT b.id, b.title AS name, b.thumbnail AS image, 'blog' AS result_type,
            ts_rank(b.search_vector, to_tsquery('simple', $1))::real AS rank
        FROM blogs b
        WHERE b.is_approved = TRUE
            AND b.search_vector @@ to_tsquery('simple', $1)
        ORDER BY rank DESC
        LIMIT $2
        "#,
    )
    .bind(tsquery)
    .bind(limit.clamp(1, 50))
    .fetch_all(db)
    .await?;
    Ok(rows)
}

async fn search_products_ilike(
    db: &PgPool,
    ilike: &str,
    limit: i64,
) -> Result<Vec<SearchResult>, ApiError> {
    Ok(sqlx::query_as::<_, SearchResult>(
        r#"
        SELECT id, name, NULL AS image, 'product' AS result_type, 0::real AS rank
        FROM products WHERE status = 'active'
            AND (name ILIKE $1 OR description ILIKE $1)
        ORDER BY rating DESC
        LIMIT $2
        "#,
    )
    .bind(ilike)
    .bind(limit.clamp(1, 50))
    .fetch_all(db)
    .await?)
}

async fn search_events_ilike(
    db: &PgPool,
    ilike: &str,
    limit: i64,
) -> Result<Vec<SearchResult>, ApiError> {
    Ok(sqlx::query_as::<_, SearchResult>(
        r#"
        SELECT e.id, e.name AS name, e.cover AS image, 'event' AS result_type,
            CASE WHEN e.end_at >= NOW() THEN 0::real ELSE 1::real END AS rank
        FROM events e
        WHERE (
            e.name ILIKE $1
            OR e.description ILIKE $1
            OR COALESCE(e.location, '') ILIKE $1
        )
        ORDER BY rank ASC, e.start_at DESC
        LIMIT $2
        "#,
    )
    .bind(ilike)
    .bind(limit.clamp(1, 50))
    .fetch_all(db)
    .await?)
}

struct SearchTypeParams<'a> {
    db: &'a PgPool,
    search_type: &'a str,
    ilike: &'a str,
    prefix: &'a str,
    _tsquery: &'a str,
    limit: i64,
    viewer_id: Option<i64>,
    user_filters: &'a SearchUserFilters,
}

async fn search_type_internal(p: SearchTypeParams<'_>) -> Result<Vec<SearchResult>, ApiError> {
    if p.search_type == "user" {
        return search_users(p.db, p.ilike, p.prefix, p.limit, p.viewer_id, p.user_filters).await;
    }

    let query = match p.search_type {
        "page" => {
            "SELECT page_id AS id, page_title AS name, avatar AS image, 'page' AS result_type, 0::real AS rank FROM pages WHERE active = TRUE AND (page_name ILIKE $1 OR page_title ILIKE $1) ORDER BY like_count DESC LIMIT $2"
        }
        "group" => {
            "SELECT id, group_title AS name, avatar AS image, 'group' AS result_type, 0::real AS rank FROM groups WHERE active = TRUE AND (group_name ILIKE $1 OR group_title ILIKE $1) ORDER BY member_count DESC LIMIT $2"
        }
        "hashtag" => {
            "SELECT id, tag AS name, NULL AS image, 'hashtag' AS result_type, 0::real AS rank FROM hashtags WHERE tag ILIKE $1 ORDER BY use_count DESC LIMIT $2"
        }
        _ => return Ok(vec![]),
    };

    Ok(sqlx::query_as::<_, SearchResult>(query)
        .bind(p.ilike)
        .bind(p.limit)
        .fetch_all(p.db)
        .await?)
}

/// Convert user query to PostgreSQL tsquery
pub fn to_tsquery(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| {
            let sanitized = w.replace(['\'', '\\'], "");
            format!("{}:*", sanitized)
        })
        .collect::<Vec<_>>()
        .join(" & ")
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SearchResult {
    pub id: i64,
    pub name: Option<String>,
    pub image: Option<String>,
    pub result_type: String,
    pub rank: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_tsquery_single_word() {
        assert_eq!(to_tsquery("hello"), "hello:*");
    }

    #[test]
    fn test_to_tsquery_multiple_words() {
        assert_eq!(to_tsquery("hello world"), "hello:* & world:*");
    }

    #[test]
    fn test_to_tsquery_strips_special_chars() {
        let result = to_tsquery("it's a test\\");
        assert_eq!(result, "its:* & a:* & test:*");
    }

    #[test]
    fn test_to_tsquery_empty() {
        assert_eq!(to_tsquery(""), "");
    }

    #[test]
    fn test_to_tsquery_extra_whitespace() {
        assert_eq!(to_tsquery("  foo   bar  "), "foo:* & bar:*");
    }
}
