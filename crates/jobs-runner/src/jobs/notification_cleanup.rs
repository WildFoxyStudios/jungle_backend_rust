use sqlx::PgPool;
use std::time::Duration;

/// Delete old read notifications (every 24 hours)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(24 * 3600);
    loop {
        match sqlx::query(
            "DELETE FROM notifications WHERE is_read = TRUE AND created_at < NOW() - INTERVAL '30 days'",
        )
        .execute(&pool)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!(deleted = result.rows_affected(), "notification_cleanup: old notifications removed");
                }
            }
            Err(e) => tracing::error!(error = %e, "notification_cleanup failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
