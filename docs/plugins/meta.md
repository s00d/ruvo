---
title: meta
editLink: false
---

# `meta`

**Document meta, OG/Twitter, JSON-LD, and head inject**

| | |
|--|--|
| Crate | [`sova-meta`](https://docs.rs/sova-meta/0.1.2) `0.1.2` |
| Plugin id | `meta` |
| Category | Content |

## Install

```bash
cargo add sova --features meta
```

## Features

| Feature | What you get |
|---------|-------------|
| `meta` | SEO head tags, Sitemap, Robots. |
| `meta-i18n` | Locale-aware meta. |
| `meta-openapi` | OpenAPI helpers for Meta routes. |
| `meta-store` | Meta helpers backed by `KvStore`. |
| `meta-templates` | Inject meta into MiniJinja HTML. |

## Overview

**When:** document title, description, OG/Twitter, JSON-LD, head inject.

**Does:**
- `Meta::page().title(…).description(…)`
- Route/router `.with(Meta::…)`
- Sitemap / robots helpers in crate

### Example

```rust
app.get("/", home).with(Meta::page().title("Home").description("Welcome"));
```

## Quick start

**`App::web()`** installs Meta + Sitemap + Robots. Set site / public URL on the preset, page tags on routes:

```rust
use sova::prelude::*;
use sova::{Html, Meta, Parser, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Blog")
        .public_url("https://example.com");

    app.get("/about", || async { Html("<h1>About</h1>") })
        .with(
            Meta::page()
                .title("About")
                .description("About the blog"),
        );

    app.run().await
}
```

Customize sitemap exclusions after `into_app()` if needed (cabinet excludes `/cabinet/*`, `/api/*`, …). Demo: `meta_blog`.

## Examples

- `examples/web/meta_blog`

## Related

[`templates`](/plugins/templates) · [`i18n`](/plugins/i18n) · [`openapi`](/plugins/openapi)
