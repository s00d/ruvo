//! HTTP RED metrics (Prometheus) + optional OpenTelemetry / Elasticsearch logs for Sova.
//!
//! ```ignore
//! app.use_middleware(request_id());
//! app.install(
//!     Observability::new()
//!         .with_elasticsearch() // ELASTICSEARCH_URL → bulk ship tracing logs
//! );
//! app.use_middleware(logger());
//! ```
//!
//! Declarative toggles also work via `[observability]` in `sova.toml`
//! (`metrics_path`, `otel`, `elasticsearch`) when not set on the builder.

#[cfg(feature = "prometheus")]
mod prometheus;
#[cfg(feature = "otel")]
mod otel;
#[cfg(feature = "elasticsearch")]
mod elasticsearch;

#[cfg(feature = "elasticsearch")]
pub use elasticsearch::ElasticsearchLog;

use sova_core::extend::{named, MwEntry};
use sova_core::{with_state, App, MatchedRouteCapture, Plugin, Request, Response};
use std::time::Instant;

/// Observability plugin: Prometheus scrape + HTTP RED middleware (+ optional sinks).
pub struct Observability {
    metrics_path: String,
    metrics_path_explicit: bool,
    #[cfg(feature = "prometheus")]
    handle: Option<prometheus::Handle>,
    #[cfg(feature = "otel")]
    otel: bool,
    #[cfg(feature = "otel")]
    otel_explicit: bool,
    #[cfg(feature = "elasticsearch")]
    elasticsearch: bool,
    #[cfg(feature = "elasticsearch")]
    elasticsearch_explicit: bool,
}

impl Observability {
    /// Prometheus `/metrics` + request middleware (feature `prometheus`).
    pub fn new() -> Self {
        Self {
            metrics_path: "/metrics".into(),
            metrics_path_explicit: false,
            #[cfg(feature = "prometheus")]
            handle: Some(prometheus::install_recorder()),
            #[cfg(feature = "otel")]
            otel: false,
            #[cfg(feature = "otel")]
            otel_explicit: false,
            #[cfg(feature = "elasticsearch")]
            elasticsearch: false,
            #[cfg(feature = "elasticsearch")]
            elasticsearch_explicit: false,
        }
    }

    pub fn metrics_path(mut self, path: impl Into<String>) -> Self {
        self.metrics_path = path.into();
        self.metrics_path_explicit = true;
        self
    }

    /// Install OTLP tracing from env (`OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`).
    #[cfg(feature = "otel")]
    pub fn with_otel(mut self) -> Self {
        self.otel = true;
        self.otel_explicit = true;
        self
    }

    /// Ship tracing logs to Elasticsearch (`ELASTICSEARCH_URL`, see [`ElasticsearchLog`]).
    #[cfg(feature = "elasticsearch")]
    pub fn with_elasticsearch(mut self) -> Self {
        self.elasticsearch = true;
        self.elasticsearch_explicit = true;
        self
    }

    /// Alias for [`Self::new`].
    pub fn prometheus() -> Self {
        Self::new()
    }

    fn apply_config(&mut self, app: &App) {
        let Some(doc) = app.config_doc() else {
            return;
        };
        let Some(section) = doc.section("observability") else {
            return;
        };
        if !self.metrics_path_explicit {
            if let Some(p) = section.get("metrics_path").and_then(|v| v.as_str()) {
                self.metrics_path = p.to_string();
            }
        }
        #[cfg(feature = "otel")]
        if !self.otel_explicit {
            if let Some(v) = section.get("otel").and_then(|v| v.as_bool()) {
                self.otel = v;
            }
        }
        #[cfg(feature = "elasticsearch")]
        if !self.elasticsearch_explicit {
            if let Some(v) = section.get("elasticsearch").and_then(|v| v.as_bool()) {
                self.elasticsearch = v;
            }
        }
    }
}

impl Default for Observability {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Observability {
    fn id(&self) -> &'static str {
        "observability"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Observability")
            .description("HTTP metrics, OpenTelemetry, Elasticsearch log shipping")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        self.apply_config(app);

        #[cfg(feature = "otel")]
        if self.otel {
            match otel::install_from_env() {
                Ok(()) => tracing::info!("otel OTLP exporter installed"),
                Err(e) => tracing::warn!(error = %e, "otel install skipped"),
            }
        }

        #[cfg(feature = "elasticsearch")]
        if self.elasticsearch {
            match elasticsearch::install_from_env() {
                Ok(()) => tracing::info!("elasticsearch log sink installed"),
                Err(e) => tracing::warn!(error = %e, "elasticsearch log sink skipped"),
            }
        }

        #[cfg(feature = "prometheus")]
        if let Some(handle) = self.handle.clone() {
            let path = self.metrics_path.clone();
            let handle_route = handle.clone();
            let mount = path.clone();
            app.get(&mount, move |_req: Request| {
                let handle = handle_route.clone();
                async move {
                    handle.run_upkeep();
                    let body = handle.render();
                    Ok::<_, sova_core::Error>(
                        Response::text(body).header("content-type", "text/plain; version=0.0.4"),
                    )
                }
            });

            app.use_middleware(metrics_middleware(path));
        }
    }
}

fn is_probe(path: &str, metrics_path: &str) -> bool {
    path == metrics_path || path == "/healthz" || path == "/ready"
}

#[cfg(feature = "prometheus")]
fn metrics_middleware(metrics_path: String) -> MwEntry {
    named(
        "observability-metrics",
        with_state(metrics_path, |metrics_path, mut req, next| async move {
            let capture = MatchedRouteCapture::new();
            req.set(capture.clone());
            let method = req.method.as_str().to_string();
            let path = req.path.clone();
            let probe = is_probe(&path, metrics_path.as_str());
            let start = Instant::now();
            if !probe {
                metrics::gauge!("http_requests_in_flight").increment(1.0);
            }
            let res = next(req).await;
            if !probe {
                metrics::gauge!("http_requests_in_flight").decrement(1.0);
                let status = res.status_code().as_u16();
                let route = capture
                    .get()
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| path.clone());
                let elapsed = start.elapsed().as_secs_f64();
                metrics::counter!(
                    "http_requests_total",
                    "method" => method.clone(),
                    "route" => route.clone(),
                    "status" => status.to_string()
                )
                .increment(1);
                metrics::histogram!(
                    "http_request_duration_seconds",
                    "method" => method,
                    "route" => route
                )
                .record(elapsed);
            }
            res
        }),
    )
}
