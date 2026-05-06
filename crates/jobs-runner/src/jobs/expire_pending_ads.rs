//! Pause ads whose budget is exhausted (or whose end_date is past). Runs hourly.

use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        cron::tracked(&pool, "expire_pending_ads", || async {
            let r = sqlx::query(
                r#"
                UPDATE user_ads
                   SET status = 'paused'
                 WHERE status = 'active'
                   AND budget <= 0
                "#,
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(count = r.rows_affected(), "expire_pending_ads: expired");
            }
            Ok(format!("paused {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
