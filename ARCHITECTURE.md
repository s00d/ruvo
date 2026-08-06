# Architecture

Ruvo is a small Express-like HTTP framework: `ruvo-core` owns the request path;
plugins add optional middleware and helpers.

## Request path

```text
accept
  → server/conn (semaphore, JoinSet, hyper auto HTTP/1.1+HTTP/2 + with_upgrades)
  → to_ruvo_request (+ optional OnUpgrade)
  → CompiledRouter::dispatch
  → root middleware (onion)
  → matchit route match
  → route / mount middleware
  → handler
  → IntoResponse
  → hyper response body
```

`App::build()` compiles routes once into a cheap-to-clone [`Server`]. Prefer
`Server::handle` (or `handle_request`) in tests and embedded use so the matcher
is not rebuilt per request. `Server::state` / `Server::run_startup` (feature
`testing`) are non-destructive.

`handle_request(method, path, body)` is thin sugar over `Request::builder` with
**no custom headers**. For cookies, `Content-Type`, etc. use
`Request::builder().header(...).build()` and `handle`.

## Lifecycle

```text
start: compile → on_startup → BackgroundServices → accept
stop:  stop accept → drain connections → stop services → on_shutdown
```

`app.run()` is the primary entrypoint: it parses CLI commands (`check`, `routes`,
`openapi --out`, `tasks`, `i18n missing`, plus plugin-registered commands such as
`migrate`) and exits, or starts the server path.
CLI command mode runs startup/shutdown hooks and skips the accept loop.

Plugin errors that already know their HTTP shape use [`Error::Response`] /
[`Error::custom`]; `wrap_errors` does **not** pass them through `error_handler`.
Validation (`ruvo-vld`) and SeaORM `DbErr` convert into that bridge so handlers
stay on `ruvo_core::Result` with `?`.

## Database (plugin `ruvo-db`, feature `db`)

One SeaORM/sqlx Postgres pool per process:

```text
Db::from_env() → on_startup connect+ping → req.db() → ConnectionTrait
transaction() middleware → commit on 2xx / rollback otherwise
myapp migrate|status|down via App::register_cli
```

Raw backends (`ruvo-store-postgres`, `ruvo-tasks-postgres`) reuse
`DatabaseConnection::get_postgres_connection_pool()` — ORM and queues share the
same pool; they do not open a second connection string.

## TLS (feature `tls`)

TLS terminates in the accept task (not the accept loop): `TcpStream` →
`tokio-rustls` handshake (with timeout) → generic IO into hyper auto (HTTP/1.1+HTTP/2).
`OnUpgrade` / WebSocket need no TLS-specific code (WSS is WS over TLS).

Hot reload: `ResolvesServerCert` + `ArcSwap<CertifiedKey>` — call
`TlsRuntime::reload()` or replace PEM on disk and reload. Optional
`.redirect_http(80)` and `.hsts(true)`.

`dev-tls` uses `rcgen` for local self-signed certs.

## Env (plugin `ruvo-env`)

Call `ruvo_env::load()` explicitly at the top of `main` (never inside
`App::new()`). Cascade: `.env` → `.env.local` → `.env.{mode}` →
`.env.{mode}.local`; real process env always wins.

## Store encryption (feature `store-crypto`)

`Encrypted<S: KvStore>` wraps any backend; values are XChaCha20-Poly1305,
keys use stable HMAC tails for `clear_prefix`. `incr` bypasses encryption.

## UDP

Plain datagrams only — no DTLS. Encrypted datagrams: future `ruvo-quic`.

## Long-lived connections

| Kind | How |
|------|-----|
| HTTP upgrade (WS) | Route handler + `req.on_upgrade()`; budget via `max_upgraded_connections` → **503** + `Retry-After` |
| SSE / stream body | Normal `Response`; `request_timeout` ends when the handler **returns** — stream chunks after that are not cut by it |
| Raw Hyper | `Router::raw` — **last resort**; skips middleware |

Upgraded / service state is **process-local** (not shared across processes).

## Public surface: root vs `extend`

| Surface | Audience | Examples |
|---------|----------|----------|
| **crate root** | Applications | `App`, `Server`, `Router`, `Request`, `Response`, typed bodies (`Html`/`Json`/…), `Error`/`Result`/`IntoResponse`, `Plugin`, `Next`, `logger`, `with_state`, `ClientAddr`, `BackgroundService`, `OnUpgrade` |
| **`extend`** | Plugins / advanced | `Handler`/`IntoHandler`, `ErrorResponse`, middleware traits, `named`/`with_leaked`, bodies (`Body`, `HttpBody`), path helpers, `RouteEntry`/`RouteTable`, `Extensions`/`TypeMap` (`StateMap` alias), `MatchedMeta`, `RequestBuilder`, `wait_shutdown` |

Route metadata is a [`TypeMap`](crates/ruvo-core/src/state.rs) on each HTTP route (`route_meta`).
**Same type twice — last wins**; different types never conflict. After a match,
the bag is available on the request as `MatchedMeta` / `req.route_meta::<T>()`.

The `ruvo` facade re-exports the same root list and `ruvo::extend`.

## Ownership

| Layer | Owns |
|-------|------|
| **ruvo-core** | App/Router/Server, dispatch, Request/Response, middleware traits, listen/drain, `ClientAddr`, route `TypeMap`, `BackgroundService`, `OnUpgrade` |
| **plugins** | Optional features: cookies, session, rate-limit, cors, compress, static, multipart, templates, vld, openapi, i18n, ws, tasks, store, udp, sse, **db** (SeaORM), store-postgres, tasks-postgres |

Plugins depend on `ruvo_core` (and sometimes other plugins). Core does not depend on plugins. **KvStore is not in core** — wire via `app.state(...)`.

## Tests

| Location | Scope |
|----------|--------|
| `src/**` `#[cfg(test)]` | Unit tests of **private** helpers (`collect_limited`, path normalize, …) |
| `crates/*/tests/` and `plugins/*/tests/` | Integration against the **public** API (`root` + `extend`) |

Feature `testing` exposes `App::run_startup` / `run_shutdown` (delegating to `Server`) for lifecycle tests
(`cargo test --features testing` or `--all-features`).
Feature `listen-reuseport` enables `BoundApp::reuseport(true)` (`SO_REUSEPORT`).
