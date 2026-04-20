//! Aggregate previous day's metrics into `daily_analytics`. Runs at 00:10 UTC.

use sqlx::PgPool;
use std::time::Duration;
use time::OffsetDateTime;

pub async fn run(pool: PgPool) {
    // Ensure target table exists (idempotent).
    let _ = sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS daily_analytics (
            date               DATE PRIMARY KEY,
            new_users          INTEGER NOT NULL DEFAULT 0,
            active_users       INTEGER NOT NULL DEFAULT 0,
            posts_created      INTEGER NOT NULL DEFAULT 0,
            messages_sent      INTEGER NOT NULL DEFAULT 0,
            calls_started      INTEGER NOT NULL DEFAULT 0,
            revenue_cents      BIGINT NOT NULL DEFAULT 0,
            created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(&pool)
    .await;

    let check_every = Duration::from_secs(15 * 60);
    loop {
        let now = OffsetDateTime::now_utc();
        // Run once at minute 10 past midnight
        if now.hour() == 0 && now.minute() >= 10 && now.minute() < 25 {
            if let Err(e) = snapshot(&pool).await {
                tracing::error!(error = %e, "analytics_snapshot_daily failed");
            }
            // Sleep the rest of the hour to avoid double-running
            tokio::time::sleep(Duration::from_secs(3600)).await;
            continue;
        }
        tokio::time::sleep(check_every).await;
    }
}

async fn snapshot(pool: &PgPool) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"
        INSERT INTO daily_analytics
            (date, new_users, active_users, posts_created, messages_sent, calls_started, revenue_cents)
        SELECT
            (CURRENT_DATE - 1)::date,
            (SELECT COUNT(*) FROM users WHERE DATE(created_at) = CURRENT_DATE - 1),
            (SELECT COUNT(DISTINCT user_id) FROM user_sessions WHERE DATE(last_activity_at) = CURRENT_DATE - 1),
            (SELECT COUNT(*) FROM posts WHERE DATE(created_at) = CURRENT_DATE - 1 AND deleted_at IS NULL),
            (SELECT COUNT(*) FROM messages WHERE DATE(created_at) = CURRENT_DATE - 1),
            (SELECT COUNT(*) FROM calls WHERE DATE(created_at) = CURRENT_DATE - 1),
            (SELECT COALESCE(SUM(CAST(amount AS BIGINT) * 100), 0) FROM transactions
              WHERE DATE(created_at) = CURRENT_DATE - 1 AND status = 'completed')
        ON CONFLICT (date) DO UPDATE SET
            new_users     = EXCLUDED.new_users,
            active_users  = EXCLUDED.active_users,
            posts_created = EXCLUDED.posts_created,
            messages_sent = EXCLUDED.messages_sent,
            calls_started = EXCLUDED.calls_started,
            revenue_cents = EXCLUDED.revenue_cents
        "#,
    )
    .execute(pool)
    .await?;
    tracing::info!("analytics_snapshot_daily: previous day snapshot inserted");
    Ok(())
}
