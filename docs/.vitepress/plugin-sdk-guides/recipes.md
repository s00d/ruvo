Recipes distilled from in-tree plugins. Full app docs: [Plugins](/plugins/).

### 1. Marker + dependent MW (cookies → session → csrf)

```text
Cookies::install  → state(CookieLayerPresent) + named("cookies")
Session::install  → soft-install cookies if missing
                  → state(SessionStoreHandle)
                  → named("session") + with_state
                  → routes.with(Needs::<CookieLayerPresent>)
Csrf::install     → requires(["session"])
                  → named("csrf") + with_state
```

**Use when:** layered HTTP concerns with hard ordering.

### 2. Leaked security headers (cors / shield)

Builder + toml unset-fill → `named(..., with_leaked(cfg, …))` mutating response headers / answering OPTIONS.

**Use when:** config is immutable after boot; zero Arc churn.

### 3. Pool lifecycle (redis / db)

```text
state(Pool::empty)
on_startup: connect + ping (Err if URL empty)
on_shutdown: clear
register_check: ping
optional: inject_conn MW (db)
optional: register_cli migrate/seed (db)
```

**Use when:** shared network resource must be ready before traffic.

### 4. Library crate + thin facade plugin (store)

`sova-store` is mostly a library (`KvStore`, `Cache`). Facade `SharedStore` plugin id `"store"` only does `app.state(AppStore)`.

**Use when:** most consumers need types without install, but apps want one shared handle.

### 5. Non-plugin helpers (env, sse)

- `sova-env`: call `load()` before building `App`
- `sova-sse`: `app.state(SseChannel::new())` manually; helpers build `Response`

**Use when:** no middleware/registry needed — avoid fake `Plugin` wrappers.

### 6. Background worker (tasks)

```text
state(registry + TaskBackend)
register_check / register_cli
optional POST /_tasks/enqueue (guarded)
service(scheduler)
service(worker)  // wait_shutdown loops
```

Emit `tracing` with `target: "sova.tasks"` if DevTools should show Jobs.

### 7. Soft feature hooks (devtools)

Compile-time `#[cfg(feature = "session")]` etc. After `next`, read extensions if present. Log hook parses known targets. `logger_skip_path` for panel routes. Release builds hard-off unless env escape hatch.

### 8. Outbound client (http)

`state(HttpClient)` + named configs from toml; propagate `request_id`; respect `Deadline` / SSRF guards; structured `http.client` spans for DevTools HTTP tab.

### Choosing a shape

| Need | Shape |
|------|-------|
| Global MW + config | Plugin + `named` / `with_leaked` / `with_state` |
| Shared pool | Plugin + lifecycle + check |
| Pure helpers | Library module, no Plugin |
| Optional integration | Soft `try_state` / Cargo feature |
| Must have dep | `requires` + docs |
