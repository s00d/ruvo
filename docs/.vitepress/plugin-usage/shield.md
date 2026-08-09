**`App::web()`** already installs Shield. **Do not** reinstall — duplicate `shield` id fails at `build`. For a custom CSP/stack, build with `App::new()` and install Shield once:

```rust
let mut app = App::new();
app.install(Shield::new() /* builders when you need them */);
```
