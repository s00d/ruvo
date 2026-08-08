---
title: meta
editLink: false
---

# `meta`

**Document meta, OG/Twitter, JSON-LD, and head inject** · crate `sova-meta` `0.1.0` · id `sitemap`

```bash
cargo add sova --features meta,meta-i18n,meta-store,meta-templates
```

| Feature | What you get |
|---------|-------------|
| `meta` | SEO head tags, Sitemap, Robots (`sova_meta`). |
| `meta-i18n` | Locale-aware meta. |
| `meta-store` | Meta helpers backed by KvStore. |
| `meta-templates` | Inject meta into MiniJinja HTML. |

Document meta, OG/Twitter, JSON-LD, sitemap and robots for Sova.

## Usage

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
