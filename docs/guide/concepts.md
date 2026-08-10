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
