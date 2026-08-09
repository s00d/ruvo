---
title: Errors
editLink: false
---

# Errors

Startup Err vs panic, ErrorResponse, soft degradation.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/errors.md`, then `pnpm docs:generate`.

### Prefer `Result` over panic in `install`

| Situation | Prefer |
|-----------|--------|
| Missing `DATABASE_URL` / `REDIS_URL` | `on_startup` → `Err("…")` so process exits cleanly |
| Invalid SMTP config that must never run | Documented panic **or** startup Err (mail `SmtpBuilder` historically panics — avoid copying blindly) |
| Optional feature unavailable | Soft-skip + `tracing::debug!` |

### `ErrorResponse`

```rust
use sova_core::extend::ErrorResponse;
```

Typed errors that convert to HTTP responses — used by http client, db, storage, vld. Implement for domain errors your handlers/`?` use.

### Middleware errors

Returning a `Response` short-circuit is fine (401/429). Panicking in MW takes down the connection — only for truly invariant bugs.

### SDK / dependency failures

- `requires` missing → build error listing ids
- SDK major mismatch → build error
- Soft dep missing → degrade features, do not panic

### User-facing messages

Keep startup errors actionable (`redis: empty url — set REDIS_URL or [redis] url`). Log with `tracing` targets your plugin owns (`sova_redis`, …).
