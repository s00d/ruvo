//! Spawn frontend (Vite / npm) commands.

use crate::project::FrontendConfig;
use std::process::{Child, Command, Stdio};

/// Run a shell command in `dir` (blocking). Returns stderr hint on failure.
pub fn run_blocking(cfg: &FrontendConfig, cmdline: &str) -> Result<(), String> {
    eprintln!("ruvo: frontend$ {cmdline}  (in {})", cfg.dir.display());
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
            "ruvo: warning: frontend out dir {} does not exist after build",
            cfg.out.display()
        );
    }
    Ok(())
}

/// Start `frontend.dev` as a child (non-blocking).
pub fn spawn_dev(cfg: &FrontendConfig) -> Result<Child, String> {
    eprintln!("ruvo: frontend$ {}  (in {})", cfg.dev, cfg.dir.display());
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

/// Kill a child process group (Unix) or the process (Windows).
pub fn kill_child(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        unsafe {
            // libc SIGTERM without depending on the libc crate
            extern "C" {
                fn kill(pid: i32, sig: i32) -> i32;
            }
            const SIGTERM: i32 = 15;
            let _ = kill(-pid, SIGTERM);
        }
        let _ = child.wait();
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}
