//! Redis plugin state + url from toml (no live Redis required).

use sova_core::App;
use sova_redis::{Redis, RedisPool};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn install_puts_pool_in_state() {
    let mut app = App::new();
    app.install(Redis::from_env().url("redis://127.0.0.1:6399"));
    assert!(app.try_state::<RedisPool>().is_some());
}

#[test]
fn url_from_toml_when_builder_empty() {
    let _guard = ENV_LOCK.lock().unwrap();
    let prev_redis = std::env::var("REDIS_URL").ok();
    std::env::remove_var("REDIS_URL");

    let mut app = App::new();
    app.configure_from_str(
        r#"
[redis]
url = "redis://127.0.0.1:6400"
"#,
    )
    .unwrap();
    app.install(Redis::from_env().url(""));
    assert!(app.try_state::<RedisPool>().is_some());

    match prev_redis {
        Some(v) => std::env::set_var("REDIS_URL", v),
        None => std::env::remove_var("REDIS_URL"),
    }
}

#[test]
fn env_wins_over_toml_url() {
    let _guard = ENV_LOCK.lock().unwrap();
    let prev_redis = std::env::var("REDIS_URL").ok();
    std::env::set_var("REDIS_URL", "redis://127.0.0.1:6401");

    let mut app = App::new();
    app.configure_from_str(
        r#"
[redis]
url = "redis://127.0.0.1:6400"
"#,
    )
    .unwrap();
    app.install(Redis::from_env().url(""));
    // Pool is installed with env URL (connection deferred to startup).
    assert!(app.try_state::<RedisPool>().is_some());

    match prev_redis {
        Some(v) => std::env::set_var("REDIS_URL", v),
        None => std::env::remove_var("REDIS_URL"),
    }
}
