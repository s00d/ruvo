//! load_from tempfile cascade (integration).

use sova_env::{load_from, require, EnvError};
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn with_test_mode<R>(f: impl FnOnce() -> R) -> R {
    let prev_sova = std::env::var_os("SOVA_ENV");
    let prev_app = std::env::var_os("APP_ENV");
    std::env::set_var("SOVA_ENV", "test");
    std::env::remove_var("APP_ENV");
    let out = f();
    match prev_sova {
        Some(v) => std::env::set_var("SOVA_ENV", v),
        None => std::env::remove_var("SOVA_ENV"),
    }
    match prev_app {
        Some(v) => std::env::set_var("APP_ENV", v),
        None => std::env::remove_var("APP_ENV"),
    }
    out
}

#[test]
fn load_from_tempfile_cascade() {
    let _g = lock_env();
    with_test_mode(|| {
        let dir = TempDir::new().unwrap();
        // Mode file first, `.env` last → base wins for CASCADE_A.
        std::fs::write(dir.path().join(".env.test"), "CASCADE_A=override\n").unwrap();
        std::fs::write(dir.path().join(".env"), "CASCADE_A=base\nCASCADE_B=keep\n").unwrap();
        std::env::remove_var("CASCADE_A");
        std::env::remove_var("CASCADE_B");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("CASCADE_A").unwrap(), "base");
        assert_eq!(std::env::var("CASCADE_B").unwrap(), "keep");
        std::env::remove_var("CASCADE_A");
        std::env::remove_var("CASCADE_B");
    });
}

#[test]
fn process_env_wins_over_file() {
    let _g = lock_env();
    with_test_mode(|| {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env"), "CASCADE_WIN=file\n").unwrap();
        std::env::set_var("CASCADE_WIN", "process");
        load_from(dir.path()).unwrap();
        assert_eq!(std::env::var("CASCADE_WIN").unwrap(), "process");
        std::env::remove_var("CASCADE_WIN");
    });
}

#[test]
fn require_ok_and_missing() {
    let _g = lock_env();
    std::env::set_var("CASCADE_REQ", "yes");
    assert_eq!(require("CASCADE_REQ").unwrap(), "yes");
    std::env::remove_var("CASCADE_REQ");
    assert!(matches!(
        require("CASCADE_REQ").unwrap_err(),
        EnvError::Missing(_)
    ));
}
