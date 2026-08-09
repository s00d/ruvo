Construct a backend, pass into `Tasks` (soft-wire helpers prefer pool from state):

```rust
use sova::{Db, Tasks};

app.install(Db::from_env());
app.install(Tasks::sql(&app).queues(["default"]).job(/* … */));

// or:
// app.install(Redis::from_env());
// app.install(Tasks::redis(&app).job(/* … */));
// app.install(Tasks::memory().job(/* … */));
```

Advanced (custom store):

```rust
use std::sync::Arc;
let pool = app.try_state::<sova::DbPool>().unwrap().as_ref().clone();
app.install(Tasks::new(Arc::new(sova::tasks::Sql::from_db_pool(&pool))).job(/* … */));
```

Features: `tasks-store`, `tasks-file`, `tasks-sql`, `tasks-redis`.
