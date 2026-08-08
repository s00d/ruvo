---
title: cookies
editLink: false
---

# `cookies`

**Parse Cookie header into request-local Cookies** · crate `ruvo-cookies` · id `cookies`

```bash
cargo add ruvo --features cookies
```

| Feature | What you get |
|---------|-------------|
| `cookies` | Cookie jar helpers (`ruvo-cookies`). |

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
