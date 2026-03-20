use std::collections::HashSet;

use lang_core::{Type, compile_module};
use lang_ir::{
    HighExprKind, WasmFunctionBody, lower_to_high_ir, lower_to_low_ir, lower_to_wasm_ir,
    optimize_high_module,
};

#[test]
fn lowers_typed_module_into_high_and_low_ir() {
    let source = "\
fn max(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
";

    let typed = compile_module(source).module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let low = lower_to_low_ir(&high);

    assert_eq!(high.functions.len(), 1);
    assert_eq!(high.functions[0].name, "max");
    assert_eq!(high.functions[0].return_type, Type::Int);
    assert_eq!(low.functions.len(), 1);
    assert_eq!(
        low.functions[0]
            .instructions
            .last()
            .expect("instruction")
            .op_name(),
        "return"
    );
}

#[test]
fn optimizer_inlines_small_pure_helpers_and_prunes_dead_functions() {
    let source = "\
fn add_one(n: Int) -> Int:
  n + 1

fn wrap(n: Int) -> Int:
  add_one(n)

fn dead_helper() -> Int:
  99

fn main() -> Int:
  wrap(41)
";

    let typed = compile_module(source).module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let optimized = optimize_high_module(&high, &HashSet::from([String::from("main")]));

    assert_eq!(
        optimized
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>(),
        vec!["main"]
    );
    assert!(matches!(
        optimized.functions[0].body.kind,
        HighExprKind::Binary { .. }
    ));
}

#[test]
fn optimizer_keeps_binder_based_helpers_out_of_line() {
    let source = "\
fn wrap(n: Int) -> Int:
  let one = 1
  n + one

fn main() -> Int:
  wrap(41)
";

    let typed = compile_module(source).module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let optimized = optimize_high_module(&high, &HashSet::from([String::from("main")]));

    assert_eq!(optimized.functions.len(), 2);
    assert!(
        optimized
            .functions
            .iter()
            .any(|function| function.name == "wrap")
    );
    assert!(matches!(
        optimized
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function")
            .body
            .kind,
        HighExprKind::Call { .. }
    ));
}

#[test]
fn lowers_backend_facing_wasm_ir_and_specializes_suffix_recursion() {
    let source = "\
fn can_form(text: String) -> Bool:
  if text == \"\":
    true
  else:
    [\"dreamer\", \"eraser\", \"dream\", \"erase\"].any(suffix -> strip_suffix(text, suffix).map(can_form).unwrap_or(false))
";

    let typed = compile_module(source).module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let wasm = lower_to_wasm_ir(&high);

    assert_eq!(wasm.functions.len(), 1);
    match &wasm.functions[0].body {
        WasmFunctionBody::SuffixRecursion(spec) => {
            assert_eq!(spec.param_name, "text");
            assert_eq!(spec.suffixes, ["dreamer", "eraser", "dream", "erase"]);
        }
        other => panic!("expected suffix recursion specialization, got {other:?}"),
    }
    assert!(wasm.helper_usage.uses_ends_with_at);
    assert!(!wasm.helper_usage.uses_strip_suffix);
    assert!(!wasm.helper_usage.uses_unwrap_or);
    assert!(!wasm.helper_usage.uses_option_runtime);
}

#[test]
fn lowers_backend_facing_wasm_ir_and_specializes_parse_or_zero() {
    let source = "\
fn parse_or_zero(text: String) -> Int:
  match parse.i64(text):
    Ok(value) -> value
    Err(_) -> 0
";

    let typed = compile_module(source).module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let wasm = lower_to_wasm_ir(&high);

    assert_eq!(wasm.functions.len(), 1);
    match &wasm.functions[0].body {
        WasmFunctionBody::ParseI64OrZero(spec) => {
            assert_eq!(spec.param_name, "text");
        }
        other => panic!("expected parse-or-zero specialization, got {other:?}"),
    }
    assert!(wasm.helper_usage.uses_parse_i64);
    assert!(wasm.helper_usage.uses_parse_i64_or_zero);
}
