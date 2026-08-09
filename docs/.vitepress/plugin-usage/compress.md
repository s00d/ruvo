```rust
use sova::{App, Compress};

let mut app = App::api().title("API").version("1.0").into_app();
app.install(
    Compress::new()
        .threshold(1024) // skip tiny bodies
        .level(6),
);
```

Negotiates `br` / `gzip` / `deflate` from `Accept-Encoding`. Bodies are buffered then compressed.
