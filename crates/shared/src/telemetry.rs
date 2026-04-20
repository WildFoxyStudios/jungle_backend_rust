//! OpenTelemetry distributed tracing wiring.
//!
//! - Initializes an OTLP gRPC exporter if `OTEL_EXPORTER_OTLP_ENDPOINT` is set.
//! - Falls back to a no-op (tracing only goes to stdout via `tracing-subscriber`).
//! - Adds a `tracing-opentelemetry` layer so every `#[tracing::instrument]` or
//!   `tracing::info_span!` becomes an OTLP span.
//!
//! Call [`init`] from every microservice's `main()` **before** constructing
//! the `tracing_subscriber::registry()`.

use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{self as sdktrace, RandomIdGenerator, Sampler},
    Resource,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the global tracing subscriber and (if configured) the OpenTelemetry
/// exporter. Reads:
///
/// - `OTEL_EXPORTER_OTLP_ENDPOINT` — e.g. `http://otel-collector:4317`
/// - `OTEL_TRACES_SAMPLER_ARG`     — sample ratio 0.0..1.0 (default 1.0)
/// - `RUST_LOG`                    — tracing filter (default `info,sqlx=warn,tower_http=debug`)
pub fn init(service_name: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,tower_http=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty());

    let otel_layer = endpoint.as_deref().and_then(|ep| {
        match build_tracer(service_name, ep) {
            Ok(tracer) => Some(tracing_opentelemetry::layer().with_tracer(tracer)),
            Err(e) => {
                eprintln!("OTLP init failed: {e} (continuing without distributed tracing)");
                None
            }
        }
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    if endpoint.is_some() {
        tracing::info!(service = service_name, "OpenTelemetry tracing enabled");
    }
}

fn build_tracer(service_name: &str, endpoint: &str) -> Result<sdktrace::Tracer, String> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| format!("OTLP exporter: {e}"))?;

    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    let provider = sdktrace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_sampler(Sampler::TraceIdRatioBased(sample_rate()))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(service_name.to_string());
    global::set_tracer_provider(provider);

    Ok(tracer)
}

fn sample_rate() -> f64 {
    std::env::var("OTEL_TRACES_SAMPLER_ARG")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0)
}

/// Shut down the tracer provider cleanly. Call this in a graceful-shutdown hook
/// so in-flight spans are flushed before exit.
pub fn shutdown() {
    global::shutdown_tracer_provider();
}
