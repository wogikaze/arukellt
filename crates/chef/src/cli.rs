use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
    #[command(
        about = "Compile a source file to WebAssembly for a supported prototype subset",
        long_about = "Compile a source file to WebAssembly for a supported prototype subset.\n\nFor API-by-target coverage, see docs/std.md#target-support-matrix."
    )]
    Build {
        #[arg(help = "Path to the .ar source file to compile")]
        file: PathBuf,
        #[arg(
            long,
            help = "WASM ABI target; `wat` is kept as a deprecated alias for `--target wasm-js --emit wat`"
        )]
        target: BuildTarget,
        #[arg(
            long,
            help = "Output format to emit for the selected target ABI",
            default_value = "wasm"
        )]
        emit: BuildEmit,
        #[arg(short, long, help = "Write the build output to this path")]
        output: Option<PathBuf>,
    },
    #[command(about = "Run the benchmark manifest and report JSON metrics")]
    Benchmark {
        #[arg(help = "Path to the benchmark manifest JSON file")]
        file: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum BuildTarget {
    Wat,
    WasmJs,
    WasmWasi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum BuildEmit {
    Wasm,
    Wat,
    WatMin,
}
