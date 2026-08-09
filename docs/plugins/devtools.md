---
title: devtools
editLink: false
---

# `devtools`

**In-app debug bar (HTML inject, SSE timeline, request snapshots)**

| | |
|--|--|
| Crate | [`sova-devtools`](https://docs.rs/sova-devtools/0.1.3) `0.1.3` |
| Plugin id | `devtools` |
| Category | Ops |

## Install

```bash
cargo add sova --features devtools
```

## Features

| Feature | What you get |
|---------|-------------|
| `devtools` | Bar + session/mail/http/auth/db/tasks soft-hooks |
| `devtools-store` | + Cache/KV (`sova.store`) |
| `devtools-redis` | + Redis messaging traces |
| `devtools-i18n` / `devtools-csrf` / `devtools-passport` / `devtools-rate-limit` | optional soft-hooks |

## Overview

**When:** local debugging of HTML apps — request timeline, SQL, logs, outbound HTTP, mail, session.

**Does:**
- Injects a bottom bar into `text/html` only (not JSON/SSE/streams)
- Collects per-request snapshot (correlated via `request_id`)
- Site-wide live feed over SSE `GET /_devtools/events`
- JSON: `/_devtools/requests`, `/_devtools/requests/:id`, `/_devtools/logs`, `/_devtools/config`
- Soft hooks: session dump, FakeMail, sqlx/http tracing via `add_log_event_hook`
- Mirrors console/`tracing` into Logs; skips `/_devtools` access logs via `logger_skip_path`
- **Release builds:** hard-off unless `SOVA_DEVTOOLS=1`

Full guide (screenshots + tour GIF): [DevTools](/guide/devtools)

### Example

```rust
app.install(DevTools::new()); // on in debug / development
```

### Config

```toml
[development.devtools]
enabled = true

[production.devtools]
enabled = false
```

```bash
SOVA_DEVTOOLS=1   # force on (incl. release)
SOVA_DEVTOOLS=0   # force off
```

Default: on in debug + development profile; off in `--release`.

For SQL tab with SeaORM: `Db::from_env().sqlx_logging(true)` and/or `RUST_LOG=sqlx=debug`.

### Notes
- GET-only under `/_devtools/*` — not for production exposure
- Install after session/mail if you want those tabs filled
- See `examples/web/devtools` (`devtools_demo`)
- Guide: [/guide/devtools](/guide/devtools)

## Quick start

Debug-only toolbar for **HTML** pages. Full walkthrough: [DevTools guide](/guide/devtools).

```rust
use sova::{App, DevTools, Mail, Parser, ServerArgs};

#[tokio::main]
async fn main() -> sova::Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("App")
        .public_url("http://127.0.0.1:3000")
        .into_app();

    app.install(Mail::from_env()); // optional — Mail tab
    app.install(DevTools::new());  // on in debug; off in --release

    app.get("/", || async { sova::Html("<html><body><h1>hi</h1></body></html>") });
    app.run().await
}
```

```bash
cargo run -p devtools_demo
# http://127.0.0.1:3030/ — click the bottom bar
```

Open Timeline, click another link — SSE updates the list. Mail / HTTP tabs fill after those demo actions.

**Production:** `cargo build --release` keeps DevTools disabled (even with `.enabled(true)`). Use `SOVA_DEVTOOLS=1` only as an ops escape hatch.
