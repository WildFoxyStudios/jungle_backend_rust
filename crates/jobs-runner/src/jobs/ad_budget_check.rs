use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Pause ads with depleted budgets every 30 minutes.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(30 * 60);
    loop {
        cron::tracked(&pool, "ad_budget_check", || async {
            let r = sqlx::query(
                "UPDATE user_ads SET status = 'paused' WHERE status = 'active' AND budget <= 0",
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    paused = r.rows_affected(),
                    "ad_budget_check: ads paused (budget depleted)"
                );
            }
            Ok(format!("paused {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
