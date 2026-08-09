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
