//! Publish scheduled posts when their `scheduled_at` reaches NOW(). Runs every 60 seconds.

use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(60);
    loop {
        cron::tracked(&pool, "publish_scheduled_posts", || async {
            let r = sqlx::query(
                r#"
                UPDATE posts
                   SET published_at = NOW()
                 WHERE scheduled_at IS NOT NULL
                   AND scheduled_at <= NOW()
                   AND published_at IS NULL
                   AND deleted_at IS NULL
                "#,
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    count = r.rows_affected(),
                    "publish_scheduled_posts: published"
                );
            }
            Ok(format!("published {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
