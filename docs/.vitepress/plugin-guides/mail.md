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
