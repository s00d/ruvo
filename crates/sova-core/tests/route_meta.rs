//! Route meta is a TypeId bag (last-wins per type) via [`RouteValue`].

use http::Method;
use sova_core::extend::RouteValue;
use sova_core::{App, Request, Response, Router};

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetaA(u8);
impl RouteValue for MetaA {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetaB(u8);
impl RouteValue for MetaB {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaxLike(u32);
impl RouteValue for MaxLike {
    fn label(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(format!("MaxLike({})", self.0))
    }
}

#[tokio::test]
async fn route_meta_multi_type_and_last_wins() {
    let mut app = App::new();
    app.get("/x", |_r: Request| async { Response::text("ok") })
        .with(MetaA(1))
        .with(MetaB(2))
        .with(MetaA(9));

    let entries = app.route_entries();
    let meta = match &entries[0] {
        sova_core::extend::RouteEntry::Http { meta, .. } => meta,
        _ => panic!("expected http"),
    };
    assert_eq!(meta.get::<MetaA>().as_deref(), Some(&MetaA(9)));
    assert_eq!(meta.get::<MetaB>().as_deref(), Some(&MetaB(2)));

    let res_meta = app
        .handle(Request::builder().method(Method::GET).path("/x").build())
        .await;
    assert_eq!(res_meta.body_bytes(), Some(b"ok".as_slice()));

    let mut app2 = App::new();
    app2.get("/y", |req: Request| async move {
        let a = req.route_meta::<MetaA>().map(|a| a.0).unwrap_or(0);
        let b = req.route_meta::<MetaB>().map(|b| b.0).unwrap_or(0);
        Response::text(format!("{a}:{b}"))
    })
    .with(MetaA(3))
    .with(MetaB(4));

    let res = app2.handle_request(Method::GET, "/y", "").await;
    assert_eq!(res.body_bytes(), Some(b"3:4".as_slice()));
}

#[tokio::test]
async fn with_inherits_app_router_route() {
    let mut api = Router::new();
    api.with(MaxLike(5));
    api.get("/a", |req: Request| async move {
        let v = req.route_meta::<MaxLike>().map(|m| m.0).unwrap_or(0);
        Response::text(v.to_string())
    });
    api.get("/b", |req: Request| async move {
        let v = req.route_meta::<MaxLike>().map(|m| m.0).unwrap_or(0);
        Response::text(v.to_string())
    })
    .with(MaxLike(50));

    let mut app = App::new();
    app.with(MaxLike(1));
    app.mount("/api", api);
    app.get("/root", |req: Request| async move {
        let v = req.route_meta::<MaxLike>().map(|m| m.0).unwrap_or(0);
        Response::text(v.to_string())
    });

    assert_eq!(
        app.handle_request(Method::GET, "/root", "")
            .await
            .body_bytes(),
        Some(b"1".as_slice())
    );
    assert_eq!(
        app.handle_request(Method::GET, "/api/a", "")
            .await
            .body_bytes(),
        Some(b"5".as_slice())
    );
    assert_eq!(
        app.handle_request(Method::GET, "/api/b", "")
            .await
            .body_bytes(),
        Some(b"50".as_slice())
    );
}

#[tokio::test]
async fn needs_fails_build_without_state() {
    use sova_core::extend::Needs;

    let mut app = App::new();
    app.get("/x", |_r: Request| async { Response::text("ok") })
        .with(Needs::<u64>::new());
    let err = match app.build() {
        Ok(_) => panic!("expected needs failure"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("needs state"), "{err}");
}

#[tokio::test]
async fn needs_ok_when_state_present() {
    use sova_core::extend::Needs;

    let mut app = App::new();
    app.state(7u64);
    app.get("/x", |_r: Request| async { Response::text("ok") })
        .with(Needs::<u64>::new());
    assert!(app.build().is_ok());
}
