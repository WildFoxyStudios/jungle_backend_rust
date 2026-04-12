mod openapi;
mod proxy;
mod rate_limit;
mod routing;

use shared::config::AppConfig;
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(AppConfig::from_env());

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
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .allow_credentials(true);

    let service_map = routing::ServiceMap::from_env();
    let rate_limiter = rate_limit::RateLimiter::new(redis_conn);

    let state = proxy::GatewayState {
        client: reqwest::Client::new(),
        services: Arc::new(service_map),
        rate_limiter: Arc::new(rate_limiter),
    };

    let openapi_doc = openapi::openapi_spec();

    let app = routing::create_router(state)
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", openapi_doc),
        )
        .route("/metrics", axum::routing::get(shared::metrics::metrics_handler))
        .layer(axum::middleware::from_fn(shared::metrics::metrics_middleware))
        .layer(cors);
    let addr = config.listen_addr();
    tracing::info!("api-gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
