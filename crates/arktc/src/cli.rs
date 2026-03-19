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
    #[command(about = "Parse and typecheck a source file")]
    Check {
        #[arg(help = "Path to the .ar source file to check")]
        file: PathBuf,
        #[arg(
            long,
            help = "Emit structured diagnostics JSON instead of human-readable lines"
        )]
        json: bool,
    },
    #[command(about = "Compile a source file to WebAssembly for a supported prototype subset")]
    Build {
        #[arg(help = "Path to the .ar source file to compile")]
        file: PathBuf,
        #[arg(
            long,
            help = "WebAssembly target; only the current limited wasm subset is supported"
        )]
        target: BuildTarget,
        #[arg(short, long, help = "Write the compiled .wasm bytes to this path")]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum BuildTarget {
    WasmJs,
    WasmWasi,
}
