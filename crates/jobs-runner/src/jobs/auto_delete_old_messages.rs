//! Delete messages older than the configured retention window. Runs every 6 hours.
//! Respects `site_config.auto_delete_messages_days` (0 = disabled).

use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(6 * 3600);
    loop {
        let days: Option<i32> = sqlx::query_scalar(
            r#"SELECT CAST(value AS INTEGER)
                 FROM site_config
                WHERE category = 'auto_delete' AND key = 'messages_days'"#,
        )
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

        let days = days.unwrap_or(0);
        if days <= 0 {
            cron::skipped(&pool, "auto_delete_old_messages", "retention disabled").await;
            tokio::time::sleep(interval).await;
            continue;
        }

        cron::tracked(&pool, "auto_delete_old_messages", || async {
            // Plain placeholder cannot bind into INTERVAL literal directly.
            let sql = format!(
                "DELETE FROM messages WHERE created_at < NOW() - INTERVAL '{} days' AND deleted_at IS NULL",
                days
            );
            let r = sqlx::query(&sql)
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
            if r.rows_affected() > 0 {
                tracing::info!(
                    count = r.rows_affected(),
                    days,
                    "auto_delete_old_messages: deleted"
                );
            }
            Ok(format!("deleted {} (>{}d)", r.rows_affected(), days))
        })
        .await;

        tokio::time::sleep(interval).await;
    }
}
