use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use lang_core::{CompileResult, TypedModule, compile_module};
use lang_interp::{Interpreter, Value};
use lang_ir::lower_to_high_ir;

use crate::benchmark::benchmark_command;
use crate::cli::Command;

pub fn execute(command: Command) -> Result<ExitCode> {
    match command {
        Command::Run {
            file,
            function,
            args,
            step,
        } => run_command(&file, &function, &args, step),
        Command::Test { file, json } => test_command(&file, json),
        Command::Benchmark { file } => benchmark_command(&file),
    }
}

fn run_command(
    file: &Path,
    function: &str,
    args: &[String],
    step: bool,
) -> Result<ExitCode> {
    let result = compile_path(file)?;
    if result.error_count() > 0 {
        println!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
        return Ok(ExitCode::from(1));
    }
    let typed = result.module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let parsed_args = args
        .iter()
        .map(|value| parse_scalar_value(value))
        .collect::<Result<Vec<_>>>()?;
    let mut interpreter = Interpreter::with_base_dir(&high, file.parent().map(Path::to_path_buf));
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
            println!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
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
                        failures.push(format!(
                            "snapshot mismatch for {}",
                            file.display()
                        ));
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

fn load_source(file: &Path) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))
}

fn compile_path(file: &Path) -> Result<CompileResult<TypedModule>> {
    let source = load_source(file)?;
    Ok(compile_module(&source))
}

fn load_stdout_fixture(file: &Path) -> Result<Option<String>> {
    let fixture = file.with_extension("stdout");
    if fixture.exists() {
        return fs::read_to_string(&fixture)
            .with_context(|| format!("failed to read {}", fixture.display()))
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
            let rendered = items.iter().map(render_value).collect::<Vec<_>>().join(", ");
            format!("[{rendered}]")
        }
        Value::Tuple(items) => {
            let rendered = items.iter().map(render_value).collect::<Vec<_>>().join(", ");
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
