---
title: http
editLink: false
---

# `http`

**Outbound HTTP client with SSRF guards and named configs** · crate `ruvo-http` · id `http`

```bash
cargo add ruvo --features http-client
```

| Feature | What you get |
|---------|-------------|
| `http-client` | Outbound HTTP (`ruvo-http`). |

Outbound HTTP client for Ruvo — request-bound deadline, tracing, fake transport.

## Usage

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
