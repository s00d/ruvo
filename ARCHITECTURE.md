# Architecture

Ruvo is a small Express-like HTTP framework: `ruvo-core` owns the request path;
plugins add optional middleware and helpers.

## Request path

```text
accept
  → server/conn (semaphore, JoinSet, hyper HTTP/1)
  → to_ruvo_request
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
is not rebuilt per request.

`handle_request(method, path, body)` is thin sugar over `Request::builder` with
**no custom headers**. For cookies, `Content-Type`, etc. use
`Request::builder().header(...).build()` and `handle`.

## Public surface: root vs `extend`

| Surface | Audience | Examples |
|---------|----------|----------|
| **crate root** | Applications | `App`, `Server`, `Router`, `Request`, `Response`, typed bodies (`Html`/`Json`/…), `Error`/`Result`/`IntoResponse`, `Plugin`, `Next`, `logger`, `with_state`, `ClientAddr` |
| **`extend`** | Plugins / advanced | `Handler`/`IntoHandler`, `ErrorResponse`, middleware traits, `named`/`with_leaked`, bodies (`Body`, `HttpBody`), path helpers, `RouteEntry`/`RouteTable`, `Extensions`/`TypeMap` (`StateMap` alias), `MatchedMeta`, `RequestBuilder` |

Route metadata is a [`TypeMap`](crates/ruvo-core/src/state.rs) on each HTTP route (`route_meta`).
**Same type twice — last wins**; different types never conflict. After a match,
the bag is available on the request as `MatchedMeta` / `req.route_meta::<T>()`.

The `ruvo` facade re-exports the same root list and `ruvo::extend`.

## Ownership

| Layer | Owns |
|-------|------|
| **ruvo-core** | App/Router/Server, dispatch, Request/Response, middleware traits, listen/drain, `ClientAddr`, route `TypeMap` |
| **plugins** | Optional features: cookies, session, rate-limit, cors, compress, static, multipart, templates, vld, openapi, i18n |

Plugins depend on `ruvo_core` (and sometimes other plugins). Core does not depend on plugins.

## Tests

| Location | Scope |
|----------|--------|
| `src/**` `#[cfg(test)]` | Unit tests of **private** helpers (`collect_limited`, path normalize, …) |
| `crates/*/tests/` and `plugins/*/tests/` | Integration against the **public** API (`root` + `extend`) |

Feature `testing` exposes `App::run_startup` / `run_shutdown` for lifecycle tests
(`cargo test --features testing` or `--all-features`).
