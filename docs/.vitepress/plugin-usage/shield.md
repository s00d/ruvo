**`App::web()`** already installs Shield (helmet-style headers). You only reinstall to customize:

```rust
let mut app = App::web().site("App").public_url("https://example.com").into_app();
app.install(Shield::new() /* builders for CSP etc. when you need them */);
```
