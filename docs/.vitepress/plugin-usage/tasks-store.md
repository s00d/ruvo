Construct a backend, pass into `Tasks::new`:

```rust
use std::sync::Arc;
use sova::{Db, Redis, Tasks};

app.install(Db::from_env());
app.install(Redis::from_env());

let pool = app.try_state::<sova::DbPool>().unwrap().as_ref().clone();
// let rpool = app.try_state::<sova::RedisPool>().unwrap().as_ref().clone();

let store = Arc::new(sova::tasks::Sql::from_db_pool(&pool));
// let store = Arc::new(sova::tasks::Redis::from_redis_pool(&rpool));
// let store = Arc::new(sova::tasks::Memory::new());
// let store = Arc::new(sova::tasks::File::open("data/tasks").await?);

app.install(Tasks::new(store).queues(["default"]).job(/* … */));
```

Features: `tasks-store`, `tasks-file`, `tasks-sql`, `tasks-redis`.
