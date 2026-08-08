---
title: env
editLink: false
---

# `env`

**Cascade .env loading for Sova apps (dotenvy)** · crate `sova-env` `0.1.0` · id `env`

```bash
cargo add sova --features env
```

| Feature | What you get |
|---------|-------------|
| `env` | Cascade `.env*` loader (`sova-env`). |

Explicit `.env` cascade for Sova applications.

 Call [`load`] at the top of `main` before reading configuration.
 Real process environment variables always win over file values.

 File order (later overrides earlier):
 1. `.env.{dev|prod|test}` (short alias of the active mode)
 2. `.env.{mode}` when mode is the long name (`development` / `production`)
 3. `.env.local` (skipped in `test`)
 4. `.env` (final overlay)

## Usage

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
