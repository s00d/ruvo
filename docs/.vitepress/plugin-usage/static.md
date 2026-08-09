`App::web()` already mounts `public/` → `/assets`. Extra mount:

```rust
use sova::{App, Static};
use std::time::Duration;

app.install(
    Static::new("/static", "assets")
        .max_age(Duration::from_secs(86_400))
        .immutable(true),
);
```

Dotfiles denied by default. See [`examples/web/static_files`](https://github.com/s00d/sova/tree/master/examples/web/static_files).
