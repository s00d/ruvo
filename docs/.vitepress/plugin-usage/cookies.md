Pulled in by **session / csrf / i18n-cookie**. On `App::web()` you already have a cookie jar via sessions:

```rust
async fn handler(req: Request) -> impl IntoResponse {
    let jar = req.cookies();
    let locale = jar.get("locale");
    // …
}
```

You rarely install `Cookies` alone.
