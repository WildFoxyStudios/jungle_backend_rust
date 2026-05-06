use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Purge old login attempts every 6 hours.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(6 * 3600);
    loop {
        cron::tracked(&pool, "login_attempts_cleanup", || async {
            let r = sqlx::query(
                "DELETE FROM login_attempts WHERE attempted_at < NOW() - INTERVAL '24 hours'",
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    deleted = r.rows_affected(),
                    "login_attempts_cleanup: old attempts removed"
                );
            }
            Ok(format!("deleted {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
