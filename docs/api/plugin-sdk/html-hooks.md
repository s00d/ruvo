---
title: HTML & log hooks
editLink: false
---

# HTML & log hooks

HTML inject, logger_skip_path, add_log_event_hook for DevTools-style sinks.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/html-hooks.md`, then `pnpm docs:generate`.

### HTML injection

`extend` re-exports helpers from `sova_core::html`:

- `inject`, `inject_head`, `inject_body_end`, `inject_before`, …
- `HtmlInject`, `HtmlAnchor`, `find_ci`, `replace_once`, …

[meta](/plugins/meta) injects `<head>` tags; [devtools](/plugins/devtools) injects a host marker + `bridge.js` into `text/html` only.

Guidelines:

- Only mutate `text/html` (skip JSON, SSE, streams, downloads)
- Prefer small host snippets; serve heavy SPA from a dedicated route
- Consider `Cache-Control: no-store` when injecting request-specific UI (DevTools / bfcache)

`map_html` is available for simpler “map body string” middleware; deep plugins often branch on content-type manually.

### Skip noisy access logs

```rust
sova_core::logger_skip_path("/_devtools");
// or extend::logger_skip_paths(&[…])
```

DevTools uses this so panel polling does not flood Logs / stdout.

### Tracing → plugin sinks

```rust
use sova_core::extend::add_log_event_hook;

add_log_event_hook(Arc::new(|rec: LogRecord| {
    // filter by target, attach to open request bag
}));
```

[devtools](/plugins/devtools) parses `sqlx`, `http.client`, `sova.store`, `sova.redis`, `sova.tasks`. If you emit telemetry for DevTools:

- Use stable `target:` strings
- Include `request_id` field or rely on `current_request_id()`
- Prefer `debug!` with structured fields (`op`, `key`, `duration_ms`) — avoid logging secret values

Also: `set_log_event_hook` (legacy single), `LogConfig` / `ensure_tracing` for tests.
