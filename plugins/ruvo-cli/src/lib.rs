//! Optional CLI helpers for local development (`--log-level`).
//! This crate pulls in `clap` — enable only when you want argv parsing.

use tracing_subscriber::EnvFilter;

/// Re-export so `ServerArgs::parse()` works via `use ruvo::{Parser, ServerArgs}`.
pub use clap::Parser;

/// Common server flags for local runs (`clap` derive).
///
/// ```ignore
/// let args = ServerArgs::parse();
/// args.init_tracing();
/// app.run().await?;
/// ```
#[derive(Debug, Clone, Parser)]
#[command(about = "Ruvo HTTP server", disable_help_subcommand = true)]
pub struct ServerArgs {
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

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let args = ServerArgs::try_parse_from(["ruvo"]).unwrap();
        assert!(args.log_level.is_none());
    }

    #[test]
    fn parses_flags() {
        let args = ServerArgs::try_parse_from([
            "ruvo",
            "--log-level",
            "debug",
        ])
        .unwrap();
        assert_eq!(args.log_level.as_deref(), Some("debug"));
    }
}
