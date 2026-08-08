DB inbox (+ optional WS/mail) on a full app — cabinet is the reference:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .into_app();

app.install(Db::from_env().migrations::<CabinetMigrator>());
app.install(Ws::new());
app.install(Notifications::new() /* feature flags: ws / mail / auth / templates */);
```

Features: `notifications-ws`, `notifications-mail`, `notifications-auth`, `notifications-templates`.
