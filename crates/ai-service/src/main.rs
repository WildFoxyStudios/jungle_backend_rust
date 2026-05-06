mod credits;
mod crypto;
mod handlers;
mod providers;
mod routes;

use crate::providers::ProviderRegistry;
use http::{Method, header};
use shared::{auth::AppState, config::AppConfig, db, events};
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

#[tokio::main]
async fn main() {
    shared::telemetry::init("ai-service");

    let config = Arc::new(AppConfig::from_env());
    let pool = db::create_pool(&config.database_url).await;
    db::run_migrations(&pool).await;

    let redis_client = redis::Client::open(config.redis_url.as_str()).expect("Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Redis connect");

    let origins: Vec<_> = config
        .allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::ORIGIN,
            header::COOKIE,
        ]))
        .allow_credentials(true);

    let event_bus = events::connect_event_bus(&config.nats_url).await;

    // Encryption key derived from INTERNAL_SERVICE_KEY (fallback to JWT secret)
    let master_key = std::env::var("INTERNAL_SERVICE_KEY")
        .or_else(|_| std::env::var("JWT_SECRET"))
        .unwrap_or_else(|_| "change-me-insecure-dev-key".into());
    let enc_key = shared::crypto::derive_key(master_key.as_bytes()).to_vec();

    let http = reqwest::Client::new();
    let registry = Arc::new(ProviderRegistry::new(
        pool.clone(),
        http.clone(),
        enc_key.clone(),
    ));

    let app_state = AppState {
        db: pool,
        redis: redis_conn,
        config: config.clone(),
        event_bus,
    };

    let state = handlers::AiState {
        app: app_state,
        http,
        registry,
        enc_key,
    };

    let app = routes::create_router(state)
        .route(
            "/metrics",
            axum::routing::get(shared::metrics::metrics_handler),
        )
        .layer(axum::middleware::from_fn(
            shared::metrics::metrics_middleware,
        ))
        .layer(cors);
    let addr = config.listen_addr();
    tracing::info!("ai-service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
