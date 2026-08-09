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
