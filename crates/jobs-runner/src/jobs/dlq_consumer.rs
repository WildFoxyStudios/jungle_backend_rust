//! DLQ consumer: subscribes to `dlq.>` NATS subjects and persists failed events
//! to the `event_dlq` table for manual inspection / retry from the admin UI.

use futures::StreamExt;
use shared::events::NatsEventBus;
use sqlx::PgPool;

pub async fn run(pool: PgPool, bus: NatsEventBus) {
    // Subscribe to every dlq.> subject
    let mut sub = match bus.client().subscribe("dlq.>").await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "DLQ consumer failed to subscribe");
            return;
        }
    };

    tracing::info!("DLQ consumer started — listening on dlq.>");

    let mut seen_this_minute: u32 = 0;
    let mut last_reset = std::time::Instant::now();

    while let Some(msg) = sub.next().await {
        let subject = msg.subject.to_string();
        let payload: serde_json::Value =
            serde_json::from_slice(&msg.payload).unwrap_or_else(|_| {
                serde_json::Value::String(String::from_utf8_lossy(&msg.payload).into_owned())
            });

        let result = sqlx::query(
            r#"INSERT INTO event_dlq (subject, payload, error)
               VALUES ($1, $2, $3)"#,
        )
        .bind(&subject)
        .bind(&payload)
        .bind(Option::<String>::None)
        .execute(&pool)
        .await;

        if let Err(e) = result {
            tracing::error!(subject, error = %e, "failed to persist DLQ message");
            continue;
        }

        seen_this_minute += 1;
        let elapsed = last_reset.elapsed().as_secs();
        if elapsed >= 60 {
            if seen_this_minute > 100 {
                tracing::error!(
                    count = seen_this_minute,
                    "DLQ burst detected: >100 messages in 1 minute — investigate upstream services"
                );
            }
            seen_this_minute = 0;
            last_reset = std::time::Instant::now();
        }
    }

    tracing::warn!("DLQ subscription ended");
}
