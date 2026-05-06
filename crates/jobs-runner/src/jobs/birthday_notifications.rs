use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Daily birthday notifications to followers.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(24 * 3600);
    loop {
        cron::tracked(&pool, "birthday_notifications", || async {
            let r = sqlx::query(
                r#"
                INSERT INTO notifications (recipient_id, sender_id, type, text)
                SELECT f.follower_id, u.id, 'Birthday',
                    u.first_name || '''s birthday is today!'
                FROM users u
                JOIN follows f ON f.following_id = u.id AND f.status = 'active'
                WHERE u.deleted_at IS NULL
                  AND u.birthday IS NOT NULL
                  AND EXTRACT(MONTH FROM u.birthday) = EXTRACT(MONTH FROM NOW())
                  AND EXTRACT(DAY FROM u.birthday) = EXTRACT(DAY FROM NOW())
                  AND NOT EXISTS (
                      SELECT 1 FROM notifications n
                      WHERE n.recipient_id = f.follower_id
                        AND n.sender_id = u.id
                        AND n.type = 'Birthday'
                        AND n.created_at > NOW() - INTERVAL '23 hours'
                  )
                "#,
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    sent = r.rows_affected(),
                    "birthday_notifications: notifications sent"
                );
            }
            Ok(format!("sent {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
