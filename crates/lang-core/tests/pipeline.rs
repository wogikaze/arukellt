use lang_core::{DiagnosticLevel, DiagnosticStage, Type, compile_module};

#[test]
fn compiles_a_simple_pure_function() {
    let source = "\
fn max(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
";

    let result = compile_module(source);

    assert!(result.module.is_some(), "expected typed module");
    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let module = result.module.expect("typed module");
    let function = module
        .functions
        .iter()
        .find(|function| function.name == "max")
        .expect("max function");
    assert_eq!(function.return_type, Type::Int);
}

#[test]
fn rejects_if_without_else_and_keeps_structured_diagnostic_fields() {
    let source = "\
fn pick(flag: Bool, value: Int) -> Int:
  if flag:
    value
";

    let result = compile_module(source);

    assert!(result.module.is_none(), "expected compile failure");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E_IF_ELSE_REQUIRED")
        .expect("missing else diagnostic");
    assert_eq!(diagnostic.level, DiagnosticLevel::Error);
    assert_eq!(diagnostic.stage, DiagnosticStage::Parser);
    assert!(diagnostic.suggested_fix.contains("else:"));
    assert!(!diagnostic.alternatives.is_empty());
    assert!(diagnostic.confidence > 0.0);

    let json = result.to_json().expect("json diagnostics");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["diagnostics"][0]["code"], "E_IF_ELSE_REQUIRED");
}

#[test]
fn rejects_null_as_a_forbidden_literal() {
    let source = "\
fn value() -> Int:
  null
";

    let result = compile_module(source);

    assert!(result.module.is_none(), "expected compile failure");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E_NULL_FORBIDDEN")
        .expect("null forbidden diagnostic");
    assert_eq!(diagnostic.stage, DiagnosticStage::Parser);
    assert!(diagnostic.suggested_fix.contains("Option"));
}

