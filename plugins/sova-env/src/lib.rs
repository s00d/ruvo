//! Explicit `.env` cascade for Sova applications.
//!
//! Call [`load`] at the top of `main` before reading configuration.
//! Real process environment variables always win over file values.
//!
//! File order (later overrides earlier):
//! 1. `.env.{dev|prod|test}` (short alias of the active mode)
//! 2. `.env.{mode}` when mode is the long name (`development` / `production`)
//! 3. `.env.local` (skipped in `test`)
//! 4. `.env` (final overlay)

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
    let frozen: std::collections::HashSet<String> = std::env::vars().map(|(k, _)| k).collect();

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
    std::env::var("SOVA_ENV")
        .or_else(|_| std::env::var("APP_ENV"))
        .unwrap_or_else(|_| {
            // Unit-test builds of this crate default to `test`; otherwise debug → development.
            if cfg!(test) {
                "test".into()
            } else if cfg!(debug_assertions) {
                "development".into()
            } else {
                "production".into()
            }
        })
}

/// Short file suffix for mode (`development` → `dev`).
fn mode_short(mode: &str) -> &str {
    match mode {
        "development" | "debug" => "dev",
        "production" | "release" => "prod",
        "test" => "test",
        other => other,
    }
}

fn cascade_files(mode: &str) -> Vec<String> {
    let short = mode_short(mode);
    let skip_local = mode == "test";
    let mut out = Vec::new();
    // Mode-specific first…
    out.push(format!(".env.{short}"));
    if short != mode {
        out.push(format!(".env.{mode}"));
    }
    // …optional local overlay…
    if !skip_local {
        out.push(".env.local".into());
    }
    // …then base `.env` wins over mode files.
    out.push(".env".into());
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
    fn cascade_env_overrides_mode_file() {
        let _g = lock_env();
        let prev = std::env::var_os("SOVA_ENV");
        std::env::set_var("SOVA_ENV", "test");
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env.test"), "FOO=mode\n").unwrap();
        std::fs::write(dir.path().join(".env"), "FOO=base\n").unwrap();
        std::env::remove_var("FOO");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("FOO").unwrap(), "base");
        std::env::remove_var("FOO");
        match prev {
            Some(v) => std::env::set_var("SOVA_ENV", v),
            None => std::env::remove_var("SOVA_ENV"),
        }
    }

    #[test]
    fn real_env_beats_file() {
        let _g = lock_env();
        let prev = std::env::var_os("SOVA_ENV");
        std::env::set_var("SOVA_ENV", "test");
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env"), "BAR=file\n").unwrap();
        std::env::set_var("BAR", "process");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("BAR").unwrap(), "process");
        std::env::remove_var("BAR");
        match prev {
            Some(v) => std::env::set_var("SOVA_ENV", v),
            None => std::env::remove_var("SOVA_ENV"),
        }
    }

    #[test]
    fn require_errors_clearly() {
        let _g = lock_env();
        std::env::remove_var("MISSING_TEST_VAR_X");
        let err = require("MISSING_TEST_VAR_X").unwrap_err();
        assert!(matches!(err, EnvError::Missing(_)));
    }

    #[test]
    fn require_returns_value() {
        let _g = lock_env();
        std::env::set_var("SOVA_ENV_REQUIRE_OK", "present");
        assert_eq!(require("SOVA_ENV_REQUIRE_OK").unwrap(), "present");
        std::env::remove_var("SOVA_ENV_REQUIRE_OK");
    }

    #[test]
    fn require_rejects_empty() {
        let _g = lock_env();
        std::env::set_var("SOVA_ENV_REQUIRE_EMPTY", "");
        let err = require("SOVA_ENV_REQUIRE_EMPTY").unwrap_err();
        assert!(matches!(err, EnvError::Missing(_)));
        std::env::remove_var("SOVA_ENV_REQUIRE_EMPTY");
    }

    #[test]
    fn cascade_merges_keys_env_last() {
        let _g = lock_env();
        let prev = std::env::var_os("SOVA_ENV");
        std::env::set_var("SOVA_ENV", "test");
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env.test"), "ONLY_MODE=2\nSHARED=mode\n").unwrap();
        std::fs::write(dir.path().join(".env"), "ONLY_BASE=1\nSHARED=base\n").unwrap();
        std::env::remove_var("ONLY_BASE");
        std::env::remove_var("ONLY_MODE");
        std::env::remove_var("SHARED");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("ONLY_BASE").unwrap(), "1");
        assert_eq!(std::env::var("ONLY_MODE").unwrap(), "2");
        assert_eq!(std::env::var("SHARED").unwrap(), "base");
        std::env::remove_var("ONLY_BASE");
        std::env::remove_var("ONLY_MODE");
        std::env::remove_var("SHARED");
        match prev {
            Some(v) => std::env::set_var("SOVA_ENV", v),
            None => std::env::remove_var("SOVA_ENV"),
        }
    }

    #[test]
    fn development_loads_env_dev_alias() {
        let _g = lock_env();
        let prev = std::env::var_os("SOVA_ENV");
        let prev_app = std::env::var_os("APP_ENV");
        std::env::set_var("SOVA_ENV", "development");
        std::env::remove_var("APP_ENV");
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env.dev"), "DEV_ONLY=1\nSHARED=dev\n").unwrap();
        std::fs::write(dir.path().join(".env"), "SHARED=root\n").unwrap();
        std::env::remove_var("DEV_ONLY");
        std::env::remove_var("SHARED");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("DEV_ONLY").unwrap(), "1");
        assert_eq!(std::env::var("SHARED").unwrap(), "root");
        std::env::remove_var("DEV_ONLY");
        std::env::remove_var("SHARED");
        match prev {
            Some(v) => std::env::set_var("SOVA_ENV", v),
            None => std::env::remove_var("SOVA_ENV"),
        }
        match prev_app {
            Some(v) => std::env::set_var("APP_ENV", v),
            None => std::env::remove_var("APP_ENV"),
        }
    }
}
