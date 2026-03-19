use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "arktc")]
#[command(about = "arukellt compiler")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Check {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Build {
        file: PathBuf,
        #[arg(long)]
        target: BuildTarget,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum BuildTarget {
    WasmJs,
    WasmWasi,
}
