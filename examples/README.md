# Ruvo examples

Each example is a small workspace crate. Run with `cargo run -p <name>`.

| Category | Package | What it shows |
|----------|---------|---------------|
| **cabinet** | `cabinet` | Full app: auth, SQLite, most web plugins |
| **basic** | `hello` | Modules, Cors, Static, health |
| | `rest_api` | JSON REST |
| | `blog` | `Router` + `mount` |
| | `auth` | Cookie sessions |
| | `cli` | `ServerArgs` |
| | `raw_echo` | Raw body |
| **web** | `upload` | `Request::input` / files / `Response::download` |
| | `static_files` | Static + ETag/Range |
| | `templates` | MiniJinja |
| | `templates_i18n` | Templates + `t()` |
| | `i18n` | Locales / path prefix |
| | `meta_blog` | Meta / Sitemap / Robots |
| **api** | `api_validated` | vld + OpenAPI |
| | `api_preset` | `App::api()` |
| | `api_auth` | Passport API key strategy |
| | `api_jwt` | `JwtAuth` + migrate + `/api/me` |
| | `api_oauth` | GitHub OAuth + JWT |
| | `crud` | SeaORM Postgres |
| **realtime** | `sse`, `sse_feed`, `ws_chat` | SSE / WebSocket |
| **net** | `udp_echo`, `quic_udp_echo`, `tls_hello` | UDP / QUIC / TLS |
| **misc** | `tasks`, `bench_loaded`, `share_demo` | Task queue / load / `Cell`+`Slot` handoff |

```bash
cargo run -p hello
cargo run -p cabinet
cargo run -p meta_blog
```
