//! Logout other / all sessions via SessionStore.

use sova_core::{App, Html, Request, TestClient};
use sova_session::{memory_sessions, SessionExt, SessionLayer, SessionStoreHandle};
use sova_store::{namespace, MemoryStore};
use std::sync::Arc;

fn app_with_store(store: Arc<dyn sova_store::KvStore>) -> App {
    let mut app = App::new();
    app.install(SessionLayer::new(store).cookie_name("sid"));
    app.get("/login", |req: Request| async move {
        req.session().bind_user("42");
        req.session().set("mark", "yes");
        Html("ok".to_string())
    });
    app.get("/mark", |req: Request| async move {
        Html(req.session().get("mark").unwrap_or_else(|| "none".into()))
    });
    app.post("/logout-others", |req: Request| async move {
        let n = req.logout_other_sessions().await.unwrap();
        Html(n.to_string())
    });
    app.post("/logout-all", |mut req: Request| async move {
        let n = req.logout_all_sessions().await.unwrap();
        Html(n.to_string())
    });
    app
}

#[tokio::test]
async fn logout_other_sessions_keeps_current() {
    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "sess"));
    let a = TestClient::tracked(app_with_store(Arc::clone(&store) as _)).await.unwrap();
    let b = TestClient::tracked(app_with_store(store)).await.unwrap();

    a.get("/login").await;
    b.get("/login").await;
    assert_eq!(a.get("/mark").await.body_bytes(), Some(b"yes".as_slice()));
    assert_eq!(b.get("/mark").await.body_bytes(), Some(b"yes".as_slice()));

    let n = a.post("/logout-others").await;
    assert_eq!(n.body_bytes(), Some(b"1".as_slice()));

    assert_eq!(a.get("/mark").await.body_bytes(), Some(b"yes".as_slice()));
    assert_eq!(b.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
}

#[tokio::test]
async fn logout_all_sessions_clears_current_too() {
    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "sess"));
    let a = TestClient::tracked(app_with_store(Arc::clone(&store) as _)).await.unwrap();
    let b = TestClient::tracked(app_with_store(store)).await.unwrap();

    a.get("/login").await;
    b.get("/login").await;

    let n = a.post("/logout-all").await;
    assert_eq!(n.body_bytes(), Some(b"2".as_slice()));

    assert_eq!(a.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
    assert_eq!(b.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
}

#[tokio::test]
async fn memory_sessions_exports_store_handle() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        assert!(req.try_state::<SessionStoreHandle>().is_some());
        Html("ok".to_string())
    });
    let c = TestClient::tracked(app).await.unwrap();
    assert_eq!(c.get("/").await.status_code().as_u16(), 200);
}

#[cfg(feature = "sql")]
mod sql_logout {
    use super::*;
    use sova_db::DbPool;
    use sova_session::SqlSessionStore;
    use sea_orm::Database;

    fn app_sql(store: Arc<SqlSessionStore>) -> App {
        let mut app = App::new();
        app.install(SessionLayer::from_store(store).cookie_name("sid"));
        app.get("/login", |req: Request| async move {
            req.session().bind_user("7");
            req.session().set("mark", "yes");
            Html("ok".to_string())
        });
        app.get("/mark", |req: Request| async move {
            Html(req.session().get("mark").unwrap_or_else(|| "none".into()))
        });
        app.post("/logout-others", |req: Request| async move {
            let n = req.logout_other_sessions().await.unwrap();
            Html(n.to_string())
        });
        app.post("/logout-all", |mut req: Request| async move {
            let n = req.logout_all_sessions().await.unwrap();
            Html(n.to_string())
        });
        app
    }

    async fn shared_sql() -> Arc<SqlSessionStore> {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        let pool = DbPool::new();
        pool.set(conn).await;
        let store = Arc::new(SqlSessionStore::from_db_pool(&pool));
        store.ensure_schema().await.unwrap();
        store
    }

    #[tokio::test]
    async fn sql_logout_other_sessions() {
        let store = shared_sql().await;
        let a = TestClient::tracked(app_sql(Arc::clone(&store))).await.unwrap();
        let b = TestClient::tracked(app_sql(store)).await.unwrap();

        a.get("/login").await;
        b.get("/login").await;

        assert_eq!(a.post("/logout-others").await.body_bytes(), Some(b"1".as_slice()));
        assert_eq!(a.get("/mark").await.body_bytes(), Some(b"yes".as_slice()));
        assert_eq!(b.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
    }

    #[tokio::test]
    async fn sql_logout_all_sessions() {
        let store = shared_sql().await;
        let a = TestClient::tracked(app_sql(Arc::clone(&store))).await.unwrap();
        let b = TestClient::tracked(app_sql(store)).await.unwrap();

        a.get("/login").await;
        b.get("/login").await;

        assert_eq!(a.post("/logout-all").await.body_bytes(), Some(b"2".as_slice()));
        assert_eq!(a.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
        assert_eq!(b.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
    }
}

#[cfg(feature = "redis")]
mod redis_logout {
    use super::*;
    use sova_redis::RedisPool;
    use sova_session::RedisSessionStore;

    fn app_redis(store: Arc<RedisSessionStore>) -> App {
        let mut app = App::new();
        app.install(SessionLayer::from_store(store).cookie_name("sid"));
        app.get("/login", |req: Request| async move {
            req.session().bind_user("9");
            req.session().set("mark", "yes");
            Html("ok".to_string())
        });
        app.get("/mark", |req: Request| async move {
            Html(req.session().get("mark").unwrap_or_else(|| "none".into()))
        });
        app.post("/logout-others", |req: Request| async move {
            let n = req.logout_other_sessions().await.unwrap();
            Html(n.to_string())
        });
        app.post("/logout-all", |mut req: Request| async move {
            let n = req.logout_all_sessions().await.unwrap();
            Html(n.to_string())
        });
        app
    }

    async fn shared_redis() -> Option<Arc<RedisSessionStore>> {
        let url = std::env::var("REDIS_URL").ok()?;
        let conn = RedisPool::connect(&url).await.ok()?;
        let pool = RedisPool::new();
        pool.set(conn).await;
        // Unique prefix so parallel CI runs do not clash.
        let prefix = format!(
            "sova_test_sess_{}:",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        Some(Arc::new(RedisSessionStore::with_prefix(&pool, prefix)))
    }

    #[tokio::test]
    async fn redis_logout_other_sessions() {
        let Some(store) = shared_redis().await else {
            eprintln!("skip redis_logout_other_sessions: set REDIS_URL");
            return;
        };
        let a = TestClient::tracked(app_redis(Arc::clone(&store))).await.unwrap();
        let b = TestClient::tracked(app_redis(store)).await.unwrap();

        a.get("/login").await;
        b.get("/login").await;

        assert_eq!(a.post("/logout-others").await.body_bytes(), Some(b"1".as_slice()));
        assert_eq!(a.get("/mark").await.body_bytes(), Some(b"yes".as_slice()));
        assert_eq!(b.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
    }

    #[tokio::test]
    async fn redis_logout_all_sessions() {
        let Some(store) = shared_redis().await else {
            eprintln!("skip redis_logout_all_sessions: set REDIS_URL");
            return;
        };
        let a = TestClient::tracked(app_redis(Arc::clone(&store))).await.unwrap();
        let b = TestClient::tracked(app_redis(store)).await.unwrap();

        a.get("/login").await;
        b.get("/login").await;

        assert_eq!(a.post("/logout-all").await.body_bytes(), Some(b"2".as_slice()));
        assert_eq!(a.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
        assert_eq!(b.get("/mark").await.body_bytes(), Some(b"none".as_slice()));
    }
}
