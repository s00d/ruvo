//! External-API guard: plugins must compile against public surface only.
//!
//! If this fails, the core still has a privilege hole (plugin needs `pub(crate)`).

use ruvo::extend::with_leaked;
use ruvo::{App, Next, Plugin, Request, Response, Router};
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct MyCors {
    origin: String,
}

impl Plugin for MyCors {
    fn install(self, app: &mut App) {
        app.use_middleware(with_leaked(self, |cors, req, next| async move {
            if req.method == http::Method::OPTIONS {
                return Response::empty()
                    .status(204)
                    .header("access-control-allow-origin", &cors.origin);
            }
            let mut res = next(req).await;
            res = res.header("access-control-allow-origin", &cors.origin);
            res
        }));
    }
}

struct MyStatic {
    mount: String,
    dir: PathBuf,
}

impl Plugin for MyStatic {
    fn install(self, app: &mut App) {
        let dir = Arc::new(self.dir);
        let mount = self.mount;
        let wildcard = format!("{mount}/*path");

        let d = Arc::clone(&dir);
        app.get(&mount, move |_req: Request| {
            let d = Arc::clone(&d);
            async move {
                Response::file_in(d.as_path(), Path::new("index.html")).await
            }
        });

        let d = Arc::clone(&dir);
        app.get(&wildcard, move |req: Request| {
            let d = Arc::clone(&d);
            async move {
                let rel = req
                    .param("path")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("index.html"));
                Response::file_in(d.as_path(), &rel).await
            }
        });
    }
}

#[tokio::test]
async fn external_plugins_install_and_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("x.txt"), b"hi").unwrap();

    let mut app = App::new();
    app.install(MyCors {
        origin: "*".into(),
    });
    app.install(MyStatic {
        mount: "/pub".into(),
        dir: dir.path().to_path_buf(),
    });

    let res = app
        .handle_request(http::Method::GET, "/pub/x.txt", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert!(res.headers().get("access-control-allow-origin").is_some());
}

#[tokio::test]
async fn nested_router_static_sees_module_middleware() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("secret.txt"), b"nope").unwrap();

    let mut admin = Router::new();
    admin.use_middleware(|_req: Request, _next: Next| async {
        Response::text("Unauthorized").status(401)
    });
    // Same shape as built-in Static::register — only public Router::get.
    let dir = dir.path().to_path_buf();
    let d = Arc::new(dir);
    let d2 = Arc::clone(&d);
    admin.get("/files/*path", move |req: Request| {
        let d = Arc::clone(&d2);
        async move {
            let rel = req.param("path").map(PathBuf::from).unwrap_or_default();
            Response::file_in(d.as_path(), &rel).await
        }
    });

    let mut app = App::new();
    app.mount("/admin", admin);

    let res = app
        .handle_request(http::Method::GET, "/admin/files/secret.txt", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 401);
}
