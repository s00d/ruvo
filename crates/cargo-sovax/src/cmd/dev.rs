//! `cargo sovax dev` — watch Rust (+ optional Vite) and restart on change.

use crate::frontend::{self, kill_graceful};
use crate::project::{Project, ProjectOpts};
use crate::watch;
use clap::Parser;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const READY_DEADLINE: Duration = Duration::from_secs(600);
const READY_POLL: Duration = Duration::from_millis(250);

#[derive(Parser, Debug)]
pub struct DevArgs {
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,

    #[arg(long = "manifest-path")]
    pub manifest_path: Option<PathBuf>,

    /// Do not start frontend even if detected.
    #[arg(long = "no-frontend")]
    pub no_frontend: bool,

    /// Overlap restart: spawn new process (REUSEPORT) before SIGTERM of the old one.
    /// Default: on Unix, off on Windows. Use `--no-graceful` to force kill-then-spawn.
    #[arg(long = "graceful", action = clap::ArgAction::SetTrue)]
    pub graceful: bool,

    /// Disable overlap restart (kill then spawn).
    #[arg(long = "no-graceful", action = clap::ArgAction::SetTrue)]
    pub no_graceful: bool,

    /// Seconds to wait after SIGTERM before SIGKILL (default 20).
    #[arg(long = "drain-timeout", default_value_t = 20)]
    pub drain_timeout: u64,
}

impl DevArgs {
    fn graceful_enabled(&self) -> bool {
        if self.no_graceful {
            return false;
        }
        if self.graceful {
            return true;
        }
        cfg!(unix)
    }
}

pub fn run(args: DevArgs) -> Result<(), String> {
    let project = Project::resolve(&ProjectOpts {
        package: args.package.clone(),
        manifest_path: args.manifest_path.clone(),
    })?;

    let graceful = args.graceful_enabled();
    let drain = Duration::from_secs(args.drain_timeout);

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = Arc::clone(&stop);
        ctrlc::set_handler(move || {
            stop.store(true, Ordering::SeqCst);
        })
        .map_err(|e| e.to_string())?;
    }

    let mut frontend_child: Option<Child> = None;
    if !args.no_frontend {
        if let Some(ref fe) = project.frontend {
            match frontend::spawn_dev(fe) {
                Ok(c) => frontend_child = Some(c),
                Err(e) => eprintln!("sova: frontend not started: {e}"),
            }
        } else {
            eprintln!("sova: no frontend — Rust only");
        }
    }

    let mode = if graceful {
        "graceful overlap"
    } else {
        "kill-then-spawn"
    };
    eprintln!(
        "sova: dev -p {} ({mode}; restart on .rs / Cargo.toml / .env / sova.toml)",
        project.package_name
    );

    let rust_child = Arc::new(Mutex::new(spawn_rust(&project, graceful)?));
    let project_for_watch = project.clone();
    let rust_for_watch = Arc::clone(&rust_child);
    let stop_watch = Arc::clone(&stop);

    let watch_result = watch::watch_loop(
        &project.package_dir,
        &project.workspace_dir,
        || {
            if stop_watch.load(Ordering::SeqCst) {
                return;
            }
            eprintln!(
                "sova: change detected — restarting {}…",
                project_for_watch.package_name
            );
            if let Ok(mut guard) = rust_for_watch.lock() {
                if graceful {
                    restart_graceful(&mut guard, &project_for_watch, drain, &stop_watch);
                } else {
                    kill_graceful(&mut guard, drain);
                    match spawn_rust(&project_for_watch, false) {
                        Ok(c) => *guard = c,
                        Err(e) => eprintln!("sova: restart failed: {e}"),
                    }
                }
            }
        },
        &stop,
    );

    stop.store(true, Ordering::SeqCst);
    if let Ok(mut guard) = rust_child.lock() {
        kill_graceful(&mut guard, drain);
    }
    if let Some(mut fe) = frontend_child {
        kill_graceful(&mut fe, Duration::from_secs(5));
    }

    watch_result
}

fn restart_graceful(
    guard: &mut Child,
    project: &Project,
    drain: Duration,
    stop: &AtomicBool,
) {
    let instance_id = new_instance_id();
    let new = match spawn_rust_with_id(project, true, &instance_id) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sova: restart failed (keeping old process): {e}");
            return;
        }
    };

    let port = listen_port();
    eprintln!(
        "sova: waiting for new process (instance={instance_id}) on :{port}/ready …"
    );
    if !wait_ready(port, &instance_id, READY_DEADLINE, stop) {
        if stop.load(Ordering::SeqCst) {
            let mut new = new;
            kill_graceful(&mut new, drain);
            return;
        }
        eprintln!("sova: new process not ready in time — aborting restart, killing new");
        let mut new = new;
        kill_graceful(&mut new, drain);
        return;
    }

    eprintln!(
        "sova: new process ready — draining old ({}s)…",
        drain.as_secs()
    );
    kill_graceful(guard, drain);
    *guard = new;
}

fn spawn_rust(project: &Project, reuseport: bool) -> Result<Child, String> {
    let id = new_instance_id();
    spawn_rust_with_id(project, reuseport, &id)
}

fn spawn_rust_with_id(
    project: &Project,
    reuseport: bool,
    instance_id: &str,
) -> Result<Child, String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", &project.package_name, "--manifest-path"])
        .arg(project.package_dir.join("Cargo.toml"))
        .current_dir(&project.workspace_dir)
        .env("SOVA_ENV", "development")
        .env("SOVA_INSTANCE_ID", instance_id)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if reuseport {
        cmd.env("SOVA_REUSEPORT", "1");
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    eprintln!("sova: cargo run -p {}", project.package_name);
    cmd.spawn()
        .map_err(|e| format!("failed to spawn cargo run: {e}"))
}

fn new_instance_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("dev-{nanos}")
}

fn listen_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000)
}

/// Poll until `GET /ready` returns 2xx with matching `x-sova-instance`.
///
/// With `SO_REUSEPORT` both processes share the port; matching the instance id
/// is required so we do not treat the old process as the new one.
fn wait_ready(port: u16, instance_id: &str, deadline: Duration, stop: &AtomicBool) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        if let Ok((code, Some(inst))) = http_get_ready(port) {
            if (200..300).contains(&code) && inst == instance_id {
                return true;
            }
        }
        std::thread::sleep(READY_POLL);
    }
    false
}

fn http_get_ready(port: u16) -> Result<(u16, Option<String>), ()> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().map_err(|_| ())?,
        Duration::from_millis(500),
    )
    .map_err(|_| ())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| ())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| ())?;
    let req = format!(
        "GET /ready HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).map_err(|_| ())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|_| ())?;
    let text = String::from_utf8_lossy(&buf);
    let code = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .ok_or(())?;
    let instance = text.lines().find_map(|line| {
        let lower = line.to_ascii_lowercase();
        lower
            .strip_prefix("x-sova-instance:")
            .map(|v| v.trim().to_string())
    });
    Ok((code, instance))
}
