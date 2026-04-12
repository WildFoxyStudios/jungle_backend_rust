mod jobs;

use shared::{config::AppConfig, db};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(AppConfig::from_env());
    let pool = db::create_pool(&config.database_url).await;
    db::run_migrations(&pool).await;

    let redis_client = redis::Client::open(config.redis_url.as_str()).expect("Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Redis connect");

    tracing::info!("jobs-runner started — scheduling background tasks");

    // Spawn all 11 background job loops
    let handles = vec![
        tokio::spawn(jobs::story_cleanup::run(pool.clone())),
        tokio::spawn(jobs::session_cleanup::run(pool.clone())),
        tokio::spawn(jobs::notification_cleanup::run(pool.clone())),
        tokio::spawn(jobs::pro_subscription_check::run(pool.clone())),
        tokio::spawn(jobs::event_reminders::run(pool.clone())),
        tokio::spawn(jobs::ad_budget_check::run(pool.clone())),
        tokio::spawn(jobs::hashtag_trending::run(pool.clone(), redis_conn.clone())),
        tokio::spawn(jobs::login_attempts_cleanup::run(pool.clone())),
        tokio::spawn(jobs::memories_notification::run(pool.clone())),
        tokio::spawn(jobs::birthday_notifications::run(pool.clone())),
        tokio::spawn(jobs::live_stream_cleanup::run(pool.clone())),
    ];

    // Wait for all — they loop forever unless error
    for handle in handles {
        if let Err(e) = handle.await {
            tracing::error!(error = %e, "Background job panicked");
        }
    }
}
