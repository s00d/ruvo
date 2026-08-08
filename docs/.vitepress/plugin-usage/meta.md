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
