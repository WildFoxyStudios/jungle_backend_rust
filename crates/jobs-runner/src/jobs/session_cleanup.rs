use sqlx::PgPool;
use std::time::Duration;

/// Delete expired sessions (every 1 hour)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        match sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
            .execute(&pool)
            .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!(deleted = result.rows_affected(), "session_cleanup: expired sessions removed");
                }
            }
            Err(e) => tracing::error!(error = %e, "session_cleanup failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
