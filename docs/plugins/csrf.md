---
title: csrf
editLink: false
---

# `csrf`

**Session double-submit CSRF (Laravel-style except/XSRF cookie)** · crate `sova-csrf` `0.1.2` · id `csrf`

```bash
cargo add sova --features csrf
```

| Feature | What you get |
|---------|-------------|
| `csrf` | Session double-submit CSRF (`sova_csrf`; needs `session`). |

CSRF protection via session double-submit (Laravel-style).

 Install after sessions. Mutating requests are checked for a matching token in
 (order): `X-CSRF-TOKEN` → `X-XSRF-TOKEN` → query field →
 `application/x-www-form-urlencoded` body field.
 Multipart bodies are left to handlers ([`CsrfExt::verify_csrf`]) unless the
 header/query carries the token.

 With [`Csrf::xsrf_cookie`] (default on), each response also gets a readable
 `XSRF-TOKEN` cookie so SPA clients (axios) can send `X-XSRF-TOKEN`.

## Usage

**`App::web()`** already installs CSRF after sessions. Use the token in forms / SPA bootstrap:

```rust
use sova::prelude::*;
use sova::{CsrfExt, Html, Parser, Request, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Forms")
        .public_url("http://127.0.0.1:3000");

    app.get("/form", |req: Request| async move {
        let token = req.csrf_token();
        Html(format!(
            r#"<form method="post" action="/save">
  <input type="hidden" name="_token" value="{token}" />
  <button>Save</button>
</form>"#
        ))
    });

    app.post("/save", |_req: Request| async move { "ok" });
    app.run().await
}
```

Exempt webhooks / task enqueue when composing beyond the preset:

```rust
let mut app = App::web().site("X").public_url("https://example.com").into_app();
app.install(Csrf::new().except("/_tasks/*").except("/api/webhooks/*"));
```
