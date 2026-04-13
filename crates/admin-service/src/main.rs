mod handlers;
mod routes;

use shared::{auth::AppState, config::AppConfig, db};
use std::sync::Arc;
use http::header;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
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

    let origins: Vec<_> = config.allowed_origins.iter().filter_map(|o| o.parse().ok()).collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::list([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::ORIGIN,
            header::COOKIE,
        ]))
        .allow_credentials(true);

    let event_bus: std::sync::Arc<dyn shared::events::EventBus> = match shared::events::NatsEventBus::connect(&config.nats_url).await {
        Ok(bus) => std::sync::Arc::new(bus),
        Err(e) => { tracing::warn!("NATS unavailable: {e}"); std::sync::Arc::new(shared::events::NoopEventBus) }
    };
    let state = AppState { db: pool, redis: redis_conn, config: config.clone(), event_bus };
    let app = routes::create_router(state)
        .route("/metrics", axum::routing::get(shared::metrics::metrics_handler))
        .layer(axum::middleware::from_fn(shared::metrics::metrics_middleware))
        .layer(cors);
    let addr = config.listen_addr();
    tracing::info!("admin-service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
