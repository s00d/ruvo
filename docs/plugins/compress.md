---
title: compress
editLink: false
---

# `compress`

**gzip / deflate / brotli response compression**

| | |
|--|--|
| Crate | [`sova-compress`](https://docs.rs/sova-compress/0.1.1) `0.1.1` |
| Plugin id | `compress` |
| Category | HTTP |

## Install

```bash
cargo add sova --features compress
```

## Features

| Feature | What you get |
|---------|-------------|
| `compress` | gzip / deflate / brotli response compression. |

## Overview

**When:** shrink HTML/JSON/static responses (gzip / deflate / brotli).

**Does:**
- Negotiates encoding from `Accept-Encoding`
- Buffered body compression (not chunk-streamed)
- Threshold + level + custom filter

### Example

```rust
app.install(Compress::new().threshold(1024).level(6));
```

## Quick start

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

## Related

[`static`](/plugins/static)
