use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use lang_core::{CompileResult, Diagnostic, TypedModule, compile_module};
use lang_interp::{Interpreter, Value};
use lang_ir::lower_to_high_ir;

use crate::benchmark::benchmark_command;
use crate::cli::{BuildEmit, BuildTarget, Command};

pub fn execute(command: Command) -> Result<ExitCode> {
    match command {
        Command::Run {
            file,
            function,
            args,
            step,
        } => run_command(&file, &function, &args, step),
        Command::Test { file, json } => test_command(&file, json),
        Command::Build {
            file,
            target,
            emit,
            output,
        } => build_command(&file, target, emit, output.as_deref()),
        Command::Benchmark { file } => benchmark_command(&file),
    }
}

fn run_command(file: &Path, function: &str, args: &[String], step: bool) -> Result<ExitCode> {
    let result = compile_path(file)?;
    if result.error_count() > 0 {
        eprintln!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
        return Ok(ExitCode::from(1));
    }
    let typed = result.module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let parsed_args = args
        .iter()
        .map(|value| parse_scalar_value(value))
        .collect::<Result<Vec<_>>>()?;
    let mut interpreter = Interpreter::with_live_io(&high, file.parent().map(Path::to_path_buf));
    let value = interpreter.call_function(function, parsed_args)?;
    if step {
        for line in interpreter.last_trace() {
            println!("trace: {line}");
        }
    }
    print!("{}", render_run_output(&value, interpreter.output()));
    Ok(ExitCode::SUCCESS)
}

fn test_command(file: &Path, json: bool) -> Result<ExitCode> {
    let result = compile_path(file)?;
    if result.error_count() > 0 {
        if json {
            eprintln!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
        } else {
            print_diagnostics(&result.diagnostics);
        }
        return Ok(ExitCode::from(1));
    }
    let typed = result.module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let test_names = high
        .functions
        .iter()
        .filter(|function| function.name.starts_with("test_"))
        .map(|function| function.name.clone())
        .collect::<Vec<_>>();
    let mut interpreter = Interpreter::with_base_dir(&high, file.parent().map(Path::to_path_buf));
    let mut failures = Vec::new();

    if test_names.is_empty() {
        if let Some(expected) = load_stdout_fixture(file)? {
            match interpreter.call_function("main", Vec::new()) {
                Ok(value) => {
                    let actual = render_run_output(&value, interpreter.output());
                    if actual != expected {
                        failures.push(format!("snapshot mismatch for {}", file.display()));
                    }
                }
                Err(error) => failures.push(format!("main: {error}")),
            }
        } else {
            failures.push(format!(
                "{}: no test_ functions and no .stdout fixture",
                file.display()
            ));
        }
    } else {
        for test in &test_names {
            match interpreter.call_function(test, Vec::new()) {
                Ok(Value::Bool(true)) => {}
                Ok(other) => failures.push(format!(
                    "{test}: expected Bool(true), got {}",
                    render_value(&other)
                )),
                Err(error) => failures.push(format!("{test}: {error}")),
            }
        }
    }
    if json {
        println!(
            "{}",
            serde_json::json!({
                "version": "v0.1",
                "tests": test_names,
                "failures": failures,
            })
        );
    } else {
        for failure in &failures {
            println!("{failure}");
        }
        if failures.is_empty() {
            println!("all tests passed");
        }
    }
    Ok(if failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
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

fn load_source(file: &Path) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))
}

fn compile_path(file: &Path) -> Result<CompileResult<TypedModule>> {
    let source = load_source(file)?;
    Ok(compile_module(&source))
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

fn load_stdout_fixture(file: &Path) -> Result<Option<String>> {
    if let (Some(dir), Some(stem)) = (file.parent(), file.file_stem()) {
        let meta_fixture = dir
            .join("meta")
            .join(format!("{}.stdout", stem.to_string_lossy()));
        if meta_fixture.exists() {
            return fs::read_to_string(&meta_fixture)
                .with_context(|| format!("failed to read {}", meta_fixture.display()))
                .map(Some);
        }
    }

    let adjacent_fixture = file.with_extension("stdout");
    if adjacent_fixture.exists() {
        return fs::read_to_string(&adjacent_fixture)
            .with_context(|| format!("failed to read {}", adjacent_fixture.display()))
            .map(Some);
    }
    Ok(None)
}

fn parse_scalar_value(value: &str) -> Result<Value> {
    if let Ok(number) = value.parse::<i64>() {
        return Ok(Value::Int(number));
    }
    match value {
        "true" => Ok(Value::Bool(true)),
        "false" => Ok(Value::Bool(false)),
        other => Ok(Value::String(other.to_owned())),
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Unit => String::new(),
        Value::Int(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::String(text) => text.clone(),
        Value::List(items) => {
            let rendered = items
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{rendered}]")
        }
        Value::Tuple(items) => {
            let rendered = items
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({rendered})")
        }
        Value::Variant { name, fields } => {
            let rendered = fields
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ");
            if rendered.is_empty() {
                name.clone()
            } else {
                format!("{name}({rendered})")
            }
        }
        Value::Function(name) => format!("<fn {name}>"),
        Value::Closure { .. } => "<lambda>".to_owned(),
        Value::IterUnfold { .. } => "<iter>".to_owned(),
        Value::Error => "<error>".to_owned(),
    }
}

fn render_run_output(value: &Value, captured_output: &str) -> String {
    if !captured_output.is_empty() {
        return captured_output.to_owned();
    }
    match value {
        Value::Unit => String::new(),
        _ => format!("{}\n", render_value(value)),
    }
}

fn print_diagnostics(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!(
            "[{:?}] {} {}: {}",
            diagnostic.stage, diagnostic.code, diagnostic.message, diagnostic.suggested_fix
        );
    }
}
