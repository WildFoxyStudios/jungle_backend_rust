use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Delete expired sessions every hour.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        cron::tracked(&pool, "session_cleanup", || async {
            let r = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    deleted = r.rows_affected(),
                    "session_cleanup: expired sessions removed"
                );
            }
            Ok(format!("deleted {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
