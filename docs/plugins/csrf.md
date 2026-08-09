---
title: csrf
editLink: false
---

# `csrf`

**Session double-submit CSRF (Laravel-style except/XSRF cookie)**

| | |
|--|--|
| Crate | [`sova-csrf`](https://docs.rs/sova-csrf/0.1.2) `0.1.2` |
| Plugin id | `csrf` |
| Category | HTTP |

## Install

```bash
cargo add sova --features csrf
```

## Features

| Feature | What you get |
|---------|-------------|
| `csrf` | Double-submit CSRF + XSRF cookie (needs `session`). |

## Overview

**When:** cookie-session web apps (Laravel-style double-submit). Already on `App::web()`.

**Does:**
- Issues CSRF / XSRF cookies
- Validates mutating requests
- Except paths for APIs / webhooks

### Example

```rust
app.install(Csrf::new().except(["/webhooks/*"]));
```

### Notes
- Needs session middleware

## Quick start

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
  <input type="hidden" name="csrf" value="{token}" />
  <button>Save</button>
</form>"#
        ))
    });

    app.post("/save", |_req: Request| async move { "ok" });
    app.run().await
}
```

Default form field name is **`csrf`** (not `_token`). Override with `Csrf::new().field("_token")` if you need Laravel-style names.

Exempt paths without a second CSRF layer: `App::web()` already installs CSRF — reinstalling stacks another middleware and does not replace the preset. Prefer toml / a custom app without the web preset, or verify exemptions against the single installed layer:

```rust
// Custom stack (no App::web CSRF):
let mut app = App::new();
app.install(memory_sessions());
app.install(Csrf::new().except("/_tasks/*").except("/api/webhooks/*"));
```

## Related

[`session`](/plugins/session) · [`cookies`](/plugins/cookies)
