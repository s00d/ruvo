//! Optional CLI helpers for local development (`--log-level`, file logging).
//! This crate pulls in `clap` — enable only when you want argv parsing.

use ruvo_core::{parse_log_rotate, LogConfig, LogRotate};
use std::path::PathBuf;

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

    /// Write logs to this file (env: `RUVO_LOG_FILE`).
    #[arg(long = "log-file", env = "RUVO_LOG_FILE")]
    pub log_file: Option<PathBuf>,

    /// Also write logs to stdout (env: `RUVO_LOG_STDOUT`). Default true.
    #[arg(
        long = "log-stdout",
        env = "RUVO_LOG_STDOUT",
        default_value_t = true,
        action = clap::ArgAction::Set
    )]
    pub log_stdout: bool,

    /// File rotation: `size`, `daily`, or `never` (env: `RUVO_LOG_ROTATE`).
    #[arg(long = "log-rotate", env = "RUVO_LOG_ROTATE", default_value = "size")]
    pub log_rotate: String,

    /// Max size before rotate when `--log-rotate=size` (env: `RUVO_LOG_ROTATE_SIZE`).
    #[arg(long = "log-rotate-size", env = "RUVO_LOG_ROTATE_SIZE", default_value = "10MB")]
    pub log_rotate_size: String,

    /// How many rotated archives to keep (env: `RUVO_LOG_ROTATE_KEEP`).
    #[arg(long = "log-rotate-keep", env = "RUVO_LOG_ROTATE_KEEP", default_value_t = 5)]
    pub log_rotate_keep: usize,

    /// Forwarded to `App::run` CLI (`migrate`, `check`, `routes`, …).
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    pub trailing: Vec<String>,
}

impl ServerArgs {
    /// Build [`LogConfig`] from CLI flags (with sensible defaults).
    pub fn log_config(&self) -> Result<LogConfig, String> {
        let filter = self
            .log_level
            .clone()
            .or_else(|| std::env::var("RUST_LOG").ok())
            .unwrap_or_else(|| "ruvo=info".into());

        let rotate = parse_log_rotate(
            &self.log_rotate,
            Some(self.log_rotate_size.as_str()),
            Some(self.log_rotate_keep),
        )?;

        Ok(LogConfig {
            filter,
            stdout: self.log_stdout,
            file: self.log_file.clone(),
            rotate: if self.log_file.is_some() {
                rotate
            } else {
                LogRotate::default()
            },
        })
    }

    /// Install a `tracing` subscriber using CLI / env (`RUVO_LOG=off` skips).
    pub fn init_tracing(&self) {
        match self.log_config() {
            Ok(cfg) => cfg.install(),
            Err(e) => {
                eprintln!("ruvo: log config error: {e}; falling back to defaults");
                LogConfig::from_env().install();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let args = ServerArgs::try_parse_from(["ruvo"]).unwrap();
        assert!(args.log_level.is_none());
        assert!(args.log_file.is_none());
        assert!(args.log_stdout);
        assert_eq!(args.log_rotate, "size");
        assert_eq!(args.log_rotate_keep, 5);
        assert!(args.trailing.is_empty());
    }

    #[test]
    fn parses_trailing_cli_command() {
        let args = ServerArgs::try_parse_from(["ruvo", "migrate", "status"]).unwrap();
        assert_eq!(args.trailing, vec!["migrate", "status"]);
    }

    #[test]
    fn parses_log_file_flags() {
        let args = ServerArgs::try_parse_from([
            "ruvo",
            "--log-level",
            "debug",
            "--log-file",
            "logs/app.log",
            "--log-stdout",
            "false",
            "--log-rotate",
            "daily",
            "--log-rotate-keep",
            "3",
        ])
        .unwrap();
        assert_eq!(args.log_level.as_deref(), Some("debug"));
        assert_eq!(
            args.log_file.as_deref(),
            Some(std::path::Path::new("logs/app.log"))
        );
        assert!(!args.log_stdout);
        assert_eq!(args.log_rotate, "daily");
        assert_eq!(args.log_rotate_keep, 3);

        let cfg = args.log_config().unwrap();
        assert!(!cfg.stdout);
        assert!(cfg.file.is_some());
        assert_eq!(cfg.rotate, LogRotate::Daily { keep: 3 });
    }
}
