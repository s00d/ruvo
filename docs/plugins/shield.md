---
title: shield
editLink: false
---

# `shield`

**Baseline security response headers (helmet-style)**

| | |
|--|--|
| Crate | [`sova-shield`](https://docs.rs/sova-shield/0.1.1) `0.1.1` |
| Plugin id | `shield` |
| Category | HTTP |

## Install

```bash
cargo add sova --features shield
```

## Features

| Feature | What you get |
|---------|-------------|
| `shield` | Helmet-style security response headers. |

## Overview

**When:** baseline browser security headers (helmet-style). Already on `App::web()`.

**Does:**
- Sets `X-Frame-Options`, `X-Content-Type-Options`, `Referrer-Policy`, COOP/CORP, etc.
- Optional CSP via builder
- HSTS stays on TLS (`sova_core::Tls`), not here

### Example

```rust
app.install(Shield::new().frame("DENY"));
```

### Notes
- Install **once** — duplicate `shield` id fails at build
- `App::web()` already installs Shield; do not reinstall

## Quick start

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

## Related

[`cors`](/plugins/cors) · [`csrf`](/plugins/csrf)
