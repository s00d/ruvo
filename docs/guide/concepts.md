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

- `Cell<T: Clone>` — counters / flags
- `Slot<T>` — ownership handoff for sockets/streams

See `examples/misc/share_demo`.

## Logging

`listen` / `run` install a default `tracing` subscriber (`LogConfig::from_env`).

| Control | Default |
|---------|---------|
| `RUST_LOG` | `sova=info` |
| `SOVA_LOG=off` | skip install |
| `SOVA_LOG_STDOUT` | `1` |
| `SOVA_LOG_FILE` | unset |
| `SOVA_LOG_ROTATE` | `size` |

## Stability

Pre-1.0: breaking changes without a major bump. `sova` **0.1** tracks `sova-core` **0.1**.
