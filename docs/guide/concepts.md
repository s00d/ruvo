# Concepts

![Concepts](/banners/concepts.svg)

Sova is a small Express-like HTTP framework: `sova-core` owns the request path; plugins add optional middleware and helpers.

## Request path

```text
accept
  → server/conn (semaphore, JoinSet, hyper auto HTTP/1.1+HTTP/2 + with_upgrades)
  → to_sova_request (+ optional OnUpgrade)
  → CompiledRouter::dispatch
  → root middleware (onion)  // request_id → Observability → logger → …
  → matchit route match (+ MatchedRoute / MatchedRouteCapture)
  → route / mount middleware
  → handler
  → IntoResponse
  → hyper response body
```

`App::build()` compiles routes once into a cheap-to-clone `Server`. Prefer `Server::handle` in tests so the matcher is not rebuilt per request.

## Routing helpers

`Router::head` / `Router::options` register explicit handlers. If no HEAD is registered, GET still serves HEAD with the body stripped. If no OPTIONS is registered, the router answers with `204` and an `Allow` header.

## Typed extractors

Import from `sova::extract` (`Path`, `Query`, `Json`, `Form`, `State`, `Extension`). Classic `async fn(req: Request)` handlers remain supported. See [Plugin SDK → Extractors](/api/plugin-sdk/extractors).

## Custom middleware (onion)

Signature: `(Request, Next) -> Response` where `Next` is `FnOnce(Request) -> Future<Response>`.

```rust
app.use_middleware(|req: Request, next: Next| async move {
    // before
    let mut res = next(req).await;
    // after — mutate response headers/body wrappers
    res = res.header("x-demo", "1");
    res
});
```

| Scope | API |
|-------|-----|
| Whole app | `app.use_middleware(...)` |
| Mount / group | `router.use_middleware(...)` then `app.mount("/x", router)` |
| Explain label | `sova::extend::named("auth", mw)` |
| Shared state | `with_state(S, …)` / `extend::with_leaked` |

Short-circuit by **not** calling `next` (return `401` / redirect). Pass data to handlers with `req.set(T)` / `req.get::<T>()`.

