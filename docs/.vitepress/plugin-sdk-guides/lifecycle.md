### Startup / shutdown hooks

```rust
app.on_startup(|state| {
    Box::pin(async move {
        let pool = state.get::<RedisPool>().ok_or("redis missing")?;
        pool.connect().await.map_err(|e| e.to_string())?;
        Ok(())
    })
});

app.on_shutdown(|state| {
    Box::pin(async move {
        if let Some(pool) = state.get::<RedisPool>() {
            pool.clear().await;
        }
        Ok(())
    })
});
```

**Pool pattern** ([redis](/plugins/redis), [db](/plugins/db)):

1. `state(empty_pool)` at install
2. Connect + ping on startup (fail → process does not serve)
3. Clear on shutdown
4. `register_check` for `/ready` (PING / SELECT 1)

Empty URL → register startup that returns `Err(…)` (fail-fast) instead of panicking in `install`.

### Background services

```rust
app.service(MyWorker { /* … */ });
```

Implement `BackgroundService`: run until shutdown. Inside loops use:

```rust
use sova_core::extend::wait_shutdown;

loop {
    tokio::select! {
        _ = wait_shutdown(&shutdown) => break,
        _ = work_once() => {}
    }
}
```

Used by: [tasks](/plugins/tasks) worker + scheduler, [quic](/plugins/quic), [udp](/plugins/udp).

### CLI mode

When the binary runs a CLI subcommand, avoid starting workers unless intentional (`service_in_cli`). Still register CLI commands and checks.

### Soft wiring after other plugins

Mail may `on_startup` attach templates if `try_state` finds the engine — optional integration without `requires`.
