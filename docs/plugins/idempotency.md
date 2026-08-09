---
title: idempotency
editLink: false
---

# `idempotency`

**Replay 2xx responses for Idempotency-Key on mutating methods**

| | |
|--|--|
| Crate | [`sova-idempotency`](https://docs.rs/sova-idempotency/0.1.1) `0.1.1` |
| Plugin id | `idempotency` |
| Category | HTTP |

## Install

```bash
cargo add sova --features idempotency
```

## Features

| Feature | What you get |
|---------|-------------|
| `idempotency` | Inbound `Idempotency-Key` replay for mutating methods. |

## Overview

**When:** clients retry POST/PUT/PATCH/DELETE with the same `Idempotency-Key`.

**Does:**
- On cache hit → replay status + body + content-type (`X-Idempotency-Replay: true`)
- On miss → run handler; store 2xx buffered bodies (size cap) with TTL (default 24h)

### Example

```rust
use std::sync::Arc;
use sova::{AppStore, Idempotency, KvStore};

let store = AppStore::memory();
app.install(Idempotency::from_store(Arc::clone(&store.inner)));
// Client: Idempotency-Key: <unique>
```

### Notes
- Needs feature `idempotency` (+ `store`)
- Only mutating methods; missing header → pass-through

## Quick start

```rust
app.install(SharedStore::memory());
app.install(Idempotency::from_app(&app).ttl(std::time::Duration::from_secs(3600)));
```

Or `Idempotency::from_store(kv)` with an explicit [`KvStore`].

## Related

[`store`](/plugins/store)
