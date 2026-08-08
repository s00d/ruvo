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
