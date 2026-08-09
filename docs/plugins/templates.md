---
title: templates
editLink: false
---

# `templates`

**MiniJinja HTML templates with optional autoreload**

| | |
|--|--|
| Crate | [`sova-templates`](https://docs.rs/sova-templates/0.1.1) `0.1.1` |
| Plugin id | `templates` |
| Category | Content |

## Install

```bash
cargo add sova --features templates
```

## Features

| Feature | What you get |
|---------|-------------|
| `templates` | MiniJinja HTML templates (`req.render`). |

## Overview

**When:** MiniJinja HTML views. Already on `App::web()`.

**Does:**
- Render templates from `views/`
- Optional autoreload in dev
- Shared with mail templates feature

### Example

```rust
app.install(Templates::new("views"));
Ok(req.render("home.html", json!({ "title": "Hi" }))?)
```

## Quick start

If `views/` exists, **`App::web()`** already installs MiniJinja. Handlers only render:

```rust
use sova::prelude::*;
use sova::{Meta, Parser, RenderExt, Request, Response, ServerArgs};
use serde::Serialize;

#[derive(Serialize)]
struct Page {
    title: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Blog")
        .public_url("http://127.0.0.1:3000")
        .views("views");

    app.get("/", home).with(Meta::page().title("Home"));
    app.run().await
}

async fn home(req: Request) -> Response {
    req.render(
        "home.html",
        Page {
            title: "Sova".into(),
        },
    )
    .unwrap_or_else(|e| e.into_response())
}
```

Extra per-request helpers after the preset: `sova_templates::register_per_request(&mut app, "t", sova::template_fn)`. Do not reinstall Templates on top of `App::web()` (duplicate id). Demos: `templates`, `templates_i18n`.

## Examples

- `examples/web/templates`

## Related

[`mail`](/plugins/mail) · [`meta`](/plugins/meta) · [`i18n`](/plugins/i18n)
