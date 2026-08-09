---
title: Checks & CLI
editLink: false
---

# Checks & CLI

register_check vs register_audit, probes, register_cli commands.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/checks-cli.md`, then `pnpm docs:generate`.

### Ready checks vs audits

| API | Severity | Typical use |
|-----|----------|-------------|
| `register_check` | Blocks ready / `check` CLI | DB ping, Redis PING, session store reachable |
| `register_audit` | Advisory | OpenAPI coverage, schedule sanity, meta completeness |
| `with_probes` | HTTP surface | Expose `/ready` (and related) |

```rust
app.register_check("redis", |state| {
    Box::pin(async move {
        let pool = state.get::<RedisPool>().ok_or("no redis")?;
        pool.ping().await.map_err(|e| e.to_string())?;
        Ok(())
    })
});
```

Session, redis, db, tasks all register checks. Prefer checks for “cannot serve traffic”; audits for “misconfigured but process can start”.

### CLI commands

```rust
app.register_cli("migrate", |args, state| {
    Box::pin(async move {
        // sea-orm migrator, etc.
        Ok(())
    })
});
```

Patterns:

- [db](/plugins/db) — `migrate`, optional `seed`
- [tasks](/plugins/tasks) — task admin commands
- Auth migrators via db plugin builders

CLI shares `StateMap` with the server after startup hooks appropriate to the command.

### Documenting commands

Mention in plugin guide + usage snippet. Users discover via `myapp --help` / sovax docs ([cargo sovax](/guide/cargo-sovax)).
