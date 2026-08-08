use clap::{Parser, Subcommand};

use cargo_sova::cmd::{
    build::BuildArgs, db::DbArgs, dev::DevArgs, generate::GenerateArgs, new::NewArgs,
    serve::ServeArgs,
};

#[derive(Parser, Debug)]
#[command(name = "cargo-sova", about = "Sova project tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    New(NewArgs),
    Generate(GenerateArgs),
    /// Run the app with file watch (and Vite when detected).
    ///
    /// Watches `.rs`, `Cargo.toml`, `.env*`, and `sova.toml`. On Unix, restart is
    /// graceful by default (`--graceful`): new process binds with `SO_REUSEPORT`
    /// while the old one drains. Use `--no-graceful` for kill-then-spawn.
    Dev(DevArgs),
    /// Build frontend (if any) + release binary.
    Build(BuildArgs),
    /// Run the release binary (production).
    Serve(ServeArgs),
    /// Database tooling: migrate / down / status / seed (runs the app CLI).
    Db(DbArgs),
}

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let args = if raw.get(1).map(String::as_str) == Some("sova") {
        let mut normalized = vec![raw[0].clone()];
        normalized.extend(raw.into_iter().skip(2));
        normalized
    } else {
        raw
    };
    let cli = Cli::parse_from(args);
    let result = match cli.command {
        Command::New(new) => cargo_sova::cmd::new::run(new),
        Command::Generate(generate) => cargo_sova::cmd::generate::run(generate),
        Command::Dev(dev) => cargo_sova::cmd::dev::run(dev),
        Command::Build(build) => cargo_sova::cmd::build::run(build),
        Command::Serve(serve) => cargo_sova::cmd::serve::run(serve),
        Command::Db(db) => cargo_sova::cmd::db::run(db),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
