---
title: acme
editLink: false
---

# `acme`

**Let's Encrypt HTTP-01 certificates with TLS hot-reload**

| | |
|--|--|
| Crate | [`sova-acme`](https://docs.rs/sova-acme/0.1.1) `0.1.1` |
| Plugin id | `acme` |
| Category | HTTP |

## Install

```bash
cargo add sova --features acme
```

## Features

| Feature | What you get |
|---------|-------------|
| `acme` | Let's Encrypt HTTP-01 + TLS hot-reload (`Acme`). |

## Overview

**When:** you want HTTPS with Let's Encrypt and automatic renewals without restarting the process.

**Does:**
- HTTP-01 challenges on port 80
- Issues / renews certificates via ACME (`instant-acme`)
- Hot-reloads PEM into the shared [`Tls`](https://docs.rs/sova-core) resolver (`reload_pem`)
- Attaches TLS to the app on `install` (`App::use_tls`) — no separate `.tls(...)` on bind
- Optional redirect of non-challenge HTTP → HTTPS
- Events: `CertificateIssued` / `CertificateRenewed` / `AcmeFailed`
- CLI: `acme status` / `acme renew`

### Example

```rust
app.install(
    Acme::lets_encrypt(["example.com"])
        .email("ops@example.com")
        .dir("./data/acme")
        .hsts(true),
);
app.listen(443).await?;
```

`install` prepares the cert (or a temporary placeholder) and wires HTTPS into the next `listen` / `bind(...).run()`. Use `Acme::lets_encrypt_staging` (or `.staging(true)`) while testing. Port 80 must reach this process.

Advanced: `acme.tls()?` + `.with_tls(tls)` + `bind(...).tls(tls)?` still work if you need a custom bind/`Tls` handle.

## Quick start

```rust
app.install(
    Acme::lets_encrypt(["example.com"])
        .email("ops@example.com")
        .dir("./data/acme")
        .hsts(true),
);
app.listen(443).await?;
```

## Examples

- [`examples/net/acme_hello`](https://github.com/s00d/sova/tree/master/examples/net/acme_hello)

## Related

[`quic`](/plugins/quic)
