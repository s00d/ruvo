//! Route meta is a TypeId bag (last-wins per type).

use http::Method;
use ruvo_core::{App, Request, Response};

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetaA(u8);

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetaB(u8);

#[tokio::test]
async fn route_meta_multi_type_and_last_wins() {
    let mut app = App::new();
    app.get("/x", |_r: Request| async { Response::text("ok") })
        .route_meta(MetaA(1))
        .route_meta(MetaB(2))
        .route_meta(MetaA(9));

    let entries = app.route_entries();
    let meta = match &entries[0] {
        ruvo_core::extend::RouteEntry::Http { meta, .. } => meta,
        _ => panic!("expected http"),
    };
    assert_eq!(meta.get::<MetaA>().as_deref(), Some(&MetaA(9)));
    assert_eq!(meta.get::<MetaB>().as_deref(), Some(&MetaB(2)));

    let res_meta = app
        .handle(Request::builder().method(Method::GET).path("/x").build())
        .await;
    assert_eq!(res_meta.body_bytes(), Some(b"ok".as_slice()));

    // MatchedMeta injection for handlers:
    let mut app2 = App::new();
    app2.get("/y", |req: Request| async move {
        let a = req.route_meta::<MetaA>().map(|a| a.0).unwrap_or(0);
        let b = req.route_meta::<MetaB>().map(|b| b.0).unwrap_or(0);
        Response::text(format!("{a}:{b}"))
    })
    .route_meta(MetaA(3))
    .route_meta(MetaB(4));

    let res = app2.handle_request(Method::GET, "/y", "").await;
    assert_eq!(res.body_bytes(), Some(b"3:4".as_slice()));
}
