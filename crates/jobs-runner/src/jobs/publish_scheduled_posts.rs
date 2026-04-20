//! Publish scheduled posts when their `scheduled_at` reaches NOW().
//! Runs every 60 seconds.

use sqlx::PgPool;
use std::time::Duration;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(60);
    loop {
        match sqlx::query(
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
        {
            Ok(r) if r.rows_affected() > 0 => {
                tracing::info!(count = r.rows_affected(), "publish_scheduled_posts: published");
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "publish_scheduled_posts failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
