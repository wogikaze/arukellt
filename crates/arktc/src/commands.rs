use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use arktc_driver::build_path;
use lang_core::{CompileResult, TypedModule, compile_module};

use crate::cli::{BuildEmit, BuildTarget, Command};

pub fn execute(command: Command) -> Result<ExitCode> {
    match command {
        Command::Check { file, json } => check_command(&file, json),
        Command::Build {
            file,
            target,
            emit,
            output,
        } => build_command(&file, target, emit, output.as_deref()),
    }
}

fn check_command(file: &Path, json: bool) -> Result<ExitCode> {
    let result = compile_path(file)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
    } else {
        for diagnostic in &result.diagnostics {
            println!(
                "[{:?}] {} {}: {}",
                diagnostic.stage, diagnostic.code, diagnostic.message, diagnostic.suggested_fix
            );
        }
    }
    Ok(exit_code_for_result(&result))
}

fn build_command(
    file: &Path,
    target: BuildTarget,
    emit: BuildEmit,
    output: Option<&Path>,
) -> Result<ExitCode> {
    build_path(file, target, emit, output)
}

fn load_source(file: &Path) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))
}

fn compile_path(file: &Path) -> Result<CompileResult<TypedModule>> {
    let source = load_source(file)?;
    Ok(compile_module(&source))
}

fn exit_code_for_result(result: &CompileResult<TypedModule>) -> ExitCode {
    if result.error_count() == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
