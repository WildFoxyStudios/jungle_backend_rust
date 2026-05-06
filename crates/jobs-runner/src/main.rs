mod cron;
mod jobs;

use shared::{config::AppConfig, db, events::NatsEventBus};
use std::sync::Arc;
#[tokio::main]
async fn main() {
    shared::telemetry::init("jobs-runner");

    let config = Arc::new(AppConfig::from_env());
    let pool = db::create_pool(&config.database_url).await;
    db::run_migrations(&pool).await;

    let redis_client = redis::Client::open(config.redis_url.as_str()).expect("Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Redis connect");

    // Try to connect to NATS for DLQ consumer (optional — jobs still run without it)
    let nats_bus = NatsEventBus::connect(&config.nats_url).await.ok();

    tracing::info!("jobs-runner started — scheduling background tasks");

    let mut handles = vec![
        tokio::spawn(jobs::story_cleanup::run(pool.clone())),
        tokio::spawn(jobs::session_cleanup::run(pool.clone())),
        tokio::spawn(jobs::notification_cleanup::run(pool.clone())),
        tokio::spawn(jobs::pro_subscription_check::run(pool.clone())),
        tokio::spawn(jobs::event_reminders::run(pool.clone())),
        tokio::spawn(jobs::ad_budget_check::run(pool.clone())),
        tokio::spawn(jobs::hashtag_trending::run(
            pool.clone(),
            redis_conn.clone(),
        )),
        tokio::spawn(jobs::login_attempts_cleanup::run(pool.clone())),
        tokio::spawn(jobs::memories_notification::run(pool.clone())),
        tokio::spawn(jobs::moderation_dispatch::run(pool.clone())),
        tokio::spawn(jobs::birthday_notifications::run(pool.clone())),
        tokio::spawn(jobs::live_stream_cleanup::run(pool.clone())),
        // New jobs added in Batch 3
        tokio::spawn(jobs::publish_scheduled_posts::run(pool.clone())),
        tokio::spawn(jobs::auto_delete_old_messages::run(pool.clone())),
        tokio::spawn(jobs::weekly_memories_digest::run(pool.clone())),
        tokio::spawn(jobs::analytics_snapshot_daily::run(pool.clone())),
        tokio::spawn(jobs::crypto_payment_reconciliation::run(pool.clone())),
        tokio::spawn(jobs::expire_pending_ads::run(pool.clone())),
        tokio::spawn(jobs::newsletter_dispatcher::run(pool.clone())),
        tokio::spawn(jobs::disappearing_messages_cleanup::run(pool.clone())),
        tokio::spawn(jobs::unmute_expired_conversations::run(pool.clone())),
        tokio::spawn(jobs::post_viewers_cleanup::run(pool.clone())),
        // Phase 6 — Feed EdgeRank scoring (every ~30m)
        tokio::spawn(jobs::feed_ranking::run(pool.clone())),
        // Phase 8 — Recommendations PYMK (daily at 03:00 UTC)
        tokio::spawn(jobs::pagerank::run(pool.clone())),
        // Phase 19 — Webhooks & GDPR
        tokio::spawn(jobs::webhooks_dispatcher::run(pool.clone())),
        tokio::spawn(jobs::erasure_processor::run(pool.clone())),
    ];

    // DLQ consumer runs only when NATS is reachable
    if let Some(bus) = nats_bus {
        handles.push(tokio::spawn(jobs::dlq_consumer::run(pool.clone(), bus)));
    } else {
        tracing::warn!("NATS not reachable — DLQ consumer disabled");
    }

    // Wait for all — they loop forever unless error
    for handle in handles {
        if let Err(e) = handle.await {
            tracing::error!(error = %e, "Background job panicked");
        }
    }
}
