//! Optional OpenTelemetry OTLP tracing (feature `otel`).

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::sync::OnceLock;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

static PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

/// Install OTLP exporter + tracing-opentelemetry layer from env.
///
/// Env: `OTEL_EXPORTER_OTLP_ENDPOINT` (required to enable), `OTEL_SERVICE_NAME` (default `sova`).
pub fn install_from_env() -> Result<(), String> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .map_err(|_| "OTEL_EXPORTER_OTLP_ENDPOINT not set — skipping OTLP".to_string())?;
    if endpoint.is_empty() {
        return Err("OTEL_EXPORTER_OTLP_ENDPOINT empty".into());
    }
    let service = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "sova".into());

    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| e.to_string())?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name(service).build())
        .build();

    let tracer = provider.tracer("sova");
    let _ = PROVIDER.set(provider);
    opentelemetry::global::set_tracer_provider(PROVIDER.get().expect("provider just set").clone());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    // Best-effort: if a subscriber is already set, layering fails — warn.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(otel_layer)
        .try_init();

    Ok(())
}
