use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Delete expired stories every 5 minutes.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(5 * 60);
    loop {
        cron::tracked(&pool, "story_cleanup", || async {
            let r = sqlx::query("DELETE FROM stories WHERE expires_at < NOW()")
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    deleted = r.rows_affected(),
                    "story_cleanup: expired stories removed"
                );
            }
            Ok(format!("deleted {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
