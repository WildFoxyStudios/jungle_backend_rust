use sqlx::PgPool;
use std::time::Duration;

/// Delete expired stories (every 5 minutes)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(5 * 60);
    loop {
        match sqlx::query("DELETE FROM stories WHERE expires_at < NOW()")
            .execute(&pool)
            .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!(deleted = result.rows_affected(), "story_cleanup: expired stories removed");
                }
            }
            Err(e) => tracing::error!(error = %e, "story_cleanup failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
