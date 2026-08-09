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
