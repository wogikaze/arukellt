use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::Parser;
use lang_core::{Diagnostic, DiagnosticLevel, format_module, lex, parse_source};

#[derive(Parser)]
#[command(name = "arktfmt")]
#[command(about = "arukellt source formatter")]
struct Cli {
    #[arg(help = "Path to the .ar source file to format")]
    file: PathBuf,
    #[arg(
        long,
        help = "Write the formatter output back to the input file instead of stdout"
    )]
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
    let lex_output = lex(&source);
    let lexer_diagnostics = lex_output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
        .cloned()
        .collect::<Vec<_>>();
    if !lexer_diagnostics.is_empty() {
        print_diagnostics(&lexer_diagnostics);
        bail!("arktfmt: cannot format invalid source");
    }
    let parse_output = parse_source(&source);
    let parser_diagnostics = parse_output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
        .cloned()
        .collect::<Vec<_>>();
    if !parser_diagnostics.is_empty() {
        print_diagnostics(&parser_diagnostics);
        bail!("arktfmt: cannot format invalid source");
    }
    let formatted = format_module(&parse_output.module);
    if cli.write {
        fs::write(&cli.file, &formatted)
            .with_context(|| format!("failed to write {}", cli.file.display()))?;
    } else {
        print!("{formatted}");
    }
    Ok(ExitCode::SUCCESS)
}

fn print_diagnostics(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!(
            "[{:?}] {} {}: {}",
            diagnostic.stage, diagnostic.code, diagnostic.message, diagnostic.suggested_fix
        );
    }
}
