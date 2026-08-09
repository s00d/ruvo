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

### Config

```bash
SOVA_MAIL=fake|smtp|file          # or SOVA_MAIL_MAILER
SOVA_MAIL_URL=smtp://…            # or SMTP_URL (required for smtp)
SOVA_MAIL_FROM="App <noreply@example.com>"
SOVA_MAIL_PATH=./mail             # for file mailer
```

```toml
[mail]
from = "App <noreply@example.com>"   # unset-fill if builder/env did not set from
```

Default without URL: fake transport (dev).

## Quick start

Outbound mail (facade features `mail` / `mail-templates`). `MailClient::send` takes `Email`:

```rust
use sova::{Email, Mail, MailExt};

let _ = app.configure_from_path("sova.toml"); // optional [mail] from=
app.install(Mail::from_env());

req.mail()
    .to("user@example.com")
    .send(Email::new().subject("Hi").text("Hello"))
    .await?;
```

```bash
SOVA_MAIL=fake|smtp|file
SOVA_MAIL_URL=smtp://user:pass@localhost:1025   # or SMTP_URL
SOVA_MAIL_FROM="App <noreply@example.com>"
```

```toml
[mail]
from = "App <noreply@example.com>"
```

Install Mail **once**. Templates: `mail-templates` / markdown: `mail-markdown`.

## Examples

- `examples/cabinet`

## Related

[`auth`](/plugins/auth) · [`notifications`](/plugins/notifications) · [`templates`](/plugins/templates)
