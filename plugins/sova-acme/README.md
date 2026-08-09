[![crates.io](https://img.shields.io/crates/v/sova-acme?style=for-the-badge)](https://crates.io/crates/sova-acme)
[![docs.rs](https://img.shields.io/docsrs/sova-acme?style=for-the-badge)](https://docs.rs/sova-acme)
[![License](https://img.shields.io/crates/l/sova-acme?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)

# sova-acme

Let's Encrypt **HTTP-01** certificates for Sova with **TLS hot-reload** (no process restart).

**Guide:** [https://s00d.github.io/sova/plugins/acme](https://s00d.github.io/sova/plugins/acme)

```bash
cargo add sova --features "tls,acme"
```

```rust
use sova::{Acme, App, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let acme = Acme::lets_encrypt(["example.com"])
        .email("ops@example.com")
        .dir("./data/acme");
    let tls = acme.tls()?; // existing cert or temporary self-signed
    let mut app = App::new();
    app.get("/", || async { "hello https" });
    app.install(acme.with_tls(tls.clone()));
    app.bind("0.0.0.0:443").tls(tls.hsts(true))?.run().await
}
```

Port **80** must be reachable for HTTP-01. The plugin listens there for challenges and (by default) redirects other traffic to HTTPS. Use `.staging(true)` / `Acme::lets_encrypt_staging` while testing.

CLI: `acme status` / `acme renew` (force). Events: `CertificateIssued`, `CertificateRenewed`, `AcmeFailed`.

## License

MIT — see [LICENSE](LICENSE).
