//! Release microbenches for the production request path (`Server::handle`).
//!
//! Run: `cargo bench -p sova-core --bench dispatch`
//! Quick smoke: `cargo bench -p sova-core --bench dispatch -- --quick`

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use http::Method;
use sova_core::{App, Json, Next, Request, Response, Router};
use std::hint::black_box;
use std::time::Duration;

fn build_minimal() -> App {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });
    app.get("/users/:id", |r: Request| async move {
        Response::text(r.param("id").unwrap_or("").to_string())
    });
    app
}

/// Realistic API-shaped app: path params, JSON POST, query, layered middleware,
/// scoped 404 catcher — mirrors production route density without I/O plugins.
fn build_realistic() -> App {
    let mut api = Router::new();
    api.get("/health", |_r: Request| async {
        Json(serde_json::json!({ "ok": true }))
    });
    api.get("/users/:id", |r: Request| async move {
        let id = r.param("id").unwrap_or("").to_string();
        Json(serde_json::json!({ "id": id }))
    });
    api.post("/echo", |mut r: Request| async move {
        let body = r
            .json::<serde_json::Value>()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        Json(body)
    });
    api.catch(404, |_r: Request| async {
        Json(serde_json::json!({ "error": "not_found" }))
    });

    let mut app = App::new();
    // Cheap identity layers (common in prod stacks before real plugins).
    for _ in 0..3 {
        app.use_middleware(|req: Request, next: Next| async move { next(req).await });
    }
    app.get("/", |_r: Request| async { Response::text("home") });
    app.get("/search", |r: Request| async move {
        let q = r.query("q").unwrap_or("").to_string();
        Response::text(format!("q={q}"))
    });
    app.mount("/api", api);
    app
}

fn bench_build(c: &mut Criterion) {
    let mut g = c.benchmark_group("build");
    g.bench_function("minimal", |b| {
        b.iter(|| {
            let app = build_minimal();
            black_box(app.build().unwrap())
        })
    });
    g.bench_function("realistic", |b| {
        b.iter(|| {
            let app = build_realistic();
            black_box(app.build().unwrap())
        })
    });
    g.finish();
}

fn bench_handle_minimal(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let server = build_minimal().build().unwrap();

    let mut g = c.benchmark_group("handle_minimal");
    g.throughput(Throughput::Elements(1));
    g.bench_function("root", |b| {
        b.to_async(&rt).iter(|| async {
            let res = server.handle_request(Method::GET, "/", "").await;
            black_box(res.status_code())
        })
    });
    g.bench_function("param", |b| {
        b.to_async(&rt).iter(|| async {
            let res = server
                .handle_request(Method::GET, "/users/42", "")
                .await;
            black_box(res.status_code())
        })
    });
    g.finish();
}

fn bench_handle_realistic(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let server = build_realistic().build().unwrap();

    let mut g = c.benchmark_group("handle_realistic");
    g.sample_size(80);
    g.measurement_time(Duration::from_secs(12));
    g.throughput(Throughput::Elements(1));

    let cases = [
        ("home", Method::GET, "/", ""),
        ("api_health", Method::GET, "/api/health", ""),
        ("api_user", Method::GET, "/api/users/42", ""),
        ("search_query", Method::GET, "/search?q=sova", ""),
        ("api_404", Method::GET, "/api/missing", ""),
        ("echo_json", Method::POST, "/api/echo", r#"{"hello":"world"}"#),
    ];

    for (name, method, path, body) in cases {
        g.bench_with_input(BenchmarkId::from_parameter(name), &body, |b, body| {
            b.to_async(&rt).iter(|| async {
                let res = server.handle_request(method.clone(), path, body).await;
                black_box(res.status_code())
            })
        });
    }
    g.finish();
}

fn bench_burst(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let server = std::sync::Arc::new(build_realistic().build().unwrap());

    let mut g = c.benchmark_group("burst");
    g.sample_size(40);
    g.measurement_time(Duration::from_secs(15));
    g.throughput(Throughput::Elements(256));
    g.bench_function("256_mixed", |b| {
        let server = std::sync::Arc::clone(&server);
        b.to_async(&rt).iter(|| {
            let server = std::sync::Arc::clone(&server);
            async move {
                let mut futs = Vec::with_capacity(256);
                for i in 0..256 {
                    let s = std::sync::Arc::clone(&server);
                    futs.push(async move {
                        let path = match i % 5 {
                            0 => "/",
                            1 => "/api/health",
                            2 => "/api/users/7",
                            3 => "/search?q=x",
                            _ => "/api/missing",
                        };
                        s.handle_request(Method::GET, path, "").await.status_code()
                    });
                }
                let results = futures_util::future::join_all(futs).await;
                black_box(results.len())
            }
        })
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_build,
    bench_handle_minimal,
    bench_handle_realistic,
    bench_burst
);
criterion_main!(benches);
