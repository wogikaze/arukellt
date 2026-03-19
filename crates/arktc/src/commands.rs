use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use lang_core::{CompileResult, TypedModule, compile_module};

use crate::cli::{BuildTarget, Command};

pub fn execute(command: Command) -> Result<ExitCode> {
    match command {
        Command::Check { file, json } => check_command(&file, json),
        Command::Build { file, target, output } => build_command(&file, target, output.as_deref()),
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

fn build_command(file: &Path, target: BuildTarget, output: Option<&Path>) -> Result<ExitCode> {
    let source = load_source(file)?;
    let bytes = lang_backend_wasm::build_module_from_source(
        &source,
        match target {
            BuildTarget::WasmJs => lang_backend_wasm::WasmTarget::JavaScriptHost,
            BuildTarget::WasmWasi => lang_backend_wasm::WasmTarget::Wasi,
        },
    )?;
    if let Some(path) = output {
        fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(ExitCode::SUCCESS)
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
