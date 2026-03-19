use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "lang")]
#[command(about = "Arukel v0 LLM-first language toolchain")]
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
    Run {
        file: PathBuf,
        #[arg(long, default_value = "main")]
        function: String,
        #[arg(long, num_args = 1..)]
        args: Vec<String>,
        #[arg(long)]
        interpreter: bool,
        #[arg(long)]
        step: bool,
    },
    Test {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Fmt {
        file: PathBuf,
    },
    Build {
        file: PathBuf,
        #[arg(long)]
        target: BuildTarget,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Benchmark {
        file: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum BuildTarget {
    WasmJs,
    WasmWasi,
}
