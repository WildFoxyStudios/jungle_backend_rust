use realtime_service::{event_consumer, hub, routes};

use http::{Method, header};
use shared::{auth::AppState, config::AppConfig, db};
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
#[tokio::main]
async fn main() {
    shared::telemetry::init("realtime-service");

    // Keep realtime-service aligned with gateway defaults.
    // If SERVER_PORT is missing, this service should listen on 3012.
    if std::env::var("SERVER_PORT").is_err() {
        // SAFETY: Set once during startup, before any worker threads are spawned.
        unsafe { std::env::set_var("SERVER_PORT", "3012") };
    }

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

    let event_bus = shared::events::connect_event_bus(&config.nats_url).await;
    let state = AppState {
        db: pool,
        redis: redis_conn,
        config: config.clone(),
        event_bus,
    };

    // Create shared ConnectionHub for both HTTP routes and NATS event relay
    let connection_hub = hub::ConnectionHub::new();

    // Spawn NATS event consumer to relay events to WebSocket clients
    tokio::spawn(event_consumer::spawn_event_consumer(
        state.event_bus.clone(),
        connection_hub.clone(),
        state.db.clone(),
    ));

    let app = routes::create_router_with_hub(state, connection_hub)
        .route(
            "/metrics",
            axum::routing::get(shared::metrics::metrics_handler),
        )
        .layer(axum::middleware::from_fn(
            shared::metrics::metrics_middleware,
        ))
        .layer(cors);
    let addr = config.listen_addr();
    tracing::info!("realtime-service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
