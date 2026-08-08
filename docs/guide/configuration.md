# Configuration

![Configuration](/banners/configuration.svg?v=6)

One `sova.toml` for app limits, plugin defaults, and cargo-sovax frontend. Secrets stay in process env (often via `sova-env` + `.env*`).

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
url = "postgres://postgres@localhost/sova"

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

`SOVA_PROFILE` → else `SOVA_ENV` → else `development` (debug) / `production` (release).
Aliases: `debug`→`development`, `release`→`production`.
`cargo sovax dev` sets `SOVA_ENV=development`; `serve` sets `production`.

## Merge order

built-in defaults → `[section]` → `[<profile>.section]` → env (`DATABASE_URL` / `REDIS_URL` / `SOVA_*`) → explicit builder methods (toml only fills unset builder fields).

**Load:** `App::configure()` or `configure_from_path`. Presets `App::web()` / `App::api()` call `sova_env::load()` then `configure()`.

**Do not put in toml:** Fortify feature bits, RateLimit key fns, Notification channels, OAuth matrices, job handlers.

## URLs

| Toml | Env (wins if set and non-empty) |
|------|----------------------------------|
| `[db] url` | `DATABASE_URL` |
| `[redis] url` | `REDIS_URL` |

## Env cascade (`sova-env`)

Call `sova_env::load()` at the top of `main` (never inside `App::new()`).

Later file wins; process env is never overwritten:

1. `.env.{short}` (`.env.dev` / `.env.prod` / `.env.test`)
2. `.env.{mode}` when mode ≠ short
3. `.env.local` (skipped in `test`)
4. `.env`

See also [env](/plugins/#env) and kitchen-sink `examples/cabinet/sova.toml`.

List installed plugins: `cargo run -- plugins`.
