---
title: http
editLink: false
---

# `http`

**Outbound HTTP client with SSRF guards and named configs**

| | |
|--|--|
| Crate | [`sova-http`](https://docs.rs/sova-http/0.1.1) `0.1.1` |
| Plugin id | `http` |
| Category | Integrations |

## Install

```bash
cargo add sova --features http-client
```

## Features

| Feature | What you get |
|---------|-------------|
| `http-client` | Outbound HTTP client + SSRF guards (`req.http()`). |

## Overview

**When:** call upstream HTTP APIs from handlers (with SSRF guards).

**Does:**
- `OutboundHttp` plugin + `req.http()`
- Named clients / configs, request-bound deadline, tracing
- Fake transport for tests

### Example

```rust
app.install(OutboundHttp::new());
let upstream = req.http().get("https://example.com/api").send().await?;
```

## Quick start

Outbound HTTP client on top of any app:

```rust
let mut app = App::api().title("API").version("1.0").into_app();
app.install(OutboundHttp::new());

async fn proxy(req: Request) -> Result<impl IntoResponse> {
    let client = req.http();
    let upstream = client.get("https://example.com/api").send().await?;
    // …
    Ok(())
}
```

SSRF guards / named configs — see crate docs. Cabinet: `OutboundHttp` + fetch demo module.

## Examples

- [`examples/cabinet`](https://github.com/s00d/sova/tree/master/examples/cabinet)

## Related

[`ai`](/plugins/ai)
