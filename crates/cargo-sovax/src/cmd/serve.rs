//! `cargo sovax serve` — run the release binary (production).

use crate::cmd::build::{self, BuildArgs};
use crate::project::{Project, ProjectOpts};
use clap::Parser;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
pub struct ServeArgs {
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,

    #[arg(long = "manifest-path")]
    pub manifest_path: Option<PathBuf>,

    /// Do not build if the release binary is missing (error instead).
    #[arg(long = "no-build")]
    pub no_build: bool,

    /// Skip frontend when auto-building.
    #[arg(long = "no-frontend")]
    pub no_frontend: bool,
}

pub fn run(args: ServeArgs) -> Result<(), String> {
    let project = Project::resolve(&ProjectOpts {
        package: args.package.clone(),
        manifest_path: args.manifest_path.clone(),
    })?;

    let bin = project.release_bin();
    if !bin.is_file() {
        if args.no_build {
            return Err(format!(
                "release binary not found at {} (run `cargo sovax build` or omit --no-build)",
                bin.display()
            ));
        }
        eprintln!("sova: release binary missing — building…");
        build::run(BuildArgs {
            package: args.package.clone(),
            manifest_path: args.manifest_path.clone(),
            no_frontend: args.no_frontend,
        })?;
    }

    if !bin.is_file() {
        return Err(format!("release binary still missing: {}", bin.display()));
    }

    eprintln!("sova: serve {}", bin.display());
    let mut cmd = Command::new(&bin);
    cmd.current_dir(&project.package_dir)
        .env("SOVA_ENV", "production")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd
        .status()
        .map_err(|e| format!("failed to start {}: {e}", bin.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("process exited with {status}"))
    }
}
