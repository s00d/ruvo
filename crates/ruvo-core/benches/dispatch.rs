use criterion::{criterion_group, criterion_main, Criterion};
use http::Method;
use ruvo_core::{App, Request, Response};
use std::hint::black_box;

fn build_app() -> App {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });
    app.get("/users/:id", |r: Request| async move {
        Response::text(r.param("id").unwrap_or("").to_string())
    });
    app.use_middleware(|req: Request, next: ruvo_core::Next| async move { next(req).await });
    app
}

fn bench_build(c: &mut Criterion) {
    c.bench_function("app_build", |b| {
        b.iter(|| {
            let app = build_app();
            black_box(app.build().unwrap())
        })
    });
}

fn bench_handle(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let app = build_app();
    let server = app.build().unwrap();

    c.bench_function("handle_root", |b| {
        b.to_async(&rt).iter(|| async {
            let res = server
                .handle_request(Method::GET, "/", "")
                .await;
            black_box(res.status_code())
        })
    });

    c.bench_function("handle_param", |b| {
        b.to_async(&rt).iter(|| async {
            let res = server
                .handle_request(Method::GET, "/users/42", "")
                .await;
            black_box(res.status_code())
        })
    });

    c.bench_function("handle_via_app", |b| {
        b.to_async(&rt).iter(|| async {
            let res = app.handle_request(Method::GET, "/", "").await;
            black_box(res.status_code())
        })
    });
}

criterion_group!(benches, bench_build, bench_handle);
criterion_main!(benches);
