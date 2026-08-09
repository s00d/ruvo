[![crates.io](https://img.shields.io/crates/v/sova-devtools?style=for-the-badge)](https://crates.io/crates/sova-devtools)
[![docs.rs](https://img.shields.io/docsrs/sova-devtools?style=for-the-badge)](https://docs.rs/sova-devtools)
[![License](https://img.shields.io/crates/l/sova-devtools?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)

# sova-devtools

In-app DevTools for Sova — bottom bar on HTML pages, SSE site-wide timeline, request snapshots (DB / logs / HTTP / mail / session).

**Guide:** [https://s00d.github.io/sova/plugins/devtools](https://s00d.github.io/sova/plugins/devtools)

```bash
cargo add sova --features "web,devtools"
```

Enable with `SOVA_DEVTOOLS=1` or `[development.devtools] enabled = true`. Default: on in debug builds.

On install the plugin calls `logger_skip_path("/_devtools")` so the panel does not flood access logs. Apps can skip other paths with `sova::logger_skip_path("/healthz")`. Console/`tracing` lines show up under Logs (and on the request snapshot when `request_id` is present).

## Frontend

UI lives in `ui/` (Vite + Vue 3 + Tailwind CSS + Vue Router + Pinia + TypeScript + Lucide).

**Visual:** Symfony-style chip toolbar + Clockwork master-detail panes (IBM Plex, Lucide).

**Playground (UI only):** `npm --prefix plugins/sova-devtools/ui run playground` — viewport 375/768/1280 + mock fixtures.

**Backlog (Rust collectors):** memory, route/middleware, cache, AJAX XHR timeline.

**Embedding model**

- Each HTML page gets a tiny `bridge.js` (status bar only).
- The full panel is the SPA at `/_devtools/app`:
  - **Dock** — iframe toggled by the status bar (open state only in `sessionStorage`).
  - **New tab** — button opens `/_devtools/app` in a new browser tab; does not change dock behavior.

```bash
npm --prefix plugins/sova-devtools/ui ci
npm --prefix plugins/sova-devtools/ui run build
```

Built assets are committed under `assets/` and embedded via `include_str!`.

## License

MIT — see [LICENSE](LICENSE).
