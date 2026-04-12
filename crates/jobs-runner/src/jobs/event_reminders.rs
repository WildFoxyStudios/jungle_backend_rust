use sqlx::PgPool;
use std::time::Duration;

/// Send event reminders 24h before start (every 1 hour)
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        let result = sqlx::query(
            r#"
            INSERT INTO notifications (recipient_id, sender_id, type, target_type, target_id, text)
            SELECT er.user_id, e.user_id, 'EventReminder', 'event', e.id,
                CONCAT('Reminder: "', e.name, '" starts in less than 24 hours')
            FROM events e
            JOIN event_responses er ON er.event_id = e.id AND er.response IN ('going', 'interested')
            WHERE e.start_date BETWEEN NOW() AND NOW() + INTERVAL '24 hours'
              AND NOT EXISTS (
                SELECT 1 FROM notifications n
                WHERE n.recipient_id = er.user_id AND n.type = 'EventReminder'
                  AND n.target_id = e.id AND n.created_at > NOW() - INTERVAL '12 hours'
              )
            "#,
        )
        .execute(&pool)
        .await;

        match result {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    tracing::info!(sent = r.rows_affected(), "event_reminders: notifications sent");
                }
            }
            Err(e) => tracing::error!(error = %e, "event_reminders failed"),
        }

        tokio::time::sleep(interval).await;
    }
}
