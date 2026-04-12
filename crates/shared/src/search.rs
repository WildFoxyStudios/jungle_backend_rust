use axum::Json;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::errors::ApiError;

/// Global search across all entity types
pub async fn search_all(
    db: &PgPool,
    query: &str,
    search_type: Option<&str>,
    viewer_id: Option<i64>,
    limit: i64,
) -> Result<Json<Value>, ApiError> {
    let tsquery = to_tsquery(query);
    let ilike = format!("%{}%", crate::sanitize::sanitize_search_query(query));
    let prefix = format!(
        "{}%",
        crate::sanitize::sanitize_search_query(query)
    );
    let limit = limit.clamp(1, 50);

    match search_type.unwrap_or("all") {
        "user" => {
            let users = sqlx::query_as::<_, SearchResult>(
                r#"
                SELECT id, username AS name, avatar AS image, 'user' AS result_type,
                    CASE WHEN username ILIKE $2 THEN 0 ELSE 1 END AS rank
                FROM users
                WHERE deleted_at IS NULL AND is_active = TRUE
                    AND (username ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1)
                    AND ($4::bigint IS NULL OR id NOT IN (
                        SELECT blocked_id FROM blocks WHERE blocker_id = $4
                        UNION ALL
                        SELECT blocker_id FROM blocks WHERE blocked_id = $4
                    ))
                ORDER BY rank, is_verified DESC
                LIMIT $3
                "#,
            )
            .bind(&ilike)
            .bind(&prefix)
            .bind(limit)
            .bind(viewer_id)
            .fetch_all(db)
            .await?;

            Ok(Json(json!({ "data": users, "type": "users" })))
        }

        "post" => {
            let posts = sqlx::query_as::<_, SearchResult>(
                r#"
                SELECT p.id, LEFT(p.content, 200) AS name, NULL AS image, 'post' AS result_type,
                    ts_rank(p.search_vector, to_tsquery('simple', $1))::real AS rank
                FROM posts p
                WHERE p.deleted_at IS NULL AND p.is_approved = TRUE AND p.privacy = 'everyone'
                    AND p.search_vector @@ to_tsquery('simple', $1)
                ORDER BY rank DESC
                LIMIT $2
                "#,
            )
            .bind(&tsquery)
            .bind(limit)
            .fetch_all(db)
            .await?;

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
            let blogs = sqlx::query_as::<_, SearchResult>(
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
            .bind(&tsquery)
            .bind(limit)
            .fetch_all(db)
            .await?;

            Ok(Json(json!({ "data": blogs, "type": "blogs" })))
        }

        "product" => {
            let products = sqlx::query_as::<_, SearchResult>(
                r#"
                SELECT id, name, NULL AS image, 'product' AS result_type, 0::real AS rank
                FROM products WHERE status = 'active'
                    AND (name ILIKE $1 OR description ILIKE $1)
                ORDER BY rating DESC
                LIMIT $2
                "#,
            )
            .bind(&ilike)
            .bind(limit)
            .fetch_all(db)
            .await?;

            Ok(Json(json!({ "data": products, "type": "products" })))
        }

        // "all" — search everything in parallel
        _ => {
            let (users, pages, groups, hashtags) = tokio::join!(
                search_type_internal(db, "user", &ilike, &prefix, &tsquery, 5),
                search_type_internal(db, "page", &ilike, &prefix, &tsquery, 5),
                search_type_internal(db, "group", &ilike, &prefix, &tsquery, 5),
                search_type_internal(db, "hashtag", &ilike, &prefix, &tsquery, 5),
            );
            Ok(Json(json!({
                "data": {
                    "users": users.unwrap_or_default(),
                    "pages": pages.unwrap_or_default(),
                    "groups": groups.unwrap_or_default(),
                    "hashtags": hashtags.unwrap_or_default(),
                }
            })))
        }
    }
}

async fn search_type_internal(
    db: &PgPool,
    search_type: &str,
    ilike: &str,
    _prefix: &str,
    _tsquery: &str,
    limit: i64,
) -> Result<Vec<SearchResult>, ApiError> {
    let query = match search_type {
        "user" => {
            "SELECT id, username AS name, avatar AS image, 'user' AS result_type, 0::real AS rank FROM users WHERE deleted_at IS NULL AND is_active = TRUE AND (username ILIKE $1 OR first_name ILIKE $1) ORDER BY is_verified DESC LIMIT $2"
        }
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
        .bind(ilike)
        .bind(limit)
        .fetch_all(db)
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
