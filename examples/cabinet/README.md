# Ruvo Cabinet demo

Kitchen-sink sample app: Fortify auth, personal cabinet, Postgres via `ruvo-db`
(ORM + shared SQL KV/queue), and most HTTP/web Ruvo plugins.

## Run

```bash
# Optional: DATABASE_URL overrides [db] url in ruvo.toml
# export DATABASE_URL=postgres://postgres@localhost/ruvo
cp .env.example .env   # optional (.env wins over .env.dev in cascade)
# or: cp .env.example .env.dev

# schema + demo user
cargo ruvo db migrate -p cabinet
cargo ruvo db seed -p cabinet
# or: cargo run -p cabinet -- migrate && cargo run -p cabinet -- seed

cargo run -p cabinet

# or via cargo-ruvo (watch / prod)
cargo ruvo dev -p cabinet          # graceful overlap on Unix; watches .rs/.env/ruvo.toml
cargo ruvo dev -p cabinet --no-graceful
cargo ruvo build -p cabinet
cargo ruvo serve -p cabinet
```

Open http://127.0.0.1:3000

Seed user (after `db seed`): `demo@ruvo.local` / `demo1234`

Useful DB commands:

```bash
cargo ruvo db status -p cabinet
cargo ruvo db down -p cabinet 1
```

## Frontend

Server HTML via **Minijinja** (POST forms + CSRF stay as-is). Client polish only:

- **Tailwind CSS** — Play CDN (`cdn.tailwindcss.com`) configured in `views/layout.html`
- **Vue 3** — CDN islands (`public/js/app.js` nav/flash, `public/js/live.js` WebSocket)
- No npm / Vite required — `cargo ruvo dev` runs Rust only

Static files: `Static` → `/assets` (`public/`).

### Optional Vite

`cargo ruvo` auto-detects `frontend/package.json` (or `[frontend]` in `ruvo.toml`). Example:

```json
// frontend/package.json
{ "scripts": { "dev": "vite", "build": "vite build" }, "devDependencies": { "vite": "^6" } }
```

```js
// frontend/vite.config.js
import { defineConfig } from "vite";
export default defineConfig({
  base: "/assets/build/",
  build: { outDir: "../public/build", emptyOutDir: true },
  server: { proxy: { "/": "http://127.0.0.1:3000" } }, // optional
});
```

Then `cargo ruvo dev -p cabinet` starts Vite + Rust; `cargo ruvo build` runs `npm run build` then release.

## Config (`ruvo.toml`)

Declarative settings: `[server]`, `[mail]`, `[storage]`, `[meta]`, `[observability]`,
`[cors]`, `[session]`, … with `[development.*]` / `[production.*]` overlays.
`main` calls `configure_from_path` on this file. Code still owns Fortify features,
RateLimit keys, Notification channels, and absolute storage paths.

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for merge order and what stays in env.

## What maps to which plugin

| Area | Plugin / feature |
|------|------------------|
| CORS / security headers | `Cors` (`origins`/`exposed`), `Shield` (helmet-style) |
| CSRF (session double-submit) | `Csrf` + `req.csrf_token()` |
| Compression | `Compress::new()` (gzip/deflate/br, Express-style) |
| Rate limit (SQL KV) | `RateLimit::fixed_window(...).key(Identity)` + Fortify `RateLimit::login()` |
| Sessions | `SessionLayer::from_store(SqlSessionStore)` (`ruvo_sessions`) |
| Observability | `request_id` + `Observability` → `/metrics` |
| Static assets | `Static` → `/assets` |
| Templates + flash | `Templates`, `with_flash` (`errors` / `old` / `status`) |
| i18n en/ru | `I18n` + cookie `locale` |
| SEO | `Meta`, `Sitemap`, `Robots` |
| Forms / JSON validation | `Vld`, `validate_form` / `validate_body` |
| Avatar upload | `UploadRules` + `storage().store_as` → `public/uploads` (cloud: `examples/misc/storage`) |
| OpenAPI | `OpenApi` → `/docs` |
| Background job | `Tasks` + `tasks::Sql` (`welcome_email` via `Mail`) |
| WebSocket | `Ws` → `/cabinet/ws` |
| Outbound HTTP | `Http` / `req.http()` → `/cabinet/fetch` |
| Mail | `Mail` + MiniJinja (`mail-templates`): `.view` / Mailable; auth uses `mail/verify.html` + `mail/reset.html` |
| Activity / audit log | `Activity` + `auth-activity` → `GET /activity` (admin) |
| Notifications inbox | `Notifications` + channels/ACL → `/notifications`, optional WS |
| CLI / env | `ServerArgs`, `ruvo_env` |

Identity, notes, KV (rate-limit/cache), task queue, and **`ruvo_sessions`** share one **`DbPool`** (`DATABASE_URL`).

## Intentionally skipped

| Plugin | Why / where else |
|--------|------------------|
| `udp` / `quic-udp` / `tls` | Not a web cabinet — see `udp_echo`, `quic_udp_echo`, `tls_hello` |
| SSE | Overlaps WS in this demo |

## Smoke checklist

1. Register or log in as demo user  
2. Dashboard + create a note  
3. Upload avatar on profile  
4. Open `/cabinet/live` and send a WS message  
5. `/cabinet/fetch` with `https://example.com`  
6. `/docs` OpenAPI UI  
7. `/sitemap.xml` and `/robots.txt`
