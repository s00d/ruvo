//! Default tracing subscriber for `listen` / `run` / `serve`.

/// Install a default `tracing` subscriber unless one is already set or `RUVO_LOG=off`.
pub fn ensure_tracing() {
    if std::env::var_os("RUVO_LOG").is_some_and(|v| v == "off") {
        return;
    }
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("ruvo=info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
