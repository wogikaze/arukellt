use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use lang_core::{DiagnosticStage, compile_module};
use lang_interp::{Interpreter, value_from_json, value_to_json};
use lang_ir::lower_to_high_ir;
use serde::Deserialize;
use serde_json::Value as JsonValue;

pub fn benchmark_command(file: &Path) -> Result<ExitCode> {
    let manifest =
        fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))?;
    let cases: Vec<BenchmarkCase> = serde_json::from_str(&manifest)?;
    let mut parse_success = 0usize;
    let mut typecheck_success = 0usize;
    let mut execution_success = 0usize;
    let mut passed = 0usize;
    let mut case_results = Vec::new();

    for case in &cases {
        let result = compile_module(&case.source);
        let parser_errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.stage == DiagnosticStage::Lexer
                    || diagnostic.stage == DiagnosticStage::Parser
            })
            .count();
        if parser_errors == 0 && result.error_count() == 0 {
            parse_success += 1;
        }
        if result.error_count() == 0 {
            typecheck_success += 1;
        }

        let mut case_passed = false;
        let mut execution_ok = false;
        if let Some(typed) = result.module {
            let high = lower_to_high_ir(&typed);
            let mut interpreter = Interpreter::new(&high);
            let args = case
                .args
                .iter()
                .cloned()
                .map(value_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            if let Ok(value) = interpreter.call_function(&case.function, args) {
                execution_ok = true;
                execution_success += 1;
                if value_to_json(&value) == case.expected {
                    case_passed = true;
                    passed += 1;
                }
            }
        }

        case_results.push(serde_json::json!({
            "name": case.name,
            "passed": case_passed,
            "execution_success": execution_ok,
            "repair_rounds": 0,
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "version": "v0.1",
            "total": cases.len(),
            "parse_success": parse_success,
            "typecheck_success": typecheck_success,
            "execution_success": execution_success,
            "passed": passed,
            "cases": case_results,
        }))?
    );
    Ok(ExitCode::SUCCESS)
}

#[derive(Clone, Debug, Deserialize)]
struct BenchmarkCase {
    name: String,
    source: String,
    function: String,
    args: Vec<JsonValue>,
    expected: JsonValue,
}