Stateful helpers: `with_state(S, |arc, req, next| …)` and `extend::with_leaked` for `'static` plugin config. Full walkthrough: [Getting started → Custom middleware](/guide/getting-started#custom-middleware).

## Lifecycle

```text
start: compile → on_startup → BackgroundServices → accept
stop:  stop accept → drain connections → stop services → on_shutdown
```

`app.run()` parses CLI commands (`check`, `routes`, `openapi --out`, `tasks`, `migrate` / `seed`, …) or starts the server. CLI mode runs startup/shutdown and skips accept.

### Health probes

| Endpoint | Role | Behavior |
|----------|------|----------|
| `GET /healthz` | liveness | process up; no plugin checks |
| `GET /ready` | readiness | `CheckKind::Ready`; else `503` |

`register_check` → Ready. `register_audit` → Audit (CLI `check` only).

## Extension model

```rust
pub trait Plugin {
    fn id(&self) -> &'static str;
    fn requires(&self) -> &'static [&'static str] { &[] }
    fn meta(&self) -> PluginMeta { /* … */ }
    fn install(self, app: &mut App);
}

app.install(Cors::new());
app.install(|app| { app.get("/x", handler); });
```

App-author patterns (routes, validate, auth): [Getting started](/guide/getting-started) and [Examples](/examples). Plugin authors: [Plugin SDK](/api/plugin-sdk).

## Public surface

| Surface | Audience |
|---------|----------|
| crate root | Apps: `App`, `Request`, `Response`, `Plugin`, … |
| `extend` | Plugins / advanced: handlers, middleware traits, bodies |

## Ownership

- **sova-core** — App/Router/Server, dispatch, listen/drain
- **plugins** — optional features; core does not depend on plugins
- **KvStore** is not in core — wire via `app.state(...)`

## Share (`Cell` / `Slot`)

Cross-task sharing without inventing channels for every demo: two small handles in `sova-core` (re-exported as `sova::Cell` / `sova::Slot`).

| Type | When | API |
|------|------|-----|
| **`Cell<T: Clone>`** | Counters, flags, config snapshots | `get` / `set` / `update` / `changed().await` |
| **`Slot<T>`** | **One owned value** — TCP socket, stream, file handle (anything **not** `Clone`) | `put` / `try_take` / `take().await` |

Both are cheap [`Clone`] handles (`Arc` inside). Wire once on the app, read from handlers via `req.state::<Cell<_>>()` / `req.state::<Slot<_>>()`:

```rust
use sova::{App, BackgroundService, Cell, Request, Slot, Shutdown};
use tokio::net::{TcpListener, TcpStream};

let inbox = Slot::<TcpStream>::new();
let handed = Cell::new(0u64);
app.state(inbox.clone());
app.state(handed.clone());

// BackgroundService accepts raw TCP and hands the stream to HTTP:
// inbox.put(stream);

app.post("/grab", |req: Request| async move {
    let inbox = req.state::<Slot<TcpStream>>();
    let stream = inbox.take().await; // waits until BackgroundService put()s a socket
    // read/write `stream`, build Response
    Ok(sova::Json(serde_json::json!({ "ok": true })))
});
```

**Why `Slot`?** REST handlers and `BackgroundService` tasks do not share stack frames. You cannot return a live `TcpStream` from a background accept loop through JSON. `Slot` is a single-item mailbox: the service **`put`s** ownership, the handler **`take`s** it — no REST round-trip, no `Arc<Mutex<TcpStream>>` for one consumer.

**`Slot` semantics:** at most one unread value; a new `put` **replaces** (drops) the previous one. Not a queue — use `tokio::sync::mpsc` if you need buffering.

**`Cell` semantics:** last value wins; `changed().await` blocks until the next `set`/`update` (handy for “wait for background flag” without polling).

Typical uses:

- Raw TCP / UDP socket accepted in a service → HTTP handler upgrades or proxies it
- QUIC datagram pipe, custom protocol bridge
- Shared live counters visible on a status route (`rest_api`, `api_preset` use `Cell` for in-memory lists)

Full runnable demo:

```bash
cargo run -p share_demo
# terminal 2:  nc 127.0.0.1 9090
# terminal 3:  curl -X POST http://127.0.0.1:3020/grab
```

See [`examples/misc/share_demo`](https://github.com/s00d/sova/tree/master/examples/misc/share_demo). For **many clients** and broadcast chat, prefer [`ws`](/plugins/ws) or [`sse`](/plugins/sse) instead of rolling your own fan-out.

## Streaming responses

Core supports buffered and **streaming** bodies without a plugin:

| API | Use |
|-----|-----|
| `Response::sse(stream)` | Server-Sent Events (`text/event-stream`) |
| `Response::stream(body)` | Arbitrary chunked body |
| `Response::from_reader_stream(s)` | File/IO stream → HTTP |
| `Response::file(path).await` | Safe local file (path traversal blocked) |
| `Response::file_in(dir, rel).await` | File under a directory root |
| `Response::download(path).await` | File + `Content-Disposition: attachment` |
| `.attachment(filename)` | Force download name on any response |

Minimal SSE (no `sova-sse` plugin):

```rust
use futures_util::stream;
use sova::Response;

Response::sse(stream::iter(vec![
    Ok::<_, std::convert::Infallible>("hello\n".into()),
    Ok("world\n".into()),
]))
```

For named channels, replay, and keepalive comments use the [`sse`](/plugins/sse) plugin instead.

## Request bodies & uploads

| API | Notes |
|-----|-------|
| `req.bytes().await` | Buffer whole body (respects `body_limit`) |
| `req.json::<T>().await` | JSON decode |
| `req.form::<T>().await` | `application/x-www-form-urlencoded` |
| `req.into_body_stream()` | Take body as `HttpBody` stream (once) |
| `Upload` / `UploadRules` | Multipart fields + size/MIME limits ([getting started → uploads](/guide/getting-started#forms-flash-uploads)) |
| `FormData` | Low-level multipart access |

`Request::builder()` builds synthetic requests for tests and in-process dispatch.

## HTTP upgrades (WebSocket, …)

On upgrade requests the core stashes Hyper’s `OnUpgrade` on the request:

```rust
if let Some(up) = req.on_upgrade()? {
    let (io, permit) = up.upgrade().await?;
    // keep `permit` alive for the connection lifetime
    tokio::spawn(async move { /* protocol on `io` */ });
}
```

- `App::max_upgraded_connections(n)` — cap concurrent upgraded connections; budget exhausted → **503** + `Retry-After`.
- Prefer normal routes + `req.on_upgrade()` over `Router::raw` (escape hatch for pre-parse access).

Plugins [`ws`](/plugins/ws) wrap this; core owns the budget and permit.

## In-process events

Typed sync event bus (no Redis required for same-process fan-out):

```rust
use sova::{Event, EventBus};

#[derive(Clone)]
struct OrderPaid { id: u64 }

impl Event for OrderPaid {
    fn name(&self) -> &'static str { "order.paid" }
}

let bus = app.events();
bus.listen::<OrderPaid, _>(|e| tracing::info!(id = e.id, "paid"));
// later, in a handler:
bus.dispatch(OrderPaid { id: 42 });
```

Listeners run **synchronously** in the dispatching task — spawn from the listener for async work. Plugins (mail, auth, tasks, …) publish their own event types; see [Plugin SDK → Events](/api/plugin-sdk/events).

## Errors & content negotiation

| Preset / context | Error shape |
|------------------|-------------|
| `App::api()` | `application/problem+json` ([RFC 9457](https://www.rfc-editor.org/rfc/rfc9457)) |
| `App::web()` | Negotiates via `Accept`: HTML error page → Problem Details → plain text |
| Router 404/405 | Same Accept-aware builder |

Helpers: `problem_response`, `error_to_problem`, `problem_with_errors` (validation arrays). Catchers and `Error::custom(status, msg)` integrate through `IntoResponse`.

Web HTML errors: `html_error_page`, `negotiate_error_format`, `error_response_for_accept` (also used internally for 404/405).

## Route metadata & limits

Attach per-route or router-default values with `.with(...)`:

```rust
use sova::extend::{MaxBody, RequestTimeout};

router
    .post("/upload", handler)
    .with(MaxBody::mib(32))
    .with(RequestTimeout::from_secs(120));
```

| Type | Effect |
|------|--------|
| `MaxBody` | Override body size for this route |
| `RequestTimeout` | Per-route handler budget |
| `Deadline` | Absolute instant (advanced) |

After match, handlers read overlay metadata via `req.route_meta::<T>()` or capture types `MatchedRoute` / `MatchedRouteCapture` (pattern + params). Plugins use `RouteValue` + `MetaMap` for OpenAPI, VLD, meta tags — see [Plugin SDK → Routes](/api/plugin-sdk/routes).

Human sizes/durations in toml and builders: `parse_bytes("10mb")`, `parse_duration("7d")` (`sova_core::extend`).

## In-process dispatch

`AppDispatch` replays HTTP through the compiled router **inside** the running process (no TCP):

```rust
app.state(AppDispatch::default());
// after listen — DevTools console, integration tests:
// dispatch.try_dispatch(req) → same stack as a real hit
```

Required for DevTools HTTP replay ([devtools guide](/guide/devtools)). `None` before the server starts.

## Checks, audits & custom CLI

| Hook | When it runs |
|------|----------------|
| `register_check(name, f)` | `GET /ready`, `cargo run -- check` (Ready) |
| `register_audit(name, f)` | `cargo run -- check` only (Audit) |
| `register_cli(name, f)` | Custom subcommands (`migrate`, `seed`, …) |

`app.with_probes()` registers `/healthz` + `/ready`. `CheckKind::Ready` vs `Audit` filters which hooks run.

## Request context helpers

| Symbol | Role |
|--------|------|
| `ClientAddr` | Peer socket (`req.get::<ClientAddr>()`) |
| `RateLimitIdentity` | Stable key for rate-limit plugins |
| `RequestId` / `current_request_id()` | Correlation id (middleware + tracing) |
| `referer_or(req, fallback)` | Safe back-link for redirects |
| `req.scheme()` / `host()` / `is_secure()` | URL parts (`trust_proxy` affects scheme) |
| `req.deadline_remaining()` | Time left under route/app timeout |

Skip noisy paths in access logs: `logger_skip_path`, `logger_skip_paths`, `logger_should_skip`.

## Middleware building blocks

Beyond hand-written `(req, next)`:

| Helper | Role |
|--------|------|
| `before(name, f)` | Run before handler only |
| `after(name, f)` | Mutate response after handler |
| `around(name, before, after)` | Both |
| `map_html(name, f)` | Transform buffered HTML responses |
| `named(name, mw)` | Label for `routes` / `explain()` output |

HTML injection utilities (`inject_head`, `inject_body_end`, `HtmlInject`, …) live in `sova::html` / `extend` — used by [meta](/plugins/meta) and [devtools](/plugins/devtools).

## Log event hooks

Plugins and DevTools can observe every `tracing` event:

```rust
sova::add_log_event_hook(Arc::new(|rec: LogRecord| {
    // rec.target, rec.level, rec.fields, …
}));
```

`set_log_event_hook` replaces all hooks; `add_log_event_hook` stacks. DevTools uses this for the Logs / Cache / Jobs tabs.

## DevTools config registry

Plugins push runtime mount info for the DevTools Config tab:

```rust
app.state(DevToolsConfigRegistry::default());
// in plugin install:
registry.set("graphql", json!({ "api": "/graphql", "graphiql": "/graphiql" }));
```

Merged into `GET /_devtools/config` → `mounts` object.

## Server tuning

Set in code or `[server]` / `[production.server]` in `sova.toml` ([Configuration](/guide/configuration#server-limits)):

| Knob | Purpose |
|------|---------|
| `max_body` / `max_body_size` | Default request body cap |
| `max_connections` | Concurrent TCP accepts |
| `max_upgraded_connections` | WebSocket / upgrade budget |
| `max_concurrent_streams` | HTTP/2 stream limit |
| `request_timeout` | Handler + read budget |
| `idle_timeout` / `drain_timeout` | Connection lifecycle |
| `trust_proxy` | Honor `X-Forwarded-*` for scheme/host |
| `keep_alive` | HTTP/1 keep-alive |

TLS: `App::tls(config)` / `BoundApp::serve()` with `sova` feature `tls`. `BoundApp` = `app.bind(addr).http(...).run()`.

Debug route map: `app.explain()` or `cargo run -- routes`.

## Testing without HTTP

```rust
let server = app.build()?;
let res = server.handle_request(Method::GET, "/ping", "").await;
```

Integration tests: `TestClient` + `ResponseAssert` ([Plugin SDK → Testing](/api/plugin-sdk/testing)). Prefer `Server::handle` over `App::handle` (avoids recompile per request).

## Logging

`listen` / `run` install a default `tracing` subscriber (`LogConfig::from_env`).

| Control | Default |
|---------|---------|
| `RUST_LOG` | built-in filter below (override with env) |
| `SOVA_LOG=off` | skip install |
| `SOVA_LOG_STDOUT` | `1` |
| `SOVA_LOG_FILE` | unset |
| `SOVA_LOG_ROTATE` | `size` |

When `RUST_LOG` is unset, Sova uses:

```text
sova=info,
sova.store=debug,
sova.tasks=debug,
sova.redis=debug,
sova.grpc=debug,
sova.rabbit=debug,
sova.graphql=debug
```

DevTools tabs (Cache, Jobs, Redis, GraphQL, gRPC, Rabbit) emit at DEBUG; the base app stays at INFO. Set `RUST_LOG=sova=info` to quiet plugin traces, or `RUST_LOG=debug` for everything.

## Stability

Pre-1.0: breaking changes without a major bump. `sova` **0.1** tracks `sova-core` **0.1**.
