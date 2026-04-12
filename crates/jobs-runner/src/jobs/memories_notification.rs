use sqlx::PgPool;
use std::time::Duration;

/// "On this day" memories — notify users with posts from same date in past years (every 24 hours)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(24 * 3600);
    loop {
        match sqlx::query(
            r#"
            INSERT INTO notifications (recipient_id, sender_id, type, text)
            SELECT DISTINCT p.user_id, p.user_id, 'Memory',
                'You have memories from this day!'
            FROM posts p
            WHERE p.deleted_at IS NULL
              AND EXTRACT(MONTH FROM p.created_at) = EXTRACT(MONTH FROM NOW())
              AND EXTRACT(DAY FROM p.created_at) = EXTRACT(DAY FROM NOW())
              AND p.created_at < NOW() - INTERVAL '1 year'
              AND NOT EXISTS (
                  SELECT 1 FROM notifications n
                  WHERE n.recipient_id = p.user_id AND n.type = 'Memory'
                    AND n.created_at > NOW() - INTERVAL '23 hours'
              )
            "#,
        )
        .execute(&pool)
        .await
        {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    tracing::info!(sent = r.rows_affected(), "memories_notification: notifications created");
                }
            }
            Err(e) => tracing::error!(error = %e, "memories_notification failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
