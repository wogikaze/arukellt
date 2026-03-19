use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use lang_core::compile_module;

#[derive(Parser)]
#[command(name = "arktdoc")]
#[command(about = "arukellt documentation generator")]
struct Cli {
    #[arg(help = "Path to the .ar source file to document")]
    file: PathBuf,
    #[arg(
        long,
        default_value_t = OutputFormat::Json,
        value_enum,
        help = "Output format; only json is implemented today and other values fail explicitly"
    )]
    format: OutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Json,
    Markdown,
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
    let source = std::fs::read_to_string(&cli.file)?;
    let result = compile_module(&source);
    if cli.format != OutputFormat::Json {
        bail!(
            "arktdoc: output format `{:?}` is not supported yet; use --format json",
            cli.format
        );
    }
    if let Some(module) = result.module {
        let functions: Vec<_> = module
            .functions
            .iter()
            .map(|f| {
                serde_json::json!({
                    "name": f.name,
                    "public": f.public,
                    "params": f.params.iter().map(|p| serde_json::json!({
                        "name": p.name,
                        "type": p.ty.to_string(),
                    })).collect::<Vec<_>>(),
                    "return_type": f.return_type.to_string(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "version": "v0.1",
                "file": cli.file.display().to_string(),
                "functions": functions,
            }))?
        );
    } else {
        eprintln!("arktdoc: compilation failed — cannot generate docs");
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}
