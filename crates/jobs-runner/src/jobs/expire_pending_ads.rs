//! Expire ads whose budget is exhausted or whose end_date is past. Runs hourly.

use sqlx::PgPool;
use std::time::Duration;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        match sqlx::query(
            r#"
            UPDATE ads
               SET status = 'expired'
             WHERE status IN ('active', 'pending')
               AND (
                    (bidding IS NOT NULL AND bidding <= 0)
                 OR (budget IS NOT NULL AND spent >= budget)
                 OR (campaign_end IS NOT NULL AND campaign_end < NOW())
               )
            "#,
        )
        .execute(&pool)
        .await
        {
            Ok(r) if r.rows_affected() > 0 => {
                tracing::info!(count = r.rows_affected(), "expire_pending_ads: expired");
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "expire_pending_ads failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
