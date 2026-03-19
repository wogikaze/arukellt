use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use lang_core::{format_module, parse_source};

#[derive(Parser)]
#[command(name = "arktfmt")]
#[command(about = "arukellt source formatter")]
struct Cli {
    file: PathBuf,
    /// Write formatted output back to the file instead of stdout
    #[arg(long)]
    write: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let source = fs::read_to_string(&cli.file)
        .with_context(|| format!("failed to read {}", cli.file.display()))?;
    let parse_output = parse_source(&source);
    let formatted = format_module(&parse_output.module);
    if cli.write {
        fs::write(&cli.file, &formatted)
            .with_context(|| format!("failed to write {}", cli.file.display()))?;
    } else {
        print!("{formatted}");
    }
    Ok(ExitCode::SUCCESS)
}
