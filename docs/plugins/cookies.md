---
title: cookies
editLink: false
---

# `cookies`

**Parse Cookie header into request-local Cookies**

| | |
|--|--|
| Crate | [`sova-cookies`](https://docs.rs/sova-cookies/0.1.1) `0.1.1` |
| Plugin id | `cookies` |
| Category | HTTP |

## Install

```bash
cargo add sova --features cookies
```

## Features

| Feature | What you get |
|---------|-------------|
| `cookies` | Parse `Cookie` header → `req.cookies()` + set-cookie helpers. |

## Overview

**When:** read `Cookie` header or set cookies on responses.

**Does:**
- Parses cookies into request-local `Cookies`
- `req.cookies().get("name")`
- `Response::cookie(...)` helpers

### Example

```rust
app.install(CookieLayer);
let theme = req.get::<Cookies>().and_then(|c| c.get("theme"));
```

### Notes
- Sessions need cookies; prefer `session` plugin for signed session cookies

## Quick start

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

## Related

[`session`](/plugins/session) · [`csrf`](/plugins/csrf)
