---
title: Middleware
editLink: false
---

# Middleware

named, with_leaked, with_state, MwEntry — when to use each.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/middleware.md`, then `pnpm docs:generate`.

Middleware is the most common plugin surface. Prefer **named** layers so `routes` / explain show `"cors"`, `"session"`, not anonymous closures.

### Choose a helper

| Helper | Config ownership | When (real plugins) |
|--------|------------------|---------------------|
| `named("id", \|req, next\| …)` | none / capture by move | cookies, simple guards |
| `named(…, with_leaked(cfg, …))` | `&'static` via `Box::leak` or const | cors, shield, compress |
| `named(…, with_state(arc, …))` | `Arc<T>` (crate-root `with_state`) | session, csrf, rate-limit, i18n |
| `MwEntry` / `IntoMwEntry` | composable | auth/passport guards, db tx, route `.with(mw)` |

`with_leaked` and `named` live in `sova_core::extend`. `with_state` is also on the crate root (apps and plugins both use it).

### `named` — always label

```rust
use sova_core::extend::named;

app.use_middleware(named("cookies", |mut req, next| async move {
    // parse Cookie → req.set(Cookies)
    next(req).await
}));
```

### `with_leaked` — immutable sync config

Best when config is fixed after install (CORS allowlist, security headers). Avoids `Arc` clone on every request:

```rust
use sova_core::extend::{named, with_leaked};

let cfg = CorsConfig { /* … */ };
app.use_middleware(named(
    "cors",
    with_leaked(cfg, |cfg, req, next| async move {
        // *cfg is &'static CorsConfig
        next(req).await
    }),
));
```

Used by: [cors](/plugins/cors), [shield](/plugins/shield), [compress](/plugins/compress).

### `with_state` — Arc config / handles

When middleware needs a cloneable handle (session store, rate-limit buckets):

```rust
use sova_core::{named, with_state}; // or extend::named + root with_state

app.use_middleware(named(
    "session",
    with_state(handle, |handle, mut req, next| async move {
        // load/save session
        next(req).await
    }),
));
```

Used by: [session](/plugins/session), [csrf](/plugins/csrf), [rate-limit](/plugins/rate-limit), [i18n](/plugins/i18n).

### Order

`use_middleware` stacks like an onion — **last registered runs first** on the way in (same as typical tower/axum mental models; verify against your stack if timing-sensitive). Install deps before dependents so their MW is already present.

### Route-scoped MW

Not only global: `router.with(mw)` / `Needs` / passport `guard()` return `MwEntry` for cabinet mounts. See [auth](/plugins/auth) / [passport](/plugins/passport).

### HTML mapping

`extend::map_html` / `after` / `before` / `around` exist for response rewriting. In-tree plugins often inject manually ([meta](/plugins/meta), [devtools](/plugins/devtools)) via `sova_core::html` helpers — use either style; prefer `map_html` for simple transforms.

### Anti-patterns

- Unnamed closures on production plugins (opaque explain output)
- Leaking large or frequently rebuilt configs every request
- Doing I/O in MW without timeouts / `Deadline` when calling outbound HTTP
