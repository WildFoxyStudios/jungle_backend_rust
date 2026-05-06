use sqlx::PgPool;
use std::time::Duration;

use crate::cron;

/// Anonymise users whose GDPR erasure date has passed.
pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(3600);
    loop {
        cron::tracked(&pool, "erasure_processor", || async {
            let r = sqlx::query(
                r#"UPDATE users
                   SET deleted_at = NOW(),
                       first_name = 'Deleted',
                       last_name = 'User',
                       username = CONCAT('deleted_', id),
                       email = CONCAT('deleted_', id, '@erased.local'),
                       avatar = NULL,
                       cover = NULL,
                       bio = NULL,
                       erasure_scheduled_at = NULL
                   WHERE erasure_scheduled_at IS NOT NULL
                     AND erasure_scheduled_at <= NOW()
                     AND deleted_at IS NULL"#,
            )
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;

            if r.rows_affected() > 0 {
                tracing::info!(
                    erased = r.rows_affected(),
                    "erasure_processor: users erased"
                );
            }
            Ok(format!("erased {}", r.rows_affected()))
        })
        .await;
        tokio::time::sleep(interval).await;
    }
}
