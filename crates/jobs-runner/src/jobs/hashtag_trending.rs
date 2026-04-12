use redis::AsyncCommands;
use sqlx::PgPool;
use std::time::Duration;

#[derive(sqlx::FromRow)]
struct TrendingTag {
    tag: String,
    usage_count: i64,
}

/// Recalculate top 20 trending hashtags (every 1 hour)
pub async fn run(pool: PgPool, mut redis: redis::aio::ConnectionManager) {
    let interval = Duration::from_secs(3600);
    loop {
        let result = sqlx::query_as::<_, TrendingTag>(
            r#"
            SELECT tag, COUNT(*) as usage_count
            FROM hashtag_posts hp
            JOIN hashtags h ON h.id = hp.hashtag_id
            JOIN posts p ON p.id = hp.post_id
            WHERE p.created_at > NOW() - INTERVAL '24 hours'
            GROUP BY tag
            ORDER BY usage_count DESC
            LIMIT 20
            "#,
        )
        .fetch_all(&pool)
        .await;

        match result {
            Ok(tags) => {
                let trending: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|t| serde_json::json!({ "tag": t.tag, "count": t.usage_count }))
                    .collect();

                let json = serde_json::to_string(&trending).unwrap_or_else(|_| "[]".into());
                let _: Result<(), _> = redis.set_ex("trending:hashtags", &json, 3700).await;

                tracing::info!(count = tags.len(), "hashtag_trending: updated");
            }
            Err(e) => tracing::error!(error = %e, "hashtag_trending failed"),
        }

        tokio::time::sleep(interval).await;
    }
}
