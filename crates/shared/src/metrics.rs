use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use once_cell::sync::Lazy;
use prometheus::{
    Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
    Registry, TextEncoder,
};

static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

pub static HTTP_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("http_requests_total", "Total HTTP requests").namespace("Jungle");
    let counter = IntCounterVec::new(opts, &["method", "path", "status"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).ok();
    counter
});

pub static HTTP_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
    )
    .namespace("Jungle")
    .buckets(vec![
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0,
    ]);
    let hist = HistogramVec::new(opts, &["method", "path"]).unwrap();
    REGISTRY.register(Box::new(hist.clone())).ok();
    hist
});

pub static DB_QUERIES: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("db_queries_total", "Total database queries").namespace("Jungle");
    let counter = IntCounterVec::new(opts, &["query_type"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).ok();
    counter
});

pub static ACTIVE_WEBSOCKETS: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new(
        "Jungle_active_websocket_connections",
        "Active WebSocket connections",
    )
    .unwrap();
    REGISTRY.register(Box::new(gauge.clone())).ok();
    gauge
});

/// Chat messages successfully accepted by `send_message` (messaging-service).
pub static CHAT_MESSAGES_SENT: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new(
        "chat_messages_sent_total",
        "Chat messages persisted via send_message",
    )
    .namespace("Jungle");
    let counter = IntCounterVec::new(opts, &["message_type"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).ok();
    counter
});

/// WebSocket accept / disconnect events (realtime-service).
/// Reel view events that increment the post `view_count` (deduplicated per user).
pub static REELS_UNIQUE_VIEWS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    let c = IntCounter::with_opts(
        Opts::new("reels_unique_views_total", "Unique reel views recorded").namespace("Jungle"),
    )
    .unwrap();
    REGISTRY.register(Box::new(c.clone())).ok();
    c
});

/// Time spent building `GET /v1/reels` (handler body, end-to-end per request).
pub static REELS_FEED_DURATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let h = Histogram::with_opts(
        HistogramOpts::new(
            "reels_feed_duration_seconds",
            "Reels feed handler duration in seconds",
        )
        .namespace("Jungle")
        .buckets(vec![
            0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0,
        ]),
    )
    .unwrap();
    REGISTRY.register(Box::new(h.clone())).ok();
    h
});

pub static REALTIME_WS_LIFECYCLE: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new(
        "realtime_ws_lifecycle_total",
        "WebSocket connections opened or closed",
    )
    .namespace("Jungle");
    let counter = IntCounterVec::new(opts, &["event"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).ok();
    counter
});

/// GET /metrics — Prometheus scrape endpoint
pub async fn metrics_handler() -> String {
    // Touch lazy statics to ensure they are registered
    let _ = &*HTTP_REQUESTS;
    let _ = &*HTTP_DURATION;
    let _ = &*DB_QUERIES;
    let _ = &*ACTIVE_WEBSOCKETS;
    let _ = &*CHAT_MESSAGES_SENT;
    let _ = &*REALTIME_WS_LIFECYCLE;
    let _ = &*REELS_UNIQUE_VIEWS_TOTAL;
    let _ = &*REELS_FEED_DURATION_SECONDS;

    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .unwrap_or_default();
    String::from_utf8(buffer).unwrap_or_default()
}

/// Axum middleware that records request count and duration for every request.
pub async fn metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = normalize_path(req.uri().path());
    let start = std::time::Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    HTTP_REQUESTS
        .with_label_values(&[&method, &path, &status])
        .inc();
    HTTP_DURATION
        .with_label_values(&[&method, &path])
        .observe(duration);

    response
}

/// Collapse IDs in paths to `{id}` so metrics don't explode with cardinality.
fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = parts
        .iter()
        .map(|seg| {
            if seg.parse::<i64>().is_ok() || uuid::Uuid::try_parse(seg).is_ok() {
                "{id}".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect();
    let result = normalized.join("/");
    if result.is_empty() {
        "/".to_string()
    } else {
        result
    }
}
