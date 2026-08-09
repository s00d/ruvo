**When:** you want HTTPS with Let's Encrypt and automatic renewals without restarting the process.

**Does:**
- HTTP-01 challenges on port 80
- Issues / renews certificates via ACME (`instant-acme`)
- Hot-reloads PEM into the shared [`Tls`](https://docs.rs/sova-core) resolver (`reload_pem`)
- Optional redirect of non-challenge HTTP → HTTPS
- Events: `CertificateIssued` / `CertificateRenewed` / `AcmeFailed`
- CLI: `acme status` / `acme renew`

### Example

```rust
let acme = Acme::lets_encrypt(["example.com"])
    .email("ops@example.com")
    .dir("./data/acme");
let tls = acme.tls()?;
app.install(acme.with_tls(tls.clone()));
app.bind("0.0.0.0:443").tls(tls.hsts(true))?.run().await?;
```

Use `Acme::lets_encrypt_staging` (or `.staging(true)`) while testing. Port 80 must reach this process.
