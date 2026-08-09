---
title: response-cache
editLink: false
---

# `response-cache`

**Cache GET 200 responses in KvStore**

| | |
|--|--|
| Crate | [`sova-response-cache`](https://docs.rs/sova-response-cache/0.1.1) `0.1.1` |
| Plugin id | `response-cache` |
| Category | HTTP |

## Install

```bash
cargo add sova --features response-cache
```

## Features

| Feature | What you get |
|---------|-------------|
| `response-cache` | Cache public GET 200 responses in `KvStore`. |

## Overview

**When:** cache public GET 200 responses (lists, marketing pages).

**Does:**
- Skips requests with `Authorization` / `Cookie` unless `cache_private(true)`
- Key = method + path + sorted `Vary` headers
- Sets `X-Cache: HIT|MISS` and `Cache-Control`
- Invalidate via `ResponseCacheHandle::invalidate_prefix` (e.g. from an `EventBus` listener)

### Example

```rust
use std::sync::Arc;
use std::time::Duration;
use sova::{Memory, ResponseCache};

let store = Arc::new(Memory::new());
app.install(ResponseCache::new(store).ttl(Duration::from_secs(60)).vary(&["accept-language"]));
```

### Notes
- Feature `response-cache` (+ `store`)
- Does not replace static file ETag — dynamic routes only

## Quick start

```rust
use sova::store::Memory;
use sova::ResponseCache;
use std::sync::Arc;

app.install(ResponseCache::new(Arc::new(Memory::new())));
// From a handler / event listener:
// req.state::<ResponseCacheHandle>().invalidate_prefix("/api/notes").await;
```

## Related

[`store`](/plugins/store) · [`idempotency`](/plugins/idempotency)
