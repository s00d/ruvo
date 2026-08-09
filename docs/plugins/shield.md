---
title: shield
editLink: false
---

# `shield`

**Baseline security response headers (helmet-style)** · crate `sova-shield` `0.1.1` · id `shield`

```bash
cargo add sova --features shield
```

| Feature | What you get |
|---------|-------------|
| `shield` | Security response headers (`sova_shield`). |

Security response headers (helmet-style subset). HSTS stays on [`sova_core::Tls`].

## Usage

**`App::web()`** already installs Shield. **Do not** reinstall — duplicate `shield` id fails at `build`. For a custom CSP/stack, build with `App::new()` and install Shield once:

```rust
let mut app = App::new();
app.install(Shield::new() /* builders when you need them */);
```
