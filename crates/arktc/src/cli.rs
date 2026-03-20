use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub use arktc_driver::{BuildEmit, BuildTarget};

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
    #[command(
        about = "Compile a source file to WebAssembly for a supported prototype subset",
        long_about = "Compile a source file to WebAssembly for a supported prototype subset.\n\nFor API-by-target coverage, see docs/std.md#target-support-matrix."
    )]
    Build {
        #[arg(help = "Path to the .ar source file to compile")]
        file: PathBuf,
        #[arg(
            long,
            help = "WASM ABI target; `wat` is a deprecated alias for `--target wasm-js --emit wat`, `wasm-js-gc` is an explicit experimental JS-host GC contract, and `wasm-component-js` is the experimental Component Model JS-host contract"
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
