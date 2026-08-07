# Ruvo Cabinet demo

Kitchen-sink sample app: cookie auth, personal cabinet, SQLite (app data + KV + tasks), and most HTTP/web Ruvo plugins.

## Run

```bash
cp .env.example .env   # optional
cargo run -p cabinet
```

Open http://127.0.0.1:3000

Seed user: `demo@ruvo.local` / `demo1234`

## Frontend

Server HTML via **Minijinja** (POST forms + CSRF stay as-is). Client polish only:

- **Tailwind CSS** — Play CDN (`cdn.tailwindcss.com`) configured in `views/layout.html`
- **Vue 3** — CDN islands (`public/js/app.js` nav/flash, `public/js/live.js` WebSocket)
- No npm / Vite / SSR

Static files: `Static` → `/assets` (`public/`).

## What maps to which plugin

| Area | Plugin / feature |
|------|------------------|
| CORS / security headers | `Cors` (`origins`/`exposed`), `Shield` (helmet-style) |
| CSRF (session double-submit) | `Csrf` + `req.csrf_token()` |
| Compression | `Compress::new()` (gzip/deflate/br, Express-style) |
| Rate limit (SQLite KV) | `RateLimit::fixed_window` + headers / `key_fn` |
| Sessions | `SessionLayer` (`destroy`/`regenerate`/`rolling`) |
| Static assets | `Static` → `/assets` |
| Templates + flash | `Templates`, `with_validation_flash` |
| i18n en/ru | `I18n` + cookie `locale` |
| SEO | `Meta`, `Sitemap`, `Robots` |
| Forms / JSON validation | `Vld`, `validate_form` / `validate_body` |
| Avatar upload | `Request::input` + `Upload::save_in` |
| OpenAPI | `OpenApi` → `/docs` |
| Background job | `Tasks` + `tasks::Sqlite` (`welcome_email` via `Mail`) |
| WebSocket | `Ws` → `/cabinet/ws` |
| Outbound HTTP | `Http` / `req.http()` → `/cabinet/fetch` |
| Mail | `Mail::from_env()` / `Mail::fake` (lettre SMTP) |
| CLI / env | `ServerArgs`, `ruvo_env` |

Domain tables (`users`, `notes`) use **sqlx SQLite** in this crate (`data/app.db`).  
`ruvo-db` is Postgres-only and is **not** used here.

## Intentionally skipped

| Plugin | Why / where else |
|--------|------------------|
| `ruvo-db` / store-postgres / tasks-postgres | Postgres — see `crud` example |
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
