---
title: env
editLink: false
---

# `env`

**Cascade .env loading for Sova apps (dotenvy)**

| | |
|--|--|
| Crate | [`sova-env`](https://docs.rs/sova-env/0.1.1) `0.1.1` |
| Plugin id | `env` |
| Category | HTTP |

## Install

```bash
cargo add sova --features env
```

## Features

| Feature | What you get |
|---------|-------------|
| `env` | Cascade `.env*` loader. |

## Overview

**When:** load `.env` / cascade env files before config.

**Does:**
- dotenvy cascade for Sova apps
- Safe to install early in `main`

### Example

```rust
app.install(Env::default());
```

## Quick start

**`App::web()` / `App::api()`** load env via the `env` feature when the preset starts. Prefer `ServerArgs` + `configure()` / `sova.toml` over ad-hoc dotenv calls.

```rust
let args = ServerArgs::parse();
args.init_tracing();

let mut app = App::web()
    .site("App")
    .public_url("https://example.com");
// configure() already ran inside the preset; override path if needed:
// let mut app = App::web()....into_app();
// let _ = app.configure_from_path("sova.toml");

app.run().await
```

See [Configuration](/guide/configuration).

## Related

[`cli`](/plugins/cli)
