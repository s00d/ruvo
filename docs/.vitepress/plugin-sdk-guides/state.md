### App state

```rust
app.state(pool.clone());           // typed insert
let p = app.try_state::<RedisPool>(); // Option during install
```

Handlers and MW retrieve with `req.state::<T>()` / `req.try_state::<T>()` (request carries app `StateMap`).

Patterns:

| Pattern | Example |
|---------|---------|
| Pool handle | [redis](/plugins/redis), [db](/plugins/db) — empty handle at install, connect on startup |
| Client | [mail](/plugins/mail) `MailClient`, [http](/plugins/http) `HttpClient` |
| Registry | [tasks](/plugins/tasks) job map + backend |
| Marker type | [cookies](/plugins/cookies) `CookieLayerPresent` — zero-size proof MW ran |

### Request extensions

Per-request values via `req.set` / `req.get`:

- Parsed cookies, session bag, CSRF token, locale, `CurrentUser`, `MatchedRouteCapture`

Do **not** put process-global singletons only in request extensions.

### Hard vs soft dependencies

**Hard** — `Plugin::requires`:

```rust
fn requires(&self) -> &'static [&'static str] {
    &["session"]
}
```

Missing → error collected at install, reported on `build`.

**Soft** — runtime checks:

```rust
if !app.has_plugin("cookies") {
    app.install(Cookies::new());
}
// or:
if let Some(templates) = app.try_state::<TemplateEngine>() {
    // wire optional integration
}
```

Used by: session→cookies auto-install; mail↔templates; meta↔i18n/store; DevTools feature-gated soft hooks.

### `Needs<T>` — route-level state requirement

Compile/build-time: route declares it needs a marker in app state.

```rust
use sova_core::extend::Needs;
// session routes:
app.with(Needs::<CookieLayerPresent>::default());
```

If cookies plugin never installed its marker, build fails early. Prefer this over silent `try_state` in hot paths.

### Install order checklist

1. Infrastructure: env load (non-plugin), redis/db pools
2. Cookies → session → csrf
3. Auth/passport (needs db+session)
4. Feature plugins (mail, tasks, …)
5. Observability / DevTools last if they soft-read others

### Duplicate ids

Installing the same `id` twice is a logic bug — registry keeps one entry; design plugins to be install-once.
