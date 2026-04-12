use sqlx::PgPool;
use std::time::Duration;

/// Deactivate ads with depleted budgets (every 30 minutes)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(30 * 60);
    loop {
        match sqlx::query(
            "UPDATE user_ads SET status = 'paused' WHERE status = 'active' AND budget <= 0",
        )
        .execute(&pool)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!(paused = result.rows_affected(), "ad_budget_check: ads paused (budget depleted)");
                }
            }
            Err(e) => tracing::error!(error = %e, "ad_budget_check failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
