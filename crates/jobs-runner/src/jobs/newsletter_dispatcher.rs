//! Dispatch queued newsletter emails in batches. Runs every 5 minutes.
//! Uses `shared::email::send_email` (SMTP via lettre).

use shared::email;
use sqlx::PgPool;
use std::time::Duration;

#[derive(sqlx::FromRow)]
struct QueuedEmail {
    id: i64,
    recipient_email: String,
    subject: String,
    body: String,
    attempts: i32,
}

pub async fn run(pool: PgPool) {
    // Ensure queue table exists (idempotent)
    let _ = sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS newsletter_queue (
            id               BIGSERIAL PRIMARY KEY,
            campaign_id      BIGINT,
            recipient_email  VARCHAR(254) NOT NULL,
            recipient_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
            subject          VARCHAR(255) NOT NULL,
            body             TEXT NOT NULL,
            status           VARCHAR(20) NOT NULL DEFAULT 'pending',
            attempts         INTEGER NOT NULL DEFAULT 0,
            error_message    TEXT,
            sent_at          TIMESTAMPTZ,
            created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(&pool)
    .await;

    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_newsletter_queue_pending ON newsletter_queue (status, attempts) WHERE status = 'pending'",
    ).execute(&pool).await;

    let interval = Duration::from_secs(5 * 60);
    const BATCH: i64 = 50;

    loop {
        match sqlx::query_as::<_, QueuedEmail>(
            r#"SELECT id, recipient_email, subject, body, attempts
                 FROM newsletter_queue
                WHERE status = 'pending' AND attempts < 5
             ORDER BY created_at ASC
                LIMIT $1"#,
        )
        .bind(BATCH)
        .fetch_all(&pool)
        .await
        {
            Ok(batch) if !batch.is_empty() => {
                let mut ok = 0u64;
                for msg in batch {
                    let res = email::send_email(&msg.recipient_email, &msg.subject, &msg.body).await;
                    match res {
                        Ok(()) => {
                            let _ = sqlx::query(
                                "UPDATE newsletter_queue SET status='sent', sent_at=NOW() WHERE id=$1",
                            )
                            .bind(msg.id)
                            .execute(&pool)
                            .await;
                            ok += 1;
                        }
                        Err(err) => {
                            let next_status = if msg.attempts + 1 >= 5 { "failed" } else { "pending" };
                            let _ = sqlx::query(
                                r#"UPDATE newsletter_queue
                                      SET attempts = attempts + 1,
                                          status = $2,
                                          error_message = $3
                                    WHERE id = $1"#,
                            )
                            .bind(msg.id)
                            .bind(next_status)
                            .bind(err)
                            .execute(&pool)
                            .await;
                        }
                    }
                }
                if ok > 0 {
                    tracing::info!(sent = ok, "newsletter_dispatcher: batch sent");
                }
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "newsletter_dispatcher poll failed"),
        }
        tokio::time::sleep(interval).await;
    }
}
