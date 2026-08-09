Parse cookies on every request; set cookies on the response:

```rust
use sova::{App, CookieBuilder, CookieLayer, Cookies, ResponseCookieExt};

let mut app = App::new();
app.install(CookieLayer);

app.get("/", |req| async move {
    let theme = req
        .get::<Cookies>()
        .and_then(|c| c.get("theme").map(str::to_owned))
        .unwrap_or_else(|| "light".into());
    Ok(sova::Response::text(theme).cookie(CookieBuilder::new("theme", "dark")))
});
```

Sessions use cookies under the hood — prefer [session](/plugins/session) for signed session cookies.
