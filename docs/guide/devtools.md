# DevTools

In-app debug bar for Sova HTML apps — Symfony / Clockwork style. Inspect the last request, SQL, logs, outbound HTTP, mail, session, and a live site-wide timeline — without leaving the browser.

::: tip Development only
DevTools is **off in release builds** (`cargo build --release`). Toml or `.enabled(true)` cannot turn it on there. Ops escape hatch: `SOVA_DEVTOOLS=1`.
:::

![DevTools tour](/devtools/tour.gif)

## Install

```bash
cargo add sova --features "web,devtools"
```

```rust
use sova::{App, DevTools, Parser, ServerArgs};

#[tokio::main]
async fn main() -> sova::Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("App")
        .public_url("http://127.0.0.1:3000")
        .into_app();

    app.install(DevTools::new()); // on in debug / development by default
    app.get("/", || async { sova::Html("<html><body><h1>hi</h1></body></html>") });
    app.run().await
}
```

Demo app in the repo:

```bash
cargo run -p devtools_demo
# http://127.0.0.1:3030/
```

## What you get

| Surface | Purpose |
|---------|---------|
| Bottom bar | Status / time / SQL / errors chips; click to dock the panel |
| Dock panel | Full Vue SPA in an iframe (`/_devtools/app?embed=1`) |
| New tab | Open the same SPA in a browser tab |
| SSE feed | Live `request.finished` events → Timeline |
| JSON API | Snapshots, logs, config under `/_devtools/*` |

### Tabs

| Tab | Contents |
|-----|----------|
| **Request** | Method, path, status, duration, route / locale / CSRF / rate-limit / encoding |
| **Timeline** | Recent requests (SSE); click to load a snapshot |
| **DB** | SQL queries for the selected request (bindings redacted) |
| **Cache** | KvStore / Cache / Redis ops (`sova.store` / `sova.redis`) |
| **Logs** | `tracing` / console lines (per-request + site-wide) |
| **HTTP** | Outbound client calls |
| **Mail** | FakeMail / last messages (with `mail` feature) |
| **Jobs** | Task enqueue / worker (`sova.tasks`) |
| **Auth** | Session keys + user / email / roles (redacted) |
| **Config** | Profile + compiled DevTools feature flags |

### What comes from where

| Signal | Source | Needs |
|--------|--------|-------|
| Route pattern | `MatchedRouteCapture` soft-hook | always |
| Locale | `LocaleCode` | `devtools-i18n` / `sova-devtools/i18n` |
| CSRF present | `CsrfToken` | `devtools-csrf` |
| Rate-limit headers | `ratelimit-*` / `x-ratelimit-*` on response | always (headers) |
| Content-Encoding | response header | always |
| Session / user | Session + `CurrentUser` | `devtools` (auth) |
| SQL | sqlx / SeaORM logs | `devtools` (db) + sqlx logging |
| Outbound HTTP | `http.client` spans | `http-client` + `sova-devtools/http` |
| Mail | FakeMail bag | `mail` |
| Cache / KV | `tracing` `target: "sova.store"` | `devtools-store` (instrumented store) |
| Redis pub/queue | `target: "sova.redis"` | `devtools-redis` |
| Jobs | `target: "sova.tasks"` | `devtools` (tasks) |

Facade features: `devtools`, `devtools-store`, `devtools-redis`, `devtools-i18n`, `devtools-csrf`, `devtools-passport`, `devtools-rate-limit`.

![Request tab](/devtools/tab-request.png)

![Timeline](/devtools/tab-timeline.png)

![DB queries](/devtools/tab-db.png)

![Logs](/devtools/tab-logs.png)

![Outbound HTTP](/devtools/tab-http.png)

## How it works

1. Middleware opens a collector bag for every non-`/_devtools` request (needs `request_id`).
2. Soft hooks attach SQL / HTTP / mail / session data while the request runs.
3. On finish, a snapshot is stored and broadcast over SSE.
4. HTML responses get a tiny host marker + `bridge.js` (not the full SPA).
5. The bar toggles a dock iframe; **New tab** opens `/_devtools/app`.

Access logs for `/_devtools/*` are skipped via `logger_skip_path("/_devtools")` so the panel does not pollute the console or its own Logs tab.

## Enable / disable

| Context | Behavior |
|---------|----------|
| `cargo run` / debug build, development profile | **On** by default |
| Debug build + `SOVA_PROFILE=production` | **Off**, unless `.enabled(true)` or `SOVA_DEVTOOLS=1` |
| `cargo build --release` | **Off**, unless `SOVA_DEVTOOLS=1` |
| `SOVA_DEVTOOLS=0` | Always off |

```toml
[development.devtools]
enabled = true

[production.devtools]
enabled = false
```

```bash
SOVA_DEVTOOLS=1   # force on (including release)
SOVA_DEVTOOLS=0   # force off
```

```rust
app.install(DevTools::new());                 // default
app.install(DevTools::new().enabled(true));   // debug only — ignored in release
app.install(DevTools::new().request_cap(200).log_cap(1000));
```

## Filling the tabs

Install related plugins **before** DevTools when you want those panels populated:

```rust
app.install(Mail::from_env());       // Mail tab (fake backend)
app.install(OutboundHttp::new());    // HTTP tab
app.install(DevTools::new());
```

For Cache / Redis / Jobs, use instrumented plugins (`sova-store` / `sova-redis` / `sova-tasks`) and enable the matching facade features (`devtools-store`, `devtools-redis`; jobs come with `devtools`).

SQL (SeaORM): enable sqlx logging, e.g. `Db::from_env().sqlx_logging(true)` and/or `RUST_LOG=sqlx=debug`.

Skip other noisy routes from access logs:

```rust
sova::logger_skip_path("/healthz");
```

## Responsive UI

The dock uses the host window width for breakpoints. The **UI playground** (Vite) embeds the same SPA in an iframe so phone/tablet presets exercise real media queries:

```bash
npm --prefix plugins/sova-devtools/ui run playground
# http://localhost:5175/playground.html
```

![Desktop playground](/devtools/playground-desktop.png)

![Tablet](/devtools/playground-tablet.png)

![Mobile](/devtools/playground-mobile.png)

## Security notes

- `/_devtools/*` is a **GET-only** debug surface — do not expose it on the public internet.
- Release builds keep the plugin inert even if you forget to strip the feature from `Cargo.toml`.
- Session values and SQL bindings are redacted/masked in the UI.
- HTML pages that get the bar send `Cache-Control: no-store` so browser **Back** is not served from bfcache (otherwise no server hit → empty Timeline).

## Related

- Plugin catalog: [devtools](/plugins/devtools)
- Example: `examples/web/devtools` (`devtools_demo`)
- Core helper: `sova::logger_skip_path` (skip noisy routes from access logs)
