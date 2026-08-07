use clap::{Parser, Subcommand};

use cargo_ruvo::cmd::{
    build::BuildArgs, dev::DevArgs, generate::GenerateArgs, new::NewArgs, serve::ServeArgs,
};

#[derive(Parser, Debug)]
#[command(name = "cargo-ruvo", about = "Ruvo project tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    New(NewArgs),
    Generate(GenerateArgs),
    /// Run the app with file watch (and Vite when detected).
    Dev(DevArgs),
    /// Build frontend (if any) + release binary.
    Build(BuildArgs),
    /// Run the release binary (production).
    Serve(ServeArgs),
}

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let args = if raw.get(1).map(String::as_str) == Some("ruvo") {
        let mut normalized = vec![raw[0].clone()];
        normalized.extend(raw.into_iter().skip(2));
        normalized
    } else {
        raw
    };
    let cli = Cli::parse_from(args);
    let result = match cli.command {
        Command::New(new) => cargo_ruvo::cmd::new::run(new),
        Command::Generate(generate) => cargo_ruvo::cmd::generate::run(generate),
        Command::Dev(dev) => cargo_ruvo::cmd::dev::run(dev),
        Command::Build(build) => cargo_ruvo::cmd::build::run(build),
        Command::Serve(serve) => cargo_ruvo::cmd::serve::run(serve),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
