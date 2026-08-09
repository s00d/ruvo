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

**`App::web()`** already installs Shield (helmet-style headers). You only reinstall to customize:

```rust
let mut app = App::web().site("App").public_url("https://example.com").into_app();
app.install(Shield::new() /* builders for CSP etc. when you need them */);
```
