//! Delete messages older than the configured retention window. Runs every 6 hours.
//! Respects `site_config.auto_delete_messages_days` (0 = disabled).

use sqlx::PgPool;
use std::time::Duration;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(6 * 3600);
    loop {
        // Read retention window from site_config
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
            tokio::time::sleep(interval).await;
            continue;
        }

        let sql = format!(
            "DELETE FROM messages WHERE created_at < NOW() - INTERVAL '{} days' AND deleted_at IS NULL",
            days
        );

        match sqlx::query(&sql).execute(&pool).await {
            Ok(r) if r.rows_affected() > 0 => {
                tracing::info!(count = r.rows_affected(), days, "auto_delete_old_messages: deleted");
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "auto_delete_old_messages failed"),
        }

        tokio::time::sleep(interval).await;
    }
}
