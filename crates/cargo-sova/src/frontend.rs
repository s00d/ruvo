//! Spawn frontend (Vite / npm) commands and child process helpers.

use crate::project::FrontendConfig;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Run a shell command in `dir` (blocking). Returns stderr hint on failure.
pub fn run_blocking(cfg: &FrontendConfig, cmdline: &str) -> Result<(), String> {
    eprintln!("sova: frontend$ {cmdline}  (in {})", cfg.dir.display());
    let status = shell_command(cmdline)
        .current_dir(&cfg.dir)
        .status()
        .map_err(|e| format!("failed to spawn frontend command: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "frontend command failed with status {status}: {cmdline}"
        ))
    }
}

pub fn run_build(cfg: &FrontendConfig) -> Result<(), String> {
    run_blocking(cfg, &cfg.build)?;
    if !cfg.out.exists() {
        eprintln!(
            "sova: warning: frontend out dir {} does not exist after build",
            cfg.out.display()
        );
    }
    Ok(())
}

/// Start `frontend.dev` as a child (non-blocking).
pub fn spawn_dev(cfg: &FrontendConfig) -> Result<Child, String> {
    eprintln!("sova: frontend$ {}  (in {})", cfg.dev, cfg.dir.display());
    shell_command(&cfg.dev)
        .current_dir(&cfg.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn frontend dev: {e}"))
}

fn shell_command(cmdline: &str) -> Command {
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.args(["/C", cmdline]);
        c
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::process::CommandExt;
        let mut c = Command::new("sh");
        c.args(["-c", cmdline]);
        c.process_group(0);
        c
    }
}

/// Kill a child process group (Unix) or the process (Windows), waiting indefinitely.
pub fn kill_child(child: &mut Child) {
    kill_graceful(child, Duration::from_secs(5));
}

/// SIGTERM (process group on Unix), wait up to `timeout`, then SIGKILL.
pub fn kill_graceful(child: &mut Child, timeout: Duration) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        signal_group(pid, 15); // SIGTERM
        if wait_deadline(child, timeout) {
            return;
        }
        eprintln!(
            "sova: process did not exit within {}s — SIGKILL",
            timeout.as_secs()
        );
        signal_group(pid, 9); // SIGKILL
        let _ = child.wait();
    }
    #[cfg(not(unix))]
    {
        let _ = timeout;
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(unix)]
fn signal_group(pid: i32, sig: i32) {
    unsafe {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        let _ = kill(-pid, sig);
    }
}

fn wait_deadline(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {
                if Instant::now() >= deadline {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}
