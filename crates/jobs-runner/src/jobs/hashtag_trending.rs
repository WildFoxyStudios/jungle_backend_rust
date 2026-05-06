use redis::AsyncCommands;
use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

#[derive(sqlx::FromRow)]
struct TrendingTag {
    tag: String,
    usage_count: i64,
}

/// Recalculate top 20 trending hashtags every hour.
pub async fn run(pool: PgPool, mut redis: redis::aio::ConnectionManager) {
    let interval = Duration::from_secs(3600);
    loop {
        cron::tracked(&pool, "hashtag_trending", || async {
            let tags = sqlx::query_as::<_, TrendingTag>(
                r#"
                SELECT tag, COUNT(*) as usage_count
                FROM post_hashtags hp
                JOIN hashtags h ON h.id = hp.hashtag_id
                JOIN posts p ON p.id = hp.post_id
                WHERE p.created_at > NOW() - INTERVAL '24 hours'
                GROUP BY tag
                ORDER BY usage_count DESC
                LIMIT 20
                "#,
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;

            let trending: Vec<serde_json::Value> = tags
                .iter()
                .map(|t| serde_json::json!({ "tag": t.tag, "count": t.usage_count }))
                .collect();
            let json = serde_json::to_string(&trending).unwrap_or_else(|_| "[]".into());
            redis
                .set_ex::<_, _, ()>("trending:hashtags", &json, 3700)
                .await
                .map_err(|e| e.to_string())?;

            tracing::info!(count = tags.len(), "hashtag_trending: updated");
            Ok(format!("top {} tags refreshed", tags.len()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
