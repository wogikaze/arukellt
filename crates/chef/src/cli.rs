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
    #[command(
        about = "Run a function through the interpreter",
        long_about = "Run a function through the interpreter.\n\nIf the program calls stdin.read_text(), input is read from this process stdin, so you can pipe data into `chef run`."
    )]
    Run {
        #[arg(help = "Path to the .ar source file to run")]
        file: PathBuf,
        #[arg(long, default_value = "main", help = "Function name to call")]
        function: String,
        #[arg(long, num_args = 1.., help = "Scalar arguments passed to the function")]
        args: Vec<String>,
        #[arg(
            long,
            help = "Print an interpreter execution trace before the final result"
        )]
        step: bool,
    },
    #[command(
        about = "Run test_ functions or snapshot-check main against the adjacent .stdout fixture"
    )]
    Test {
        #[arg(help = "Path to the .ar source file to test")]
        file: PathBuf,
        #[arg(
            long,
            help = "Emit versioned JSON for test results or compile diagnostics"
        )]
        json: bool,
    },
    #[command(about = "Run the benchmark manifest and report JSON metrics")]
    Benchmark {
        #[arg(help = "Path to the benchmark manifest JSON file")]
        file: PathBuf,
    },
}
