//! DevTools demo — every console tab populated via **fake / in-memory** backends only.

use async_graphql::{Context, EmptySubscription, Object, Schema};
use bytes::Bytes;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sova::prelude::*;
use sova::{
    AppDispatch, AppStore, Db, DbExt, DevTools, DevToolsHub, Dispatch, Exchange, FakeBroker,
    FakeGrpc, FakeRedis, GraphQl, GraphqlContext, Grpc, GrpcExt, Html, HttpExt, I18n, I18nExt, Job,
    Json, KvStore, Locale, Mail, MailExt, Meta, OutboundHttp, Parser, Rabbit, RabbitExt, Redis,
    RedisPool, ServerArgs, SessionExt, SharedStore, StubBody, TaskBackend, Tasks,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

struct Query;

#[Object]
impl Query {
    async fn hello(&self) -> &str {
        "devtools"
    }

    async fn counter(&self, ctx: &Context<'_>) -> async_graphql::Result<i32> {
        let sova = ctx.data::<GraphqlContext>()?;
        Ok(sova
            .try_state::<Arc<std::sync::atomic::AtomicI32>>()
            .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0))
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn bump(&self, ctx: &Context<'_>) -> async_graphql::Result<i32> {
        let sova = ctx.data::<GraphqlContext>()?;
        let counter = sova.state::<Arc<std::sync::atomic::AtomicI32>>();
        Ok(counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let counter = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
    let locales_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("locales");

    let mut app = App::web()
        .site("DevTools demo")
        .public_url("http://127.0.0.1:3030")
        .into_app();

    std::env::set_var("SOVA_MAIL", "fake");
    std::env::set_var("SOVA_MAIL_FROM", "DevTools <devtools@localhost>");

    let store = AppStore::memory();
    app.state(store.clone());
    app.state(counter);
    app.state(AppDispatch::default());

    app.install(SharedStore::new(
        Arc::clone(&store.inner) as Arc<dyn KvStore>
    ));
    app.install(Mail::from_env());
    app.install(OutboundHttp::fake().get(
        "https://example.com/",
        StubBody::Bytes(Bytes::from("<html><body>fake outbound HTTP</body></html>")),
    ));
    app.install(Redis::fake(FakeRedis::new()));
    app.install(Grpc::fake(FakeGrpc::new().stub_json(
        "hello.Greeter/SayHello",
        json!({ "message": "hi from devtools gRPC fake" }),
    )));
    app.install(Rabbit::fake(FakeBroker::new()));
    app.install(Db::from_env().url("sqlite::memory:").sqlx_logging(true));
    app.install(
        Tasks::new(Arc::new(sova::tasks::Memory::new()))
            .job(Job::new("ping", |_task| async move { Ok(()) })),
    );
    app.install(
        I18n::new(
            &locales_dir,
            vec![
                Locale::new("en").with_name("English"),
                Locale::new("ru").with_name("Русский"),
            ],
        )
        .fallback("en"),
    );
    app.install(
        DevTools::new()
            .enabled(true)
            .console(true)
            .console_external(true),
    );
    app.install(
        GraphQl::server(schema)
            .path("/graphql")
            .graphiql_path("/graphiql")
            .graphiql(true)
            .subscriptions("/graphql/ws")
            .sdl_path("/graphql/sdl"),
    );

    app.get("/", home).with(Meta::page().title("Home"));
    app.get("/ping", ping);
    app.get("/mail", send_mail);
    app.get("/proxy", proxy);
    app.get("/store-demo", store_demo);
    app.get("/redis-demo", redis_demo);
    app.get("/db-demo", db_demo);
    app.get("/jobs-demo", jobs_demo);
    app.get("/events-demo", events_demo);
    app.get("/i18n-demo", i18n_demo);
    app.get("/grpc-demo", grpc_demo_page);
    app.get("/api/hello-grpc", hello_grpc);
    app.get("/rabbit-demo", rabbit_demo);

    tracing::info!("listening on http://127.0.0.1:3030 — all backends are fake/in-memory");
    app.listen(3030).await
}

async fn home(req: Request) -> Result<Html<String>> {
    req.session().set("demo", "1");
    req.session().set("role", "admin");
    let lang = req.locale();
    Ok(Html(format!(
        r#"<!doctype html>
<html lang="{lang}"><head><title>DevTools demo</title></head>
<body style="font-family:system-ui;max-width:42rem;margin:2rem auto;padding:0 1rem">
  <h1>{title}</h1>
  <p>{about}</p>
  <p>Open the <strong>DevTools</strong> bar (bottom). Every tab has a matching demo route — all fakes.</p>
  <ul>
    <li><a href="/graphiql">GraphQL</a> — /graphiql + POST /graphql</li>
    <li><a href="/ping">Timeline</a> — /ping</li>
    <li><a href="/db-demo">DB</a> — sqlite :memory:</li>
    <li><a href="/store-demo">Cache / KV</a> — AppStore memory</li>
    <li><a href="/redis-demo">Redis</a> — FakeRedis console + traces</li>
    <li><a href="/proxy">HTTP traces</a> — FakeTransport outbound</li>
    <li><a href="/">HTTP</a> — client + outbound traces</li>
    <li><a href="/api/hello-grpc">gRPC</a> — FakeGrpc client</li>
    <li><a href="/rabbit-demo">Rabbit</a> — FakeBroker</li>
    <li><a href="/mail">Mail</a> — fake mailer</li>
    <li><a href="/jobs-demo">Jobs</a> — Memory task queue</li>
    <li><a href="/">Auth</a> — session keys on this page</li>
    <li><a href="/events-demo">Events</a> — hub custom event</li>
    <li><a href="/i18n-demo">i18n</a> — locale ({lang})</li>
  </ul>
</body></html>"#,
        title = req.t("title"),
        about = req.t("nav.about"),
    )))
}

async fn ping() -> Html<&'static str> {
    Html("<!doctype html><html><body><p>pong — check Timeline</p></body></html>")
}

async fn send_mail(req: Request) -> Result<Html<&'static str>> {
    req.mail()
        .to("user@example.com")
        .subject("DevTools hello")
        .text("Hi from demo")
        .send()
        .await?;
    Ok(Html(
        "<!doctype html><html><body><p>mail sent (fake) — Mail tab</p></body></html>",
    ))
}

async fn proxy(req: Request) -> Result<Html<String>> {
    let _ = req.http().get("https://example.com/").send().await;
    Ok(Html(
        "<!doctype html><html><body><p>outbound fake HTTP — HTTP tab</p></body></html>".into(),
    ))
}

async fn store_demo(req: Request) -> Result<Html<String>> {
    let store = req.state::<AppStore>();
    let ns = store.namespaced("demo");
    ns.set("devtools:key", Bytes::from("hello-kv"), None).await;
    let val = ns.get("devtools:key").await;
    let text = val
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    Ok(Html(format!(
        "<!doctype html><html><body><p>KV ok: {text} — Cache → KV tab</p></body></html>"
    )))
}

async fn db_demo(req: Request) -> Result<Html<&'static str>> {
    req.db()
        .execute_unprepared("SELECT 1 AS one")
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(Html(
        "<!doctype html><html><body><p>sqlite memory query — DB tab</p></body></html>",
    ))
}

