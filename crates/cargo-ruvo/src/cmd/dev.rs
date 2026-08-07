//! `cargo ruvo dev` — watch Rust (+ optional Vite) and restart on change.

use crate::frontend::{self, kill_child};
use crate::project::{Project, ProjectOpts};
use crate::watch;
use clap::Parser;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
pub struct DevArgs {
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,

    #[arg(long = "manifest-path")]
    pub manifest_path: Option<PathBuf>,

    /// Do not start frontend even if detected.
    #[arg(long = "no-frontend")]
    pub no_frontend: bool,
}

pub fn run(args: DevArgs) -> Result<(), String> {
    let project = Project::resolve(&ProjectOpts {
        package: args.package,
        manifest_path: args.manifest_path,
    })?;

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
                Err(e) => eprintln!("ruvo: frontend not started: {e}"),
            }
        } else {
            eprintln!("ruvo: no frontend — Rust only");
        }
    }

    eprintln!(
        "ruvo: dev -p {} (restart on .rs / Cargo.toml change)",
        project.package_name
    );

    let rust_child = Arc::new(Mutex::new(spawn_rust(&project)?));
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
                "ruvo: change detected — restarting {}…",
                project_for_watch.package_name
            );
            if let Ok(mut guard) = rust_for_watch.lock() {
                kill_child(&mut guard);
                match spawn_rust(&project_for_watch) {
                    Ok(c) => *guard = c,
                    Err(e) => eprintln!("ruvo: restart failed: {e}"),
                }
            }
        },
        &stop,
    );

    stop.store(true, Ordering::SeqCst);
    if let Ok(mut guard) = rust_child.lock() {
        kill_child(&mut guard);
    }
    if let Some(mut fe) = frontend_child {
        kill_child(&mut fe);
    }

    watch_result
}

fn spawn_rust(project: &Project) -> Result<Child, String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", &project.package_name, "--manifest-path"])
        .arg(project.package_dir.join("Cargo.toml"))
        .current_dir(&project.workspace_dir)
        .env("RUVO_ENV", "development")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    eprintln!("ruvo: cargo run -p {}", project.package_name);
    cmd.spawn()
        .map_err(|e| format!("failed to spawn cargo run: {e}"))
}
