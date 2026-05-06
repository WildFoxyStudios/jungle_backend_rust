//! Purge messages that have crossed their conversation's disappearing-messages
//! lifetime. Runs every 5 minutes.
//!
//! A message is deleted (soft-deleted via `deleted_at = NOW()`) when:
//!   `NOW() - message.created_at >= conversation.destruct_after_seconds`
//!
//! Soft-deletion keeps the row for analytics while immediately hiding it from
//! all chat handlers (they already filter `WHERE deleted_at IS NULL`).
//!
//! Plan §3.1 — C5.

use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(5 * 60);
    loop {
        cron::tracked(&pool, "disappearing_messages_cleanup", || async {
            let n = sweep(&pool).await.map_err(|e| e.to_string())?;
            if n > 0 {
                tracing::info!(count = n, "disappearing_messages_cleanup: purged");
            }
            Ok(format!("purged {}", n))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}

async fn sweep(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE messages m
        SET deleted_at = NOW()
        FROM conversations c
        WHERE m.conversation_id = c.id
          AND c.destruct_after_seconds IS NOT NULL
          AND m.deleted_at IS NULL
          AND m.created_at < NOW() - (c.destruct_after_seconds * INTERVAL '1 second')
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