async fn jobs_demo(req: Request) -> Result<Html<String>> {
    let id = req
        .state::<TaskBackend>()
        .dispatch(Dispatch::new("ping").data(json!({ "from": "devtools-demo" })))
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(Html(format!(
        "<!doctype html><html><body><p>job enqueued id={id} — Jobs tab</p></body></html>"
    )))
}

async fn events_demo(req: Request) -> Result<Html<&'static str>> {
    req.state::<DevToolsHub>().emit(
        "demo.event",
        json!({ "source": "events-demo", "at": chrono_lite_now() }),
    );
    Ok(Html(
        "<!doctype html><html><body><p>custom event emitted — Events tab</p></body></html>",
    ))
}

fn chrono_lite_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn i18n_demo(req: Request) -> Result<Html<String>> {
    Ok(Html(format!(
        "<!doctype html><html lang=\"{}\"><body><p>locale={} title={}</p></body></html>",
        req.locale(),
        req.locale(),
        req.t("title"),
    )))
}

#[derive(Serialize)]
struct HelloIn {
    name: String,
}

#[derive(Deserialize)]
struct HelloOut {
    message: String,
}

async fn grpc_demo_page() -> Html<&'static str> {
    Html("<!doctype html><html><body><p>gRPC tab — <a href=\"/api/hello-grpc\">/api/hello-grpc</a></p></body></html>")
}

async fn hello_grpc(req: Request) -> Result<Json<serde_json::Value>> {
    let out: HelloOut = req
        .grpc()
        .call(
            "hello.Greeter/SayHello",
            &HelloIn {
                name: "devtools".into(),
            },
        )
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(Json(json!({ "message": out.message })))
}

async fn rabbit_demo(req: Request) -> Result<Html<String>> {
    req.rabbit()
        .declare_exchange(&Exchange::direct("devtools"))
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    req.rabbit()
        .declare_queue("devtools.queue", &sova::QueueOpts::durable())
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    req.rabbit()
        .bind("devtools.queue", "devtools", "ping")
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    req.rabbit()
        .publish(
            &Exchange::direct("devtools"),
            "ping",
            r#"{"from":"devtools-demo"}"#,
        )
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(Html(
        "<!doctype html><html><body><p>rabbit publish ok — Rabbit tab</p></body></html>".into(),
    ))
}

async fn redis_demo(req: Request) -> Result<Html<String>> {
    let pool = req.state::<RedisPool>();
    let key = "devtools:demo";
    let fake = pool
        .fake()
        .ok_or_else(|| Error::Internal("expected fake redis in demo".into()))?;

    let started = Instant::now();
    fake.set(0, key, "hello-redis")
        .map_err(|e| Error::Internal(e.0))?;
    tracing::debug!(
        target: "sova.redis",
        cmd = "set",
        key = key,
        ok = true,
        duration_ms = started.elapsed().as_secs_f64() * 1000.0,
        "sova.redis"
    );

    let started = Instant::now();
    let val = fake
        .get(0, key)
        .map_err(|e| Error::Internal(e.0))?
        .unwrap_or_default();
    tracing::debug!(
        target: "sova.redis",
        cmd = "get",
        key = key,
        hit = true,
        ok = true,
        duration_ms = started.elapsed().as_secs_f64() * 1000.0,
        "sova.redis"
    );

    Ok(Html(format!(
        "<!doctype html><html><body><p>redis ok: {val} — Redis tab</p></body></html>"
    )))
}
