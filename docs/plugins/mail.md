---
title: mail
editLink: false
---

# `mail`

**Outbound email via lettre (SMTP / fake / file)** · crate `sova-mail` `0.1.1` · id `mail`

```bash
cargo add sova --features mail,mail-markdown,mail-templates
```

| Feature | What you get |
|---------|-------------|
| `mail` | SMTP / fake / file mailer (`sova_mail`). |
| `mail-markdown` | Markdown mail bodies. |
| `mail-templates` | MiniJinja mail bodies / Mailable views. |

Outbound email for Sova (Express/Nodemailer-simple API on [lettre](https://lettre.rs/)).

 With feature `templates`, render MiniJinja views into the body (Laravel-style):

```rust
 req.mail()
     .to(user)
     .subject("Welcome")
     .view("mail/welcome.html", json!({ "name": name }))
     .send()
     .await?;

 // Mailable
 req.mail().to(user).send_mail(WelcomeMail { name }).await?;

 // Markdown body (feature `markdown`)
 req.mail().to(user).subject("Hi").markdown("# Hello\n\nWorld").send().await?;
 ```

 Layouts use Jinja `{% extends "mail/layout.html" %}` in the template file.

## Usage

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
