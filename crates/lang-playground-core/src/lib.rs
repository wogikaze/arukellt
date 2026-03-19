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
        "trace": if step { interpreter.last_trace() } else { &[] },
    });
    serde_json::to_string(&payload).map_err(|error| error.to_string())
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
