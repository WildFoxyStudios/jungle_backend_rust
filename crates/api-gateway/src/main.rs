mod openapi;
mod proxy;
mod rate_limit;
mod routing;
mod ws_proxy;

use shared::config::AppConfig;
use std::sync::Arc;
use http::{header, Method};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

#[tokio::main]
async fn main() {
    shared::telemetry::init("api-gateway");

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

    let service_map = routing::ServiceMap::from_env();
    let rate_limiter = rate_limit::RateLimiter::new(redis_conn);

    let state = proxy::GatewayState {
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("reqwest client"),
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
