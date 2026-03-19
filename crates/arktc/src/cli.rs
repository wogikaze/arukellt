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
