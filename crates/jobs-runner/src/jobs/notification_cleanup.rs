use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Delete old read notifications every 24 hours.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(24 * 3600);
    loop {
        cron::tracked(&pool, "notification_cleanup", || async {
            let r = sqlx::query(
                "DELETE FROM notifications WHERE is_read = TRUE AND created_at < NOW() - INTERVAL '30 days'",
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    deleted = r.rows_affected(),
                    "notification_cleanup: old notifications removed"
                );
            }
            Ok(format!("deleted {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
