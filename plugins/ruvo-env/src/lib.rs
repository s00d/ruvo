//! Explicit `.env` cascade for Ruvo applications.
//!
//! Call [`load`] at the top of `main` before reading configuration.
//! Real process environment variables always win over file values.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvError {
    #[error("required environment variable `{0}` is not set")]
    Missing(&'static str),
    #[error("dotenv: {0}")]
    Dotenv(#[from] dotenvy::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EnvError>;

/// Load the standard cascade into the process environment (no override of existing vars).
pub fn load() -> Result<()> {
    load_from(".")
}

/// Load cascade relative to `root` (typically `"."`).
pub fn load_from(root: impl AsRef<Path>) -> Result<()> {
    let root = root.as_ref();
    let mode = resolve_mode();
    let frozen: std::collections::HashSet<String> =
        std::env::vars().map(|(k, _)| k).collect();

    let files = cascade_files(&mode);
    let mut merged = std::collections::HashMap::new();
    for name in files {
        let path = root.join(name);
        if !path.is_file() {
            continue;
        }
        for item in dotenvy::from_path_iter(&path)? {
            let (k, v) = item?;
            merged.insert(k, v);
        }
    }
    for (k, v) in merged {
        if !frozen.contains(&k) {
            std::env::set_var(&k, v);
        }
    }
    Ok(())
}

fn resolve_mode() -> String {
    if cfg!(test) {
        return "test".into();
    }
    std::env::var("RUVO_ENV")
        .or_else(|_| std::env::var("APP_ENV"))
        .unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "development".into()
            } else {
                "production".into()
            }
        })
}

fn cascade_files(mode: &str) -> Vec<String> {
    let mut out = vec![".env".into()];
    if !cfg!(test) {
        out.push(".env.local".into());
    }
    out.push(format!(".env.{mode}"));
    if !cfg!(test) {
        out.push(format!(".env.{mode}.local"));
    }
    out
}

/// Fail fast when a variable is unset or empty.
pub fn require(name: &'static str) -> Result<String> {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(EnvError::Missing(name)),
    }
}

/// Log a warning for each key listed in `example` that is missing from the environment.
pub fn warn_missing_example(example: impl AsRef<Path>) {
    let path = example.as_ref();
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if std::env::var(key).is_err() {
            tracing::warn!(
                key,
                example = %path.display(),
                "environment variable from example is unset"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn cascade_later_file_overrides_earlier() {
        let _g = lock_env();
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env"), "FOO=base\n").unwrap();
        std::fs::write(dir.path().join(".env.test"), "FOO=mode\n").unwrap();
        std::env::remove_var("FOO");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("FOO").unwrap(), "mode");
        std::env::remove_var("FOO");
    }

    #[test]
    fn real_env_beats_file() {
        let _g = lock_env();
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env"), "BAR=file\n").unwrap();
        std::env::set_var("BAR", "process");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("BAR").unwrap(), "process");
        std::env::remove_var("BAR");
    }

    #[test]
    fn require_errors_clearly() {
        let _g = lock_env();
        std::env::remove_var("MISSING_TEST_VAR_X");
        let err = require("MISSING_TEST_VAR_X").unwrap_err();
        assert!(matches!(err, EnvError::Missing(_)));
    }
}
