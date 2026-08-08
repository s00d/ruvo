//! `cargo sovax db` — thin launcher for app migrate / seed CLI.

use crate::project::{Project, ProjectOpts};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Subcommand, Debug)]
pub enum DbCommand {
    /// Apply pending migrations (`up [N]`).
    Migrate {
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        #[arg(long = "manifest-path")]
        manifest_path: Option<PathBuf>,
        /// Optional `up` and/or step count, e.g. `up 2` or empty for all pending.
        #[arg(trailing_var_arg = true, allow_hyphen_values = false)]
        args: Vec<String>,
    },
    /// Roll back applied migrations (`down [N]`, default 1).
    Down {
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        #[arg(long = "manifest-path")]
        manifest_path: Option<PathBuf>,
        #[arg(value_name = "N")]
        steps: Option<u32>,
    },
    /// Print applied / pending migrations.
    Status {
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        #[arg(long = "manifest-path")]
        manifest_path: Option<PathBuf>,
    },
    /// Run the app `seed` CLI (must be registered via `Db::seed`).
    Seed {
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
        #[arg(long = "manifest-path")]
        manifest_path: Option<PathBuf>,
    },
}

pub fn run(args: DbArgs) -> Result<(), String> {
    match args.command {
        DbCommand::Migrate {
            package,
            manifest_path,
            args: rest,
        } => {
            let mut cli = vec!["migrate".to_string()];
            cli.extend(rest);
            run_app_cli(package, manifest_path, &cli)
        }
        DbCommand::Down {
            package,
            manifest_path,
            steps,
        } => {
            let mut cli = vec!["migrate".into(), "down".into()];
            if let Some(n) = steps {
                cli.push(n.to_string());
            }
            run_app_cli(package, manifest_path, &cli)
        }
        DbCommand::Status {
            package,
            manifest_path,
        } => run_app_cli(package, manifest_path, &["migrate".into(), "status".into()]),
        DbCommand::Seed {
            package,
            manifest_path,
        } => run_app_cli(package, manifest_path, &["seed".into()]),
    }
}

fn run_app_cli(
    package: Option<String>,
    manifest_path: Option<PathBuf>,
    cli_args: &[String],
) -> Result<(), String> {
    let project = Project::resolve(&ProjectOpts {
        package,
        manifest_path,
    })?;

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", &project.package_name, "--manifest-path"])
        .arg(project.package_dir.join("Cargo.toml"))
        .arg("--")
        .args(cli_args)
        .current_dir(&project.workspace_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    eprintln!(
        "sova: cargo run -p {} -- {}",
        project.package_name,
        cli_args.join(" ")
    );

    let status = cmd
        .status()
        .map_err(|e| format!("failed to spawn cargo run: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo run exited with {status}"))
    }
}
