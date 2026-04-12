use sqlx::PgPool;
use std::time::Duration;

/// Clean up old login attempts (every 6 hours)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(6 * 3600);
    loop {
        match sqlx::query(
            "DELETE FROM login_attempts WHERE attempted_at < NOW() - INTERVAL '24 hours'",
        )
        .execute(&pool)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!(deleted = result.rows_affected(), "login_attempts_cleanup: old attempts removed");
                }
            }
            Err(e) => tracing::error!(error = %e, "login_attempts_cleanup failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
