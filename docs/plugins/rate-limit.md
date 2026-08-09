---
title: rate-limit
editLink: false
---

# `rate-limit`

**Per-key request rate limiting** · crate `sova-rate-limit` `0.1.1` · id `rate-limit`

```bash
cargo add sova --features rate-limit
```

| Feature | What you get |
|---------|-------------|
| `rate-limit` | Fixed-window rate limiting (`sova-rate-limit`). |

Rate limiting for Sova (Express [`express-rate-limit`](https://www.npmjs.com/package/express-rate-limit)-style).

## Usage

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
