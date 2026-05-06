//! Clear `muted_until` on conversation_members whose mute window has elapsed.
//! Runs every 15 minutes.

use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(15 * 60);
    loop {
        cron::tracked(&pool, "unmute_expired_conversations", || async {
            let n = sweep(&pool).await.map_err(|e| e.to_string())?;
            if n > 0 {
                tracing::info!(count = n, "unmute_expired_conversations: cleared");
            }
            Ok(format!("cleared {}", n))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}

async fn sweep(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE conversation_members
        SET muted = FALSE, muted_until = NULL
        WHERE muted_until IS NOT NULL
          AND muted_until < NOW()
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
