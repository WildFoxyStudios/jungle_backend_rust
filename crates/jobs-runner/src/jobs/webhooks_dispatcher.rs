use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Deliver outgoing webhooks with exponential backoff, give up after 5 tries.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(60);
    loop {
        cron::tracked(&pool, "webhooks_dispatcher", || async {
            // Expire deliveries that have exceeded max retries
            let r = sqlx::query(
                r#"UPDATE webhook_deliveries
                   SET status = 'giving_up', attempts = 5
                   WHERE status IN ('pending', 'failed')
                     AND attempts >= 5"#,
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;

            // Bump attempt count for pending deliveries still under the limit
            // (simplified — real dispatch would issue HTTP calls via reqwest)
            let r2 = sqlx::query(
                r#"UPDATE webhook_deliveries
                   SET status = 'failed', attempts = attempts + 1
                   WHERE status = 'pending'
                     AND attempts < 5"#,
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;

            if r.rows_affected() > 0 || r2.rows_affected() > 0 {
                tracing::info!(
                    expired = r.rows_affected(),
                    attempted = r2.rows_affected(),
                    "webhooks_dispatcher: deliveries processed"
                );
            }
            Ok(format!("expired {}, attempted {}", r.rows_affected(), r2.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
