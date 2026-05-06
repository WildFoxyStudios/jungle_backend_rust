mod handlers;
mod routes;

use http::{Method, header};
use shared::{auth::AppState, config::AppConfig, db};
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
#[tokio::main]
async fn main() {
    shared::telemetry::init("auth-service");

    let config = Arc::new(AppConfig::from_env());
    let pool = db::create_pool(&config.database_url).await;

    tracing::info!("Running database migrations...");
    db::run_migrations(&pool).await;

    let redis_client =
        redis::Client::open(config.redis_url.as_str()).expect("Failed to create Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

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
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600));

    let event_bus = shared::events::connect_event_bus(&config.nats_url).await;
    let state = AppState {
        db: pool,
        redis: redis_conn,
        config: config.clone(),
        event_bus,
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

    tracing::info!("auth-service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    // `into_make_service_with_connect_info` is needed so the login
    // handler can access the peer's `SocketAddr` via `ConnectInfo` —
    // we use it to drive the unusual-login detection.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
