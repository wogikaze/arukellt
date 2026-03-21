use lang_core::compile_module;
use lang_interp::{Interpreter, value_to_json, values_from_json_str};
use lang_ir::lower_to_high_ir;
use serde_json::json;
use wasm_bindgen::prelude::*;

pub fn analyze_source_json(source: &str) -> Result<String, String> {
    let result = compile_module(source);
    serde_json::to_string(&result.to_json().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

pub fn run_source_json(
    source: &str,
    function: &str,
    args_json: &str,
    step: bool,
) -> Result<String, String> {
    let result = compile_module(source);
    if result.error_count() > 0 {
        return serde_json::to_string(&result.to_json().map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string());
    }
    let typed = result
        .module
        .ok_or_else(|| "typed module missing".to_owned())?;
    let high = lower_to_high_ir(&typed);
    let args = values_from_json_str(args_json).map_err(|error| error.to_string())?;
    let mut interpreter = Interpreter::new(&high);
    let value = interpreter
        .call_function(function, args)
        .map_err(|error| error.to_string())?;
    let payload = json!({
        "version": "v0.1",
        "result": value_to_json(&value),
        "output": interpreter.output(),
        "trace": if step { interpreter.last_trace() } else { &[] },
    });
    serde_json::to_string(&payload).map_err(|error| error.to_string())
}

/// Run `main()` in the given source and return a JSON object:
/// `{ "ok": bool, "output": string, "value": any, "errors": [string] }`
pub fn run_program_json(source: &str) -> String {
    let result = compile_module(source);
    if result.error_count() > 0 {
        let errors: Vec<String> = result
            .diagnostics
            .iter()
            .map(|d| format!("{} {}: {}", d.code, d.message, d.suggested_fix))
            .collect();
        return json!({ "ok": false, "errors": errors }).to_string();
    }
    let Some(typed) = result.module else {
        return json!({ "ok": false, "errors": ["typed module missing"] }).to_string();
    };
    let has_main = typed.functions.iter().any(|f| f.name == "main");
    if !has_main {
        return json!({ "ok": false, "errors": ["no main() function found"] }).to_string();
    }
    let high = lower_to_high_ir(&typed);
    let mut interpreter = Interpreter::new(&high);
    match interpreter.call_function("main", vec![]) {
        Ok(value) => json!({
            "ok": true,
            "output": interpreter.output(),
            "value": value_to_json(&value),
        })
        .to_string(),
        Err(e) => json!({ "ok": false, "errors": [e.to_string()] }).to_string(),
    }
}

#[wasm_bindgen]
pub fn analyze_source(source: &str) -> Result<String, JsValue> {
    analyze_source_json(source).map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn run_source(
    source: &str,
    function: &str,
    args_json: &str,
    step: bool,
) -> Result<String, JsValue> {
    run_source_json(source, function, args_json, step).map_err(|error| JsValue::from_str(&error))
}

/// Run `main()` and return `{ ok, output, value, errors }` as a JSON string.
/// Always succeeds (errors are reported inside the JSON, not as JS exceptions).
#[wasm_bindgen]
pub fn run_program(source: &str) -> String {
    run_program_json(source)
}
