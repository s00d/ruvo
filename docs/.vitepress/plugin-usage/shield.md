**`App::web()`** already installs Shield. **Do not** reinstall — duplicate `shield` id fails at build.

Custom stack:

```rust
use sova::{App, Shield};

let mut app = App::new();
app.install(
    Shield::new()
        .frame("DENY")
        // .csp("default-src 'self'") when you need CSP
);
```
