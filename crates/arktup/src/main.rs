use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "arktup")]
#[command(about = "arukellt toolchain manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Show,
    Install { version: String },
    Default { version: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Show => {
            println!("arukellt {}", env!("CARGO_PKG_VERSION"));
            println!("toolchain: stable");
        }
        Command::Install { version } => {
            eprintln!("arktup: install {version} — not yet implemented");
            return ExitCode::from(1);
        }
        Command::Default { version } => {
            eprintln!("arktup: default {version} — not yet implemented");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}
