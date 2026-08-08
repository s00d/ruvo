---
title: cookies
editLink: false
---

# `cookies`

**Parse Cookie header into request-local Cookies** · crate `sova-cookies` `0.1.0` · id `cookies`

```bash
cargo add sova --features cookies
```

| Feature | What you get |
|---------|-------------|
| `cookies` | Cookie jar helpers (`sova_cookies`). |

Cookie parsing middleware and `Response::cookie` extension.

## Usage

Pulled in by **session / csrf / i18n-cookie**. On `App::web()` you already have a cookie jar via sessions:

```rust
async fn handler(req: Request) -> impl IntoResponse {
    let jar = req.cookies();
    let locale = jar.get("locale");
    // …
}
```

You rarely install `Cookies` alone.
