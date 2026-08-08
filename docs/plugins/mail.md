---
title: mail
editLink: false
---

# `mail`

**Outbound email via lettre (SMTP / fake / file)** · crate `ruvo-mail` · id `mail`

```bash
cargo add ruvo --features mail,mail-markdown,mail-templates
```

| Feature | What you get |
|---------|-------------|
| `mail` | SMTP / fake / file mailer (`ruvo-mail`). |
| `mail-markdown` | Markdown mail bodies. |
| `mail-templates` | MiniJinja mail bodies / Mailable views. |

Outbound email for Ruvo (Express/Nodemailer-simple API on [lettre](https://lettre.rs/)).

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

Outbound mail next to Fortify / tasks on a real app:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .into_app();

let mail_plugin = Mail::from_env();
let mail = mail_plugin.client();
app.install(mail_plugin);

// later in a task / handler:
mail.send(/* Message */).await?;
```

Templates: `mail-templates`. Cabinet sends welcome mail from a Tasks job after register.
