---
title: rate-limit
editLink: false
---

# `rate-limit`

**Per-key request rate limiting**

| | |
|--|--|
| Crate | [`sova-rate-limit`](https://docs.rs/sova-rate-limit/0.1.3) `0.1.3` |
| Plugin id | `rate-limit` |
| Category | HTTP |

## Install

```bash
cargo add sova --features rate-limit
```

## Features

| Feature | What you get |
|---------|-------------|
| `rate-limit` | Fixed-window rate limiting (memory or `KvStore`). |

## Overview

**When:** throttle by IP, user id, or custom key.

**Does:**
- Fixed-window limits via in-memory or `KvStore` backend
- Presets: `per_minute`, `login`, `forgot`, …
- Key strategies: IP / identity / custom `key_fn`

### Example

```rust
use std::time::Duration;
app.install(RateLimit::new(60, Duration::from_secs(60)));
// or shared store:
app.install(RateLimit::fixed_window(store, 120, Duration::from_secs(60)));
```

### Notes
- Multi-instance: use `store` / `redis` + `fixed_window`

## Quick start

In-memory limiter needs no store. Shared (multi-process) uses AppStore:

```rust
use std::time::Duration;

app.install(Db::from_env());
app.install(SharedStore::sql(&app)); // or ::redb("./data/kv.redb") / ::redis(&app) / ::memory()
app.install(
    RateLimit::shared(&app, 120, Duration::from_secs(60)).key(RateLimitKey::Identity),
);
```

Or pass a KvStore explicitly: `RateLimit::fixed_window(store, 120, Duration::from_secs(60))`.

## Related

[`store`](/plugins/store) · [`redis`](/plugins/redis)
