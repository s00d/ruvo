### Adding routes from a plugin

```rust
app.get("/_devtools/app", handler);
app.post("/_tasks/enqueue", handler);
app.mount("/api", |r| { /* nested router */ });
```

Examples: [static](/plugins/static) (mount + `/*path`), [devtools](/plugins/devtools), [openapi](/plugins/openapi), [i18n](/plugins/i18n) prefix, tasks HTTP enqueue.

Guard sensitive surfaces (bearer token, `cli_mode`, localhost-only) — see tasks enqueue + DevTools release guard.

### Path helpers

```rust
use sova_core::extend::{normalize_path, to_brace_path, join_paths};
```

Static files and OpenAPI rely on consistent path normalization.

### Route introspection (`RouteValue` / `MetaMap`)

Plugins attach typed metadata to routes for other systems:

| Consumer | Meta |
|----------|------|
| [openapi](/plugins/openapi) | operation schemas |
| [vld](/plugins/vld) | validation rules / coverage |
| [meta](/plugins/meta) | per-page OG / robots / sitemap |

```rust
use sova_core::extend::{MetaMap, RouteValue, Needs};
// attach via router builders / with_update patterns used in those crates
```

If your plugin needs OpenAPI or validation awareness, follow vld/openapi for `RouteTable` / `RouteEntry` iteration rather than inventing a parallel registry.

### Match captures

After the router matches:

| Type | Purpose | Used by |
|------|---------|---------|
| `MatchedRoute` / `MatchedRouteCapture` | pattern + captures | [devtools](/plugins/devtools), [observability](/plugins/observability) |
| `MatchedMeta` / `MatchedMetaCapture` | overlay page meta | [meta](/plugins/meta) |

```rust
use sova_core::{MatchedRouteCapture, Request};

let cap = MatchedRouteCapture::new();
req.set(cap.clone());
let res = next(req).await;
let pattern = cap.get(); // Option after match
```

Install capture **before** `next` so the router can fill it.

### `Needs` on routes

See [State & dependencies](/api/plugin-sdk/state) — session uses `Needs::<CookieLayerPresent>` so session routes refuse to build without cookies MW.
