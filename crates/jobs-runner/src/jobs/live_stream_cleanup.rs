use sqlx::PgPool;
use std::time::Duration;

/// Reset stale live stream flags for users who haven't pinged in 5 minutes (every 5 minutes)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(5 * 60);
    loop {
        match sqlx::query(
            "UPDATE users SET is_live = FALSE, live_stream_id = NULL WHERE is_live = TRUE AND last_active < NOW() - INTERVAL '5 minutes'",
        )
        .execute(&pool)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!(cleaned = result.rows_affected(), "live_stream_cleanup: stale streams reset");
                }
            }
            Err(e) => tracing::error!(error = %e, "live_stream_cleanup failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
