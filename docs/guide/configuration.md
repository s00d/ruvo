# Configuration

![Configuration](/banners/configuration.svg)

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

# Recurring jobs (tasks plugin) — load toml before install(Tasks…)
[schedule.ping]
every = "15s"

[schedule.digest]
cron = "0 8 * * *"   # 5 fields → seconds padded; or 6-field cron
queue = "mailer"
# priority = 0
# payload = { "mode" = "full" }

[observability]
metrics_path = "/metrics"

[session]
# cookie = "sova_sid"
# ttl = "7d"
# same_site = "lax"
# secure = true

[csrf]
# field = "csrf"
# header = "x-csrf-token"
# auto = true

[i18n]
# default = "en"
# cookie = "locale"
# watch = true

[development.devtools]
enabled = true

[production.devtools]
enabled = false

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

Access-log noise: `sova::logger_skip_path("/healthz")` (or `logger_skip_paths`). DevTools registers `/_devtools` automatically.

See also: [DevTools guide](/guide/devtools).
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

## Schedules (`[schedule.<job>]`)

Used by [tasks](/plugins/tasks). Each table is a **registered** job name. Set **either** `cron` **or** `every` (not both). Toml overrides code `.cron()` / `.every()`.

| Key | Notes |
|-----|--------|
| `every` | Duration: `15s`, `2m`, `1h`, `500ms` |
| `cron` | 5-field (secs padded with `0 `) or 6-field |
| `queue` | optional |
| `priority` | optional int (`LOW=-100`, `NORMAL=0`, `HIGH=100`) |
| `payload` | optional TOML → JSON for scheduler enqueues |

## Plugin toml sections

| Section | Plugin |
|---------|--------|
| `[db]` | [db](/plugins/db) |
| `[redis]` | [redis](/plugins/redis) |
| `[mail]` | [mail](/plugins/mail) (`from`) |
| `[storage]` | [storage](/plugins/storage) |
| `[session]` | [session](/plugins/session) |
| `[csrf]` | [csrf](/plugins/csrf) |
| `[i18n]` | [i18n](/plugins/i18n) |
| `[observability]` | [observability](/plugins/observability) |
| `[schedule.*]` | [tasks](/plugins/tasks) |
| `[ai]` | [ai](/plugins/ai) (`system`) |
| `[devtools]` | [devtools](/plugins/devtools) (`enabled`) |

Per-plugin pages list env vars and builder knobs in **Config**.

## Env cascade (`sova-env`)

Call `sova_env::load()` at the top of `main` (never inside `App::new()`).

Later file wins; process env is never overwritten:

1. `.env.{short}` (`.env.dev` / `.env.prod` / `.env.test`)
2. `.env.{mode}` when mode ≠ short
3. `.env.local` (skipped in `test`)
4. `.env`

See also [env](/plugins/#env) and kitchen-sink `examples/cabinet/sova.toml`.

List installed plugins: `cargo run -- plugins`.
