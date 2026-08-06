use clap::{Parser, Subcommand};

use cargo_ruvo::cmd::{generate::GenerateArgs, new::NewArgs};

#[derive(Parser, Debug)]
#[command(name = "cargo-ruvo")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    New(NewArgs),
    Generate(GenerateArgs),
}

fn main() -> Result<(), String> {
    let raw: Vec<String> = std::env::args().collect();
    let args = if raw.get(1).map(String::as_str) == Some("ruvo") {
        let mut normalized = vec![raw[0].clone()];
        normalized.extend(raw.into_iter().skip(2));
        normalized
    } else {
        raw
    };
    let cli = Cli::parse_from(args);
    match cli.command {
        Command::New(new) => cargo_ruvo::cmd::new::run(new),
        Command::Generate(generate) => cargo_ruvo::cmd::generate::run(generate),
    }
}
