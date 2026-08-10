# Changelog

All notable changes to the Sova workspace are recorded here.
Versions refer to the published crates on [crates.io](https://crates.io/crates/sova).

## 0.1.29 — 2026-08-10

### Fixed (DevTools)

- **Tab persistence**: switching routes no longer resets the active DevTools tab
- **CSRF on console POST**: actions send `X-XSRF-TOKEN` + cookies (web preset CSRF no longer returns 403)
- **DB tab empty on `/db-demo`**: `sova-db` emits INFO-level `sova.db` query traces (visible with default `RUST_LOG`)
- **Cache / Jobs tabs**: default log filter includes `sova.store`, `sova.tasks`, … at DEBUG so traces appear without extra env

### Added (DevTools)

- **HTTP tab**: merged Playground client + outbound traces (Playground tab removed; `/playground` → `/http`)
- **Cache / Redis split**: separate tabs — Cache (KV + `sova.store`) and Redis (`sova.redis` + console)
- **Session console** (Auth tab): list/set/delete/regenerate session keys via `POST /_devtools/actions/session`
- Facade: `devtools-console-session`
- **`sova-devtools` 0.1.10**

### Changed

- **`sova-core` 0.1.15**: `AppDispatch`, `DevToolsConfigRegistry`; default `RUST_LOG` includes DevTools trace targets
- **`sova-db` 0.1.6**: optional SQL trace helper (`target: sova.db`)
- **`sova-redis` 0.1.4**: `Redis::fake()` in-memory backend for demos/tests
- **`sovax` 0.1.2**: CLI default log filter aligned with `LogConfig`

## 0.1.28 — 2026-08-10

### Added (GraphQL server)

- **`GraphqlContext`**: inject Sova app state + auth header into resolvers and subscriptions
- **Separate GraphiQL path** (default `/graphiql`), optional GET queries, SDL endpoint
- **WebSocket subscriptions** via `async-graphql` (`.subscriptions("/graphql/ws")`)
- **`GraphqlServerExt`**: `req.graphql_schema()` / `try_graphql_schema()`
- Example: `examples/api/api_graphql_server`

### Added (DevTools + GraphQL)

- **GraphQL tab**: operations from `sova.graphql` tracing (name, kind, duration, errors)
- **Config → GraphQL server**: mounted paths via `DevToolsConfigRegistry`
- **Events**: WebSocket upgrade (`graphql.ws.upgrade`)
- **`devtools_demo`**: GraphQL server + GraphiQL integrated

### Added (DevTools control panel)

- **Console drawer** in the DevTools UI: HTTP replay (method/path/headers/body) and Redis console (DB select, GET/SET/DEL, PUBLISH, Pub/Sub SSE)
- **Actions API**: `POST /_devtools/actions/http`, `POST /_devtools/actions/redis`, `GET /_devtools/stream/redis`
- **`AppDispatch`**: in-process HTTP dispatch for console replay (no CORS)
- **`DevTools::console()`**, **`.allow_dangerous()`** — Redis denylist, body limits, audit via `devtools.action` events
- Facade: `devtools-console`, `devtools-console-redis`
- Integration tests: `plugins/sova-devtools/tests/console.rs`
- **`sova-devtools` 0.1.8**: console drawer + actions API

### Added (DevTools control panel UX + extensions)

- **Playground tab**: full HTTP client UI (Postman-like); removed cramped drawer HTTP mode
- **`Redis::fake()`**: in-memory Redis for demos/tests; `devtools_demo` uses it out of the box
- **Cache tab**: Traces | KV | Redis segments; Redis ops TTL/SCAN/TYPE
- **Actions**: store, graphql, tasks, mail, events, rabbit; external HTTP via `console_external`
- Facade features: `devtools-console-store`, `-graphql`, `-tasks`, `-mail`, `-http-external`, `-events`, `-rabbit`

## 0.1.27 — 2026-08-10

### Fixed

- **`graphql-server`**: compile with async-graphql 7 (`Variables::from_json`, `graphiql` feature); mount + GraphiQL tests
- **`rabbit` facade**: feature enables `sova-rabbit/lapin`; empty URL fails at startup (like Redis)
- **GraphQL / gRPC**: composite install (`GraphQl::with_client`, `Grpc::client` on server builder) for BFF apps; server-only documented via `try_*`
- **gRPC**: unified Connect error envelope; `unary_with_request` for auth/headers

### Added

- **`RabbitConsumer`**: BackgroundService worker over `FakeBroker` / lapin
- **CI**: `.github/workflows/rust.yml` (plugins + facade feature matrix)
- **Example**: `examples/api/api_rabbit`

## 0.1.26 — 2026-08-10

- New plugins (client-first): **`sova-graphql`**, **`sova-grpc`**, **`sova-rabbit`**
  - `graphql` / `graphql-server`: outbound GraphQL + FakeGraphql; optional schema mount
  - `grpc`: Connect-JSON unary client + FakeGrpc; optional server mount/bind
  - `rabbit`: raw AMQP (lapin) + FakeBroker (publish/consume, ack/nack, DLQ)
- Examples: `examples/api/api_graphql`, `examples/api/api_grpc`, `examples/api/api_rabbit`

## 0.1.25 — 2026-08-10

- Facade feature `testing` pulls in `sova-testing` (sqlite `TestApp`, snapshots) + `db-sqlite`; re-exports `TestApp`, `SqliteTestDb`, `assert_json_snapshot!`, module `sova::testing`
- `sova-testing` **0.1.3**: `assert_json_snapshot!` resolves insta via `$crate` (works through facade re-export without a direct `insta` dep)

## 0.1.15 — 2026-08-09

- `sova-devtools` **0.1.2**: HTML responses get `Cache-Control: no-store` (+ bridge `pageshow` reload) so browser Back is not silent bfcache

## 0.1.14 — 2026-08-09

- New plugin **`sova-devtools` 0.1.1**: in-app debug bar (HTML inject, SSE timeline, request snapshots)
- `sova-core` **0.1.7**: `logger_skip_path` / `HtmlInject`; multi log-event hooks
- Facade feature `devtools`; guide [/guide/devtools](docs/guide/devtools.md) with screenshots + tour GIF
- Release builds hard-disable DevTools unless `SOVA_DEVTOOLS=1`

## 0.1.13 — 2026-08-09

- `sova-core` **0.1.6**: release builds no longer warn on unread `AppInner::explain` (debug route map always available)

## 0.1.12 — 2026-08-09

- New plugin **`sova-ai` 0.1.0**: AISDK wrapper — `app.install(Ai::…)`, `req.ai()`, stream SSE, `FakeAi` for tests
- Facade features: `ai`, `ai-openai`, `ai-anthropic`, `ai-google`, `ai-full`, `ai-prompt`
- Example `examples/api/api_ai`; guide `/plugins/ai`

## 0.1.11 — 2026-08-09

Performance hot path (`sova-core` 0.1.5) + deeper release benches:

- `FxHashMap` (rustc-hash) for TypeMap / MetaMap / Extensions / params / query / method maps
- Arc-wrap matched `MetaMap` / route path (no deep clone per request)
- Skip catcher snapshot wrapper when no catchers registered; skip raw-route lookup when unused
- Move response headers into hyper without clone; HEAD preserves `Content-Length`
- Route `RequestTimeout` returns **408** (was 504); request-id uses entropy + counter
- Workspace `[profile.release]` / `[profile.bench]`: thin LTO, `codegen-units = 1`
- Stand: longer defaults, warm-up, `POST /api/echo`, release-only load + criterion realistic/burst groups

## 0.1.10 — 2026-08-09

Deep audit fixes:

- Reject duplicate `Plugin::id` on `App::install` (build error)
- `Fortify::new()` defaults to Registration only
- `App::api()` installs `Vld`
- Notifications template helpers require `templates`; i18n-cookie always requires cookies
- cargo-sovax: `--fields` required; csrf/templates stack; entities stub; seed wiring; uuid/chrono features
- Docs: SSE/cookies/cors/shield/session dual-install; getting-started forms/uploads/auth features

## 0.1.9 — 2026-08-09

- Docs: CSRF field `csrf`, no double-install after `App::web()`
- Docs-gen: prefer plugin id matching page slug (`meta` not `sitemap`)
- `Notifications::ws_path` requires installed `ws`
- `cargo sovax generate plugin` uses crates.io `sova-core` (no monorepo path)
- Facade `sova` 0.1.9 (auth 0.1.5, notifications 0.1.3, cargo-sovax 0.1.6)

## 0.1.8 — 2026-08-09

- Facade crates.io README: use root README (remove conflicting stub)

## 0.1.7 — 2026-08-09

Package / docs hygiene across the monorepo:

- Shared `[workspace.package]` (`authors`, `license`, `repository`, `homepage`, `edition`)
- `README.md` + `LICENSE` in every published crate and plugin (crates.io badges)
- Index READMEs under `crates/` and `plugins/`
- Patch bump of all published packages for metadata packaging
- Break publish cycle: auth/notification test helpers moved to `sova_auth::testing` / `sova_notifications::testing`; `sova-testing` stays core+db only

## 0.1.6 — 2026-08-09

DX simplification:

- Unique `MigrationName` across plugins / examples / `cargo-sovax` codegen
- `TestClient::boot` / `tracked` run startup and accept `Into<App>`
- `Db::seed` accepts `Into<Error>` (facade `AppError` works)
- Facade `auth` without mail by default; use `auth-mail` for verify/reset
- `meta` no longer enables OpenAPI by default (`meta-openapi`)
- `Error::bad_request`; Fortify / session / testing docs updates

## 0.1.5 — earlier

HN-style example, Fortify Registration-only path, release smoke scripts, and related auth/db fixes.

See git history for older workspace commits.
