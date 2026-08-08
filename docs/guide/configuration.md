# Configuration

![Configuration](/banners/configuration.svg)

One `ruvo.toml` for app limits, plugin defaults, and cargo-ruvo frontend. Secrets stay in process env (often via `ruvo-env` + `.env*`).

## Example

```toml
[server]
max_body = "2mb"
trust_proxy = false

[mail]
from = "App <noreply@example.com>"

[storage]
driver = "local"
path = "storage"
public_url = "/storage"

[db]
url = "postgres://postgres@localhost/ruvo"

[redis]
url = "redis://127.0.0.1:6379"

# [schedule.digest]
# cron = "0 8 * * *"

[observability]
metrics_path = "/metrics"

[meta]
site_name = "My App"
public_url = "http://127.0.0.1:3000"

[frontend]
dir = "frontend"

[development.server]
max_connections = 32

[production.server]
trust_proxy = true
```

## Profile

`RUVO_PROFILE` → else `RUVO_ENV` → else `development` (debug) / `production` (release).
Aliases: `debug`→`development`, `release`→`production`.
`cargo ruvo dev` sets `RUVO_ENV=development`; `serve` sets `production`.

## Merge order

built-in defaults → `[section]` → `[<profile>.section]` → env (`DATABASE_URL` / `REDIS_URL` / `RUVO_*`) → explicit builder methods (toml only fills unset builder fields).

**Load:** `App::configure()` or `configure_from_path`. Presets `App::web()` / `App::api()` call `ruvo_env::load()` then `configure()`.

**Do not put in toml:** Fortify feature bits, RateLimit key fns, Notification channels, OAuth matrices, job handlers.

## URLs

| Toml | Env (wins if set and non-empty) |
|------|----------------------------------|
| `[db] url` | `DATABASE_URL` |
| `[redis] url` | `REDIS_URL` |

## Env cascade (`ruvo-env`)

Call `ruvo_env::load()` at the top of `main` (never inside `App::new()`).

Later file wins; process env is never overwritten:

1. `.env.{short}` (`.env.dev` / `.env.prod` / `.env.test`)
2. `.env.{mode}` when mode ≠ short
3. `.env.local` (skipped in `test`)
4. `.env`

See also [env](/plugins/#env) and kitchen-sink `examples/cabinet/ruvo.toml`.

List installed plugins: `cargo run -- plugins`.
