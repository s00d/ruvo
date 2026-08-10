//! File watch with debounce for `cargo sovax dev`.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(300);

/// Watch Rust sources, manifests, `.env*`, and `sova.toml` under the package
/// (and workspace roots). Calls `on_change` after debounce when a relevant file changes.
pub fn watch_loop(
    package_dir: &Path,
    workspace_dir: &Path,
    mut on_change: impl FnMut(),
    stop: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        notify::Config::default(),
    )
    .map_err(|e| e.to_string())?;

    let src = package_dir.join("src");
    if src.is_dir() {
        watcher
            .watch(&src, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;
    }
    let pkg_toml = package_dir.join("Cargo.toml");
    if pkg_toml.is_file() {
        watcher
            .watch(&pkg_toml, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
    }
    let ws_toml = workspace_dir.join("Cargo.toml");
    if ws_toml != pkg_toml && ws_toml.is_file() {
        watcher
            .watch(&ws_toml, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
    }
    let lock = workspace_dir.join("Cargo.lock");
    if lock.is_file() {
        watcher
            .watch(&lock, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
    }

    // Non-recursive dir watches pick up `.env*` / `sova.toml` creates & writes.
    watcher
        .watch(package_dir, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;
    if workspace_dir != package_dir {
        watcher
            .watch(workspace_dir, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
    }

    let mut pending: Option<Instant> = None;

    while !stop.load(std::sync::atomic::Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                if event_relevant(&event) {
                    pending = Some(Instant::now());
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if let Some(t) = pending {
            if t.elapsed() >= DEBOUNCE {
                pending = None;
                on_change();
            }
        }
    }

    Ok(())
}

fn event_relevant(event: &Event) -> bool {
    use notify::EventKind;
    match event.kind {
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {}
        _ => return false,
    }
    event.paths.iter().any(|p| path_relevant(p))
}

fn path_relevant(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    if name == "Cargo.toml" || name == "Cargo.lock" {
        return true;
    }
    if name.eq_ignore_ascii_case("sova.toml") {
        return true;
    }
    if name == ".env" || name.starts_with(".env.") {
        return true;
    }
    PathBuf::from(name).extension().and_then(|e| e.to_str()) == Some("rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_relevant_env_and_toml() {
        assert!(path_relevant(Path::new("/app/.env")));
        assert!(path_relevant(Path::new("/app/.env.local")));
        assert!(path_relevant(Path::new("/app/.env.development")));
        assert!(path_relevant(Path::new("/app/sova.toml")));
        assert!(path_relevant(Path::new("/app/Sova.toml")));
        assert!(path_relevant(Path::new("/app/src/main.rs")));
        assert!(!path_relevant(Path::new("/app/views/home.html")));
        assert!(!path_relevant(Path::new("/app/.envrc")));
    }
}
