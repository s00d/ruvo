//! Optional CLI helpers for local development (`--host` / `--port` / `--log-level`).
//!
//! Deployments should prefer [`App::bind`](ruvo_core::App::bind)([`Bind::Env`](ruvo_core::Bind::Env)) (`PORT`/`HOST`).
//! This crate pulls in `clap` — enable only when you want argv parsing.

use ruvo_core::{App, Bind, Error, Result};
use std::net::{IpAddr, SocketAddr};
use tracing_subscriber::EnvFilter;

/// Re-export so `ServerArgs::parse()` works via `use ruvo::{Parser, ServerArgs}`.
pub use clap::Parser;

/// Common server flags for local runs (`clap` derive).
///
/// ```ignore
/// let args = ServerArgs::parse();
/// args.init_tracing();
/// app.listen_args(&args).await?;
/// ```
#[derive(Debug, Clone, Parser)]
#[command(about = "Ruvo HTTP server", disable_help_subcommand = true)]
pub struct ServerArgs {
    /// Bind host (env: `HOST`). Default `0.0.0.0` (IPv4).
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Bind port (env: `PORT`). Default `3000`.
    #[arg(long, env = "PORT", default_value_t = 3000)]
    pub port: u16,

    /// Log filter for `tracing` (env: `RUST_LOG`). Default `ruvo=info`.
    #[arg(long = "log-level", env = "RUST_LOG")]
    pub log_level: Option<String>,
}

impl ServerArgs {
    /// Install a `tracing` subscriber using `--log-level` / `RUST_LOG` / `ruvo=info`.
    pub fn init_tracing(&self) {
        let filter = self
            .log_level
            .clone()
            .or_else(|| std::env::var("RUST_LOG").ok())
            .unwrap_or_else(|| "ruvo=info".into());
        let filter = EnvFilter::try_new(&filter).unwrap_or_else(|_| EnvFilter::new("info"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .try_init();
    }

    /// Resolve `host:port` to a [`SocketAddr`].
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        if let Ok(ip) = self.host.parse::<IpAddr>() {
            return Ok(SocketAddr::new(ip, self.port));
        }
        if let Ok(addr) = self.host.parse::<SocketAddr>() {
            return Ok(addr);
        }
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| Error::Internal(format!("invalid --host/--port: {e}")))
    }
}

/// Listen using parsed [`ServerArgs`].
pub trait ListenArgs {
    fn listen_args(
        self,
        args: &ServerArgs,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl ListenArgs for App {
    async fn listen_args(mut self, args: &ServerArgs) -> Result<()> {
        self.cli_mode(true);
        let addr = args.socket_addr()?;
        self.bind(Bind::Addr(addr)).serve().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let args = ServerArgs::try_parse_from(["ruvo"]).unwrap();
        assert_eq!(args.host, "0.0.0.0");
        assert_eq!(args.port, 3000);
        assert!(args.log_level.is_none());
    }

    #[test]
    fn parses_flags() {
        let args = ServerArgs::try_parse_from([
            "ruvo",
            "--host",
            "127.0.0.1",
            "--port",
            "8080",
            "--log-level",
            "debug",
        ])
        .unwrap();
        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, 8080);
        assert_eq!(args.log_level.as_deref(), Some("debug"));
        assert_eq!(
            args.socket_addr().unwrap(),
            "127.0.0.1:8080".parse().unwrap()
        );
    }
}
