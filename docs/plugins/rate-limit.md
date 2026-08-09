---
title: rate-limit
editLink: false
---

# `rate-limit`

**Per-key request rate limiting**

| | |
|--|--|
| Crate | [`sova-rate-limit`](https://docs.rs/sova-rate-limit/0.1.1) `0.1.1` |
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

Needs a `KvStore`. Typical cabinet-style wiring on the web preset:

```rust
use std::sync::Arc;
use std::time::Duration;

let mut app = App::web().site("App").public_url("https://example.com").into_app();
app.install(Db::from_env());
let pool = app.try_state::<DbPool>().expect("db").as_ref().clone();
let kv = Arc::new(sova::store::Sql::from_db_pool(&pool)) as Arc<dyn sova::KvStore>;

app.install(
    RateLimit::fixed_window(
        Arc::new(namespace(Arc::clone(&kv), "rl")),
        120,
        Duration::from_secs(60),
    )
    .key(RateLimitKey::Identity),
);
```

## Related

[`store`](/plugins/store) · [`redis`](/plugins/redis)
