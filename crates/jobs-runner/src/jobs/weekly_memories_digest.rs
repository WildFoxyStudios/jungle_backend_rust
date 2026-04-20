//! Weekly memories digest: every Monday 09:00 UTC, queue a "Memories" notification
//! for each user whose post history has anniversaries this week (1/2/3 years ago).

use sqlx::PgPool;
use std::time::Duration;
use time::OffsetDateTime;

pub async fn run(pool: PgPool) {
    // Check every 15 minutes; only act on Monday 09:00 UTC.
    let check_every = Duration::from_secs(15 * 60);
    loop {
        let now = OffsetDateTime::now_utc();
        let is_monday_9 = now.weekday().number_from_monday() == 1
            && now.hour() == 9
            && now.minute() < 15;

        if is_monday_9 {
            match run_once(&pool).await {
                Ok(count) if count > 0 => {
                    tracing::info!(users_notified = count, "weekly_memories_digest: queued");
                }
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "weekly_memories_digest failed"),
            }
            // Sleep a full hour to avoid double-running in the same window
            tokio::time::sleep(Duration::from_secs(3600)).await;
            continue;
        }

        tokio::time::sleep(check_every).await;
    }
}

async fn run_once(pool: &PgPool) -> Result<u64, sqlx::Error> {
    // Users with posts made exactly 1, 2 or 3 years ago this week
    let result = sqlx::query(
        r#"
        INSERT INTO notifications (recipient_id, sender_id, type, text)
        SELECT DISTINCT p.user_id,
                        p.user_id,
                        'Memories',
                        'You have new memories from previous years. Take a look!'
          FROM posts p
         WHERE p.deleted_at IS NULL
           AND (
                (p.created_at >= NOW() - INTERVAL '1 year 7 days' AND p.created_at < NOW() - INTERVAL '1 year')
             OR (p.created_at >= NOW() - INTERVAL '2 years 7 days' AND p.created_at < NOW() - INTERVAL '2 years')
             OR (p.created_at >= NOW() - INTERVAL '3 years 7 days' AND p.created_at < NOW() - INTERVAL '3 years')
           )
           AND NOT EXISTS (
               SELECT 1 FROM notifications n
                WHERE n.recipient_id = p.user_id
                  AND n.type = 'Memories'
                  AND n.created_at > NOW() - INTERVAL '6 days'
           )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
