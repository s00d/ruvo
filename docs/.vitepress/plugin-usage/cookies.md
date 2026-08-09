Pulled in by **session / csrf / i18n-cookie**. On `App::web()` you already have a cookie jar via sessions:

```rust
use sova::Cookies;

async fn handler(req: Request) -> impl IntoResponse {
    let jar = req.get::<Cookies>().expect("cookies middleware");
    let locale = jar.get("locale");
    // …
}
```

You rarely install `CookieLayer` alone.
