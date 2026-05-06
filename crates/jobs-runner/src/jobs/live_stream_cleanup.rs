use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Reset stale live-stream flags every 5 minutes.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(5 * 60);
    loop {
        cron::tracked(&pool, "live_stream_cleanup", || async {
            let r = sqlx::query(
                "UPDATE users SET is_live = FALSE, live_stream_id = NULL WHERE is_live = TRUE AND last_seen < NOW() - INTERVAL '5 minutes'",
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(cleaned = r.rows_affected(), "live_stream_cleanup: stale streams reset");
            }
            Ok(format!("cleaned {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
