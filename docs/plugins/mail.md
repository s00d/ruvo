---
title: mail
editLink: false
---

# `mail`

**Outbound email via lettre (SMTP / fake / file)**

| | |
|--|--|
| Crate | [`sova-mail`](https://docs.rs/sova-mail/0.1.1) `0.1.1` |
| Plugin id | `mail` |
| Category | Content |

## Install

```bash
cargo add sova --features mail
```

## Features

| Feature | What you get |
|---------|-------------|
| `mail` | SMTP / fake / file mailer (`req.mail()`). |
| `mail-markdown` | Markdown mail bodies. |
| `mail-templates` | MiniJinja mail bodies / Mailable views. |

## Overview

**When:** send SMTP / fake / file email from handlers or jobs.

**Does:**
- `req.mail().to(...).send(Email::…)`
- Markdown + MiniJinja views (`mail-markdown`, `mail-templates`)
- Mailable types

### Example

```rust
app.install(Mail::from_env());
req.mail()
  .to("user@example.com")
  .send(Email::new().subject("Hi").text("Hello"))
  .await?;
```

### Notes
- Install Mail **once**

## Quick start

Outbound mail (facade features `mail` / `mail-templates`). `MailClient::send` takes [`Email`], not a Message type:

```rust
use sova::{Email, Mail, MailExt};

let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .into_app();

app.install(Mail::from_env());

// in a handler / task:
req.mail()
    .to("user@example.com")
    .send(Email::new().subject("Hi").text("Hello"))
    .await?;
```

Install Mail **once** (duplicate `mail` id fails at build). Templates: feature `mail-templates`.

## Examples

- `examples/cabinet`

## Related

[`auth`](/plugins/auth) · [`notifications`](/plugins/notifications) · [`templates`](/plugins/templates)
