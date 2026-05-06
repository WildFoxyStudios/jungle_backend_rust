//! Trim old `post_viewers` rows to keep the table from growing indefinitely.
//! Runs once per day. Retention window is 90 days; after that, the detailed
//! viewers log is considered redundant (the `posts.post_views` counter is
//! the long-term source of truth).

use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

const RETENTION_DAYS: i32 = 90;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(24 * 60 * 60);
    loop {
        cron::tracked(&pool, "post_viewers_cleanup", || async {
            let n = sweep(&pool).await.map_err(|e| e.to_string())?;
            if n > 0 {
                tracing::info!(count = n, "post_viewers_cleanup: trimmed");
            }
            Ok(format!("deleted {}", n))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}

async fn sweep(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM post_viewers
        WHERE viewed_at < NOW() - ($1::int * INTERVAL '1 day')
        "#,
    )
    .bind(RETENTION_DAYS)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