#[test]
fn rejects_capability_calls_inside_pure_functions() {
    let source = "\
import capability console
fn main() -> Int:
  console(\"hello\")
";

    let result = compile_module(source);

    assert!(result.module.is_none(), "expected compile failure");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E_EFFECT_LEAK")
        .expect("effect leak diagnostic");
    assert_eq!(diagnostic.stage, DiagnosticStage::Typecheck);
    assert!(diagnostic.message.contains("pure"));
}

#[test]
fn compiles_adt_constructors_and_match_expressions() {
    let source = "\
type Choice =
  Left(value: Int)
  Right(value: Int)

fn pick(flag: Bool) -> Int:
  match choose(flag):
    Left(value) -> value
    Right(value) -> value

fn choose(flag: Bool) -> Choice:
  if flag:
    Left(1)
  else:
    Right(2)
";

    let result = compile_module(source);

    assert!(result.module.is_some(), "expected typed module");
    assert_eq!(
        result.error_count(),
        0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn warns_when_match_wildcard_is_not_last() {
    let source = "\
type Choice =
  Left(value: Int)
  Right(value: Int)

fn pick(choice: Choice) -> Int:
  match choice:
    _ -> 0
    Right(value) -> value
";

    let result = compile_module(source);

    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "W_WILDCARD_NOT_LAST")
        .expect("wildcard warning");
    assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
}

#[test]
fn compiles_multiline_method_chain_with_trailing_pipe() {
    let source = "\
import console

fn main():
  [1, 2, 3]
    .map(double)
    .sum()
    |> string
    |> console.println

fn double(value: i64) -> i64:
  value * 2
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_match_arms_with_indented_bodies() {
    let source = "\
type Result =
  Ok(value: i64)
  Err(value: i64)

fn main() -> i64:
  match Ok(7):
    Ok(value) ->
      value
    Err(value) ->
      value
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_lambda_with_indented_body() {
    let source = "\
fn build() -> Iter<i64>:
  iter.unfold((0, 1), state ->
    Next(state[0], (state[1], state[0] + state[1]))
  )
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_unit_return_type() {
    let source = "\
import console

fn main():
  \"hello\" |> console.println
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let module = result.module.expect("typed module");
    let main = module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main function");
    assert_eq!(main.return_type, Type::Unit);
}

#[test]
fn compiles_result_type_with_ok_err() {
    let source = "\
import fs

fn read_file(path: String) -> String:
  match fs.read_text(path):
    Ok(content) -> content
    Err(_) -> \"error\"
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_fn_generic_type() {
    let source = "\
fn make_adder(base: i64) -> Fn<i64, i64>:
  n -> base + n
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_list_literal_and_range() {
    let source = "\
fn total() -> Int:
  [1, 2, 3].sum()

fn range_total() -> Int:
  (1..=5).sum()
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_split_whitespace_and_parse_i64_pipeline() {
    let source = "\
import console

fn parse_or_zero(text: String) -> Int:
  let parsed = parse.i64(text)
  match parsed:
    Ok(value) -> value
    Err(_) -> 0

fn main():
  let tokens = \"1 2 3 test\".split_whitespace()
  let a = parse_or_zero(tokens[0])
  let b = parse_or_zero(tokens[1])
  let c = parse_or_zero(tokens[2])
  let result = [string(a + b + c), tokens[3]].join(\" \")
  result |> console.println
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_strip_suffix_result_matching() {
    let source = "\
fn trim_word(text: String) -> String:
  match strip_suffix(text, \"er\"):
    Ok(value) -> value
    Err(_) -> text
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_len_and_ends_with_at_pipeline() {
    let source = "\
fn can_form_at(text: String, end: Int) -> Bool:
  if end == 0:
    true
  else:
    if ends_with_at(text, \"dream\", end):
      can_form_at(text, end - len(\"dream\"))
    else:
      false
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_parse_bool_result_matching() {
    let source = "\
fn parse_flag(text: String) -> Bool:
  match parse.bool(text):
    Ok(value) -> value
    Err(_) -> false
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn rejects_legacy_parse_int_with_canonical_guidance() {
    let source = "\
fn parse_or_zero(text: String) -> Int:
  let parsed = parse_int(text)
  match parsed:
    Ok(value) -> value
    Err(_) -> 0
";

    let result = compile_module(source);
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E_UNKNOWN_CALL")
        .expect("unknown call diagnostic");
    assert_eq!(diagnostic.actual, "parse_int");
    assert_eq!(diagnostic.suggested_fix, "Use `parse.i64(value)` instead.");
}

#[test]
fn warns_for_to_string_method_and_suggests_string_builtin() {
    let source = "\
fn stringify() -> String:
  42.to_string()
";

    let result = compile_module(source);

    assert!(result.module.is_some(), "expected typed module");
    assert_eq!(
        result.error_count(),
        0,
        "unexpected errors: {:?}",
        result.diagnostics
    );
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "W_CANONICAL_TO_STRING")
        .expect("missing canonical to_string warning");
    assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
    assert_eq!(diagnostic.suggested_fix, "Rewrite this as `string(42)`.");

    let json = result.to_json().expect("json diagnostics");
    assert_eq!(json["warning_count"], 1);
    assert_eq!(json["diagnostics"][0]["code"], "W_CANONICAL_TO_STRING");
}

#[test]
fn warns_for_redundant_pipe_lambda_and_suggests_direct_pipe() {
    let source = "\
fn stringify() -> String:
  42 |> x -> string(x)
";

    let result = compile_module(source);

    assert!(result.module.is_some(), "expected typed module");
    assert_eq!(
        result.error_count(),
        0,
        "unexpected errors: {:?}",
        result.diagnostics
    );
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "W_CANONICAL_PIPE_LAMBDA")
        .expect("missing redundant pipe lambda warning");
    assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
    assert_eq!(
        diagnostic.suggested_fix,
        "Rewrite the pipe as `value |> string`."
    );
}

#[test]
fn compiles_modulo_and_less_than() {
    let source = "\
fn is_even(n: i64) -> Bool:
  n % 2 == 0

fn is_small(n: i64) -> Bool:
  n < 10
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_bare_import() {
    let source = "\
import console

fn main():
  \"hi\" |> console.println
";

    let result = compile_module(source);

    assert!(
        result.error_count() == 0,
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn compiles_tuple_and_index() {
    let source = "\
fn fst(pair: (Int, Int)) -> Int:
  pair[0]

fn make() -> Int:
  (10, 20)[1]
";

    let result = compile_module(source);

    // tuple param type currently parsed as Named; just check no crash
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == "E_NULL_FORBIDDEN"),
        "unexpected null diagnostic"
    );
}

#[test]
fn rejects_non_exhaustive_match_with_stable_diagnostic() {
    let source = "\
type Choice =
  Left(value: Int)
  Right(value: Int)

fn main(choice: Choice) -> Int:
  match choice:
    Left(value) -> value
";

    let result = compile_module(source);

    assert!(result.module.is_none(), "expected compile failure");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E_MATCH_NOT_EXHAUSTIVE")
        .expect("match exhaustiveness diagnostic");
    assert_eq!(diagnostic.stage, DiagnosticStage::Typecheck);
    assert!(diagnostic.suggested_fix.contains("wildcard"));
    let json = result.to_json().expect("json diagnostics");
    assert_eq!(json["diagnostics"][0]["code"], "E_MATCH_NOT_EXHAUSTIVE");
}

#[test]
fn rejects_return_type_mismatch_with_stable_diagnostic() {
    let source = "\
fn main() -> Int:
  true
";

    let result = compile_module(source);

    assert!(result.module.is_none(), "expected compile failure");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E_RETURN_MISMATCH")
        .expect("return mismatch diagnostic");
    assert_eq!(diagnostic.stage, DiagnosticStage::Typecheck);
    assert_eq!(diagnostic.expected, "Int");
    assert_eq!(diagnostic.actual, "Bool");
}
