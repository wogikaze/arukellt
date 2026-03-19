use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "chef")]
#[command(about = "arukellt project manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Run {
        file: PathBuf,
        #[arg(long, default_value = "main")]
        function: String,
        #[arg(long, num_args = 1..)]
        args: Vec<String>,
        #[arg(long)]
        step: bool,
    },
    Test {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Benchmark {
        file: PathBuf,
    },
}
