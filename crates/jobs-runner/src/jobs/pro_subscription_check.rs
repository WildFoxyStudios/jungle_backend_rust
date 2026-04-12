use sqlx::PgPool;
use std::time::Duration;

/// Check and expire pro subscriptions (every 1 hour)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        // Expire subscriptions past their date
        match sqlx::query(
            r#"
            UPDATE pro_subscriptions SET is_active = FALSE
            WHERE is_active = TRUE AND expires_at IS NOT NULL AND expires_at < NOW()
            RETURNING user_id
            "#,
        )
        .fetch_all(&pool)
        .await
        {
            Ok(rows) => {
                if !rows.is_empty() {
                    tracing::info!(count = rows.len(), "pro_subscription_check: expired subscriptions");

                    // Set is_pro = FALSE for users with no active subs
                    let _ = sqlx::query(
                        r#"
                        UPDATE users SET is_pro = FALSE
                        WHERE is_pro = TRUE AND id NOT IN (
                            SELECT user_id FROM pro_subscriptions WHERE is_active = TRUE
                        )
                        "#,
                    )
                    .execute(&pool)
                    .await;
                }
            }
            Err(e) => tracing::error!(error = %e, "pro_subscription_check failed"),
        }

        // Notify users expiring in next 3 days
        let _ = sqlx::query(
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
        .await;

        tokio::time::sleep(interval).await;
    }
}
