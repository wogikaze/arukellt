use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use lang_core::{CompileResult, TypedModule, compile_module};
use lang_ir::lower_to_high_ir;

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
    let source = load_source(file)?;
    let (wasm_target, emit) = resolve_build_mode(target, emit)?;
    match emit {
        BuildEmit::Wasm => {
            let bytes = lang_backend_wasm::build_module_from_source(&source, wasm_target)?;
            if let Some(path) = output {
                fs::write(path, bytes)
                    .with_context(|| format!("failed to write {}", path.display()))?;
            }
        }
        BuildEmit::Wat | BuildEmit::WatMin => {
            let result = compile_module(&source);
            if result.error_count() > 0 {
                anyhow::bail!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
            }
            let typed = result
                .module
                .ok_or_else(|| anyhow!("typed module missing"))?;
            let mut wat = lang_backend_wasm::emit_wat(&lower_to_high_ir(&typed), wasm_target)?;
            if emit == BuildEmit::WatMin {
                wat = minify_wat(&wat);
            }
            if let Some(path) = output {
                fs::write(path, wat.as_bytes())
                    .with_context(|| format!("failed to write {}", path.display()))?;
            } else {
                print!("{wat}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_build_mode(
    target: BuildTarget,
    emit: BuildEmit,
) -> Result<(lang_backend_wasm::WasmTarget, BuildEmit)> {
    match target {
        BuildTarget::Wat => {
            if emit != BuildEmit::Wasm {
                anyhow::bail!(
                    "`--target wat` cannot be combined with `--emit`; use `--target wasm-js --emit {}` instead",
                    match emit {
                        BuildEmit::Wasm => "wasm",
                        BuildEmit::Wat => "wat",
                        BuildEmit::WatMin => "wat-min",
                    }
                );
            }
            eprintln!(
                "warning: `--target wat` is deprecated; use `--target wasm-js --emit wat` instead"
            );
            Ok((
                lang_backend_wasm::WasmTarget::JavaScriptHost,
                BuildEmit::Wat,
            ))
        }
        BuildTarget::WasmJs => Ok((lang_backend_wasm::WasmTarget::JavaScriptHost, emit)),
        BuildTarget::WasmJsGc => anyhow::bail!(
            "`--target wasm-js-gc` is a reserved experimental contract for a future Wasm GC JS-host backend; current builds reject it until that backend exists. First-slice ABI plan: scalar exports only, GC refs internal to the module."
        ),
        BuildTarget::WasmComponentJs => anyhow::bail!(
            "`--target wasm-component-js` is a reserved experimental Component Model contract for a future JS-host backend; current builds reject it until that backend exists. First-slice ABI plan: scalar-only public exports, typed host interfaces, and no parity promise with `wasm-js` or `wasm-wasi`."
        ),
        BuildTarget::WasmWasi => Ok((lang_backend_wasm::WasmTarget::Wasi, emit)),
    }
}

fn minify_wat(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut pending_space = false;

    for ch in source.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            in_string = true;
            out.push(ch);
        } else if ch.is_whitespace() {
            pending_space = !out.is_empty();
        } else {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(ch);
        }
    }

    out.trim().to_owned()
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
