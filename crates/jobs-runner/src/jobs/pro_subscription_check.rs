use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Expire pro subscriptions and notify near-expiry. Runs every hour.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        cron::tracked(&pool, "pro_subscription_check", || async {
            let expired = sqlx::query(
                r#"
                UPDATE pro_subscriptions SET is_active = FALSE
                WHERE is_active = TRUE AND expires_at IS NOT NULL AND expires_at < NOW()
                RETURNING user_id
                "#,
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;

            if !expired.is_empty() {
                tracing::info!(
                    count = expired.len(),
                    "pro_subscription_check: expired subscriptions"
                );

                sqlx::query(
                    r#"
                    UPDATE users SET is_pro = FALSE
                    WHERE is_pro = TRUE AND id NOT IN (
                        SELECT user_id FROM pro_subscriptions WHERE is_active = TRUE
                    )
                    "#,
                )
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
            }

            // Notify users expiring in next 3 days (idempotent within 24h).
            let notified = sqlx::query(
                r#"
                INSERT INTO notifications (recipient_id, sender_id, type, text)
                SELECT ps.user_id, ps.user_id, 'ProExpiring',
                    'Your Pro subscription expires in less than 3 days'
                FROM pro_subscriptions ps
                WHERE ps.is_active = TRUE
                  AND ps.expires_at BETWEEN NOW() AND NOW() + INTERVAL '3 days'
                  AND NOT EXISTS (
                    SELECT 1 FROM notifications n
                    WHERE n.recipient_id = ps.user_id AND n.type = 'ProExpiring'
                      AND n.created_at > NOW() - INTERVAL '1 day'
                  )
                "#,
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;

            Ok(format!(
                "expired {}, notified {}",
                expired.len(),
                notified.rows_affected()
            ))
        })
        .await;

        tokio::time::sleep(interval).await;
    }
}
