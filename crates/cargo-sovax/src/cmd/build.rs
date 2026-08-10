//! `cargo sovax build` — optional frontend build + `cargo build --release`.

use crate::frontend;
use crate::project::{Project, ProjectOpts};
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Package to build (`-p`).
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,

    /// Path to Cargo.toml or package directory.
    #[arg(long = "manifest-path")]
    pub manifest_path: Option<PathBuf>,

    /// Skip frontend build even if detected.
    #[arg(long = "no-frontend")]
    pub no_frontend: bool,
}

pub fn run(args: BuildArgs) -> Result<(), String> {
    let project = Project::resolve(&ProjectOpts {
        package: args.package,
        manifest_path: args.manifest_path,
    })?;

    if !args.no_frontend {
        if let Some(ref fe) = project.frontend {
            frontend::run_build(fe)?;
        } else {
            eprintln!("sova: no frontend detected — skipping asset build");
        }
    }

    eprintln!("sova: cargo build --release -p {}", project.package_name);
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            &project.package_name,
            "--manifest-path",
        ])
        .arg(project.package_dir.join("Cargo.toml"))
        .current_dir(&project.workspace_dir)
        .status()
        .map_err(|e| format!("cargo build failed to start: {e}"))?;

    if status.success() {
        eprintln!("sova: release binary → {}", project.release_bin().display());
        Ok(())
    } else {
        Err(format!("cargo build failed with status {status}"))
    }
}
