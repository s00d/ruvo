//! ACME / Let's Encrypt staging demo.
//!
//! ```bash
//! ACME_DOMAINS=example.com ACME_EMAIL=ops@example.com \
//!   cargo run -p acme_hello
//! # HTTPS :443 + HTTP-01 :80 (needs public DNS → this host)
//! ```

use sova::{Acme, App, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let domains: Vec<String> = std::env::var("ACME_DOMAINS")
        .unwrap_or_else(|_| "localhost".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let email = std::env::var("ACME_EMAIL").unwrap_or_else(|_| "admin@example.com".into());
    let dir = std::env::var("ACME_DIR").unwrap_or_else(|_| "./data/acme".into());
    let https_port: u16 = std::env::var("HTTPS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(443);
    let http_port: u16 = std::env::var("HTTP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);

    let acme = Acme::lets_encrypt_staging(domains)
        .email(email)
        .dir(dir)
        .http_port(http_port)
        .https_port(https_port);
    let tls = acme.tls()?;

    let mut app = App::new();
    app.get("/", || async { "hello acme" });
    app.install(acme.with_tls(tls.clone()));

    app.bind(("0.0.0.0", https_port))
        .tls(tls.hsts(true))?
        .run()
        .await
}
