# Ruvo Cabinet demo

Kitchen-sink sample app: Fortify auth, personal cabinet, Postgres via `ruvo-db`
(ORM + shared SQL KV/queue), and most HTTP/web Ruvo plugins.

## Run

```bash
export DATABASE_URL=postgres://postgres@localhost/ruvo
cp .env.example .env   # optional

# one-shot
cargo run -p cabinet -- migrate
cargo run -p cabinet

# or via cargo-ruvo (watch / prod)
cargo ruvo dev -p cabinet
cargo ruvo build -p cabinet
cargo ruvo serve -p cabinet
```

Open http://127.0.0.1:3000

Seed user: `demo@ruvo.local` / `demo1234`

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
## What maps to which plugin

| Area | Plugin / feature |
|------|------------------|
| CORS / security headers | `Cors` (`origins`/`exposed`), `Shield` (helmet-style) |
| CSRF (session double-submit) | `Csrf` + `req.csrf_token()` |
| Compression | `Compress::new()` (gzip/deflate/br, Express-style) |
| Rate limit (SQL KV) | `RateLimit::fixed_window` + `store::Sql` |
| Sessions | `SessionLayer` on shared `SqlStore` |
| Static assets | `Static` → `/assets` |
| Templates + flash | `Templates`, `with_validation_flash` |
| i18n en/ru | `I18n` + cookie `locale` |
| SEO | `Meta`, `Sitemap`, `Robots` |
| Forms / JSON validation | `Vld`, `validate_form` / `validate_body` |
| Avatar upload | `Request::input` + `Upload::save_in` |
| OpenAPI | `OpenApi` → `/docs` |
| Background job | `Tasks` + `tasks::Sql` (`welcome_email` via `Mail`) |
| WebSocket | `Ws` → `/cabinet/ws` |
| Outbound HTTP | `Http` / `req.http()` → `/cabinet/fetch` |
| Mail | `Mail::from_env()` / `Mail::fake` (lettre SMTP) |
| CLI / env | `ServerArgs`, `ruvo_env` |

Identity, notes, sessions KV, and the task queue share one **`DbPool`** (`DATABASE_URL`).

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
