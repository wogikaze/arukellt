use lang_core::compile_module;
use lang_interp::{Interpreter, Value};
use lang_ir::lower_to_high_ir;

fn compile_and_run(source: &str, function: &str, args: Vec<Value>) -> Value {
    let typed = compile_module(source)
        .module
        .unwrap_or_else(|| panic!("compile failed for:\n{source}"));
    let high = lower_to_high_ir(&typed);
    let mut interpreter = Interpreter::new(&high);
    interpreter
        .call_function(function, args)
        .unwrap_or_else(|e| panic!("eval failed: {e}"))
}

fn compile_and_run_output(source: &str) -> String {
    let typed = compile_module(source)
        .module
        .unwrap_or_else(|| panic!("compile failed for:\n{source}"));
    let high = lower_to_high_ir(&typed);
    let mut interpreter = Interpreter::new(&high);
    interpreter.call_function("main", vec![]).expect("main");
    interpreter.output().to_owned()
}

fn compile_and_run_output_with_stdin(source: &str, stdin: &str) -> String {
    let typed = compile_module(source)
        .module
        .unwrap_or_else(|| panic!("compile failed for:\n{source}"));
    let high = lower_to_high_ir(&typed);
    let mut interpreter = Interpreter::with_io(&high, None, stdin);
    interpreter.call_function("main", vec![]).expect("main");
    interpreter.output().to_owned()
}

#[test]
fn evaluates_a_pure_function_through_high_ir() {
    let source = "\
fn max(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
";

    let typed = compile_module(source).module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let mut interpreter = Interpreter::new(&high);

    let result = interpreter
        .call_function("max", vec![Value::Int(3), Value::Int(9)])
        .expect("evaluation succeeds");

    assert_eq!(result, Value::Int(9));
    assert!(
        interpreter
            .last_trace()
            .iter()
            .any(|step| step.contains("if"))
    );
}

#[test]
fn evaluates_match_over_adt_values() {
    let source = "\
type Choice =
  Left(value: Int)
  Right(value: Int)

fn choose(flag: Bool) -> Choice:
  if flag:
    Left(7)
  else:
    Right(2)

fn pick(flag: Bool) -> Int:
  match choose(flag):
    Left(value) -> value
    Right(value) -> value
";

    let typed = compile_module(source).module.expect("typed module");
    let high = lower_to_high_ir(&typed);
    let mut interpreter = Interpreter::new(&high);

    let result = interpreter
        .call_function("pick", vec![Value::Bool(true)])
        .expect("evaluation succeeds");

    assert_eq!(result, Value::Int(7));
    assert!(
        interpreter
            .last_trace()
            .iter()
            .any(|step| step.contains("match"))
    );
}

#[test]
fn evaluates_closure_and_applies_it() {
    let source = "\
import console

fn make_adder(base: i64) -> Fn<i64, i64>:
  n -> base + n

fn main():
  make_adder(10)(32) |> string |> console.println
";
    let output = compile_and_run_output(source);
    assert_eq!(output.trim(), "42");
}

#[test]
fn evaluates_range_and_sum() {
    let source = "\
fn total() -> Int:
  [1, 2, 3, 4, 5].sum()
";
    let result = compile_and_run(source, "total", vec![]);
    assert_eq!(result, Value::Int(15));
}

#[test]
fn evaluates_map_filter_sum_pipeline() {
    let source = "\
fn main() -> Int:
  [1, 2, 3, 4, 5, 6]
    .filter(n -> n % 2 == 0)
    .map(n -> n * n)
    .sum()
";
    let result = compile_and_run(source, "main", vec![]);
    assert_eq!(result, Value::Int(56));
}

#[test]
fn evaluates_join_on_string_list() {
    let source = "\
fn greet() -> String:
  [\"hello\", \"world\"].join(\", \")
";
    let result = compile_and_run(source, "greet", vec![]);
    assert_eq!(result, Value::String("hello, world".to_owned()));
}

#[test]
fn evaluates_string_builtin() {
    let source = "\
fn digits() -> String:
  string(42)
";
    let result = compile_and_run(source, "digits", vec![]);
    assert_eq!(result, Value::String("42".to_owned()));
}

#[test]
fn evaluates_console_println_captures_output() {
    let source = "\
import console

fn main():
  \"Hello, arukellt!\" |> console.println
";
    let output = compile_and_run_output(source);
    assert_eq!(output, "Hello, arukellt!\n");
}

#[test]
fn evaluates_stdin_read_text_with_injected_input() {
    let source = "\
import stdin
import console

fn parse_or_zero(text: String) -> Int:
  let parsed = parse.i64(text)
  match parsed:
    Ok(value) -> value
    Err(_) -> 0

fn main():
  let tokens = stdin.read_text().split_whitespace()
  let a = parse_or_zero(tokens[0])
  let b = parse_or_zero(tokens[1])
  let c = parse_or_zero(tokens[2])
  let result = [string(a + b + c), tokens[3]].join(\" \")
  result |> console.println
";
    let output = compile_and_run_output_with_stdin(source, "1\n2 3\ntest\n");
    assert_eq!(output, "6 test\n");
}

#[test]
fn evaluates_stdin_read_line_with_injected_input() {
    let source = "\
import stdin
import console

fn main():
  let first = stdin.read_line()
  let second = stdin.read_line()
  let result = [first, second].join(\"|\")
  result |> console.println
";
    let output = compile_and_run_output_with_stdin(source, "alpha\nbeta\n");
    assert_eq!(output, "alpha|beta\n");
}

#[test]
fn evaluates_parse_bool_builtin() {
    let source = "\
fn parse_flag(text: String) -> Bool:
  match parse.bool(text):
    Ok(value) -> value
    Err(_) -> false
";
    let result = compile_and_run(source, "parse_flag", vec![Value::String("true".to_owned())]);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn evaluates_strip_suffix_builtin() {
    let source = "\
fn trim_word(text: String) -> String:
  match strip_suffix(text, \"er\"):
    Some(value) -> value
    None -> text
";
    let result = compile_and_run(
        source,
        "trim_word",
        vec![Value::String("dreamer".to_owned())],
    );
    assert_eq!(result, Value::String("dream".to_owned()));
}

#[test]
fn evaluates_option_map_unwrap_or_and_list_any() {
    let source = "\
fn can_form(text: String) -> Bool:
  if text == \"\":
    true
  else:
    [\"dreamer\", \"eraser\", \"dream\", \"erase\"].any(suffix -> strip_suffix(text, suffix).map(can_form).unwrap_or(false))
";
    let yes = compile_and_run(
        source,
        "can_form",
        vec![Value::String("dreameraser".to_owned())],
    );
    let no = compile_and_run(
        source,
        "can_form",
        vec![Value::String("dreamerer".to_owned())],
    );
    assert_eq!(yes, Value::Bool(true));
    assert_eq!(no, Value::Bool(false));
}

#[test]
fn evaluates_len_and_ends_with_at_builtins() {
    let source = "\
fn suffix_ok(text: String) -> Bool:
  ends_with_at(text, \"dream\", len(text))
";
    let result = compile_and_run(
        source,
        "suffix_ok",
        vec![Value::String("erase dream".to_owned())],
    );
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn invalid_parse_i64_matches_err_variant() {
    let source = "\
fn parse_flag(text: String) -> Int:
  match parse.i64(text):
    Ok(_) -> 0
    Err(_) -> 2
";
    let result = compile_and_run(source, "parse_flag", vec![Value::String("abc".to_owned())]);
    assert_eq!(result, Value::Int(2));
}

#[test]
fn evaluates_inclusive_range_to_list() {
    let source = "\
fn nums() -> Int:
  (1..=5).sum()
";
    let result = compile_and_run(source, "nums", vec![]);
    assert_eq!(result, Value::Int(15));
}

#[test]
fn evaluates_tuple_index_access() {
    let source = "\
fn first() -> Int:
  (10, 20, 30)[0]
";
    let result = compile_and_run(source, "first", vec![]);
    assert_eq!(result, Value::Int(10));
}

#[test]
fn evaluates_iter_unfold_take() {
    let source = "\
fn count_up() -> Int:
  iter.unfold(0, n -> Next(n, n + 1))
    .take(5)
    .sum()
";
    let result = compile_and_run(source, "count_up", vec![]);
    assert_eq!(result, Value::Int(0 + 1 + 2 + 3 + 4));
}

#[test]
fn evaluates_result_ok_err_match() {
    let source = "\
type Error =
  BadInput

fn parse(n: Int) -> Int:
  match Ok(n):
    Ok(value) -> value
    Err(_) -> 0
";
    let result = compile_and_run(source, "parse", vec![Value::Int(7)]);
    assert_eq!(result, Value::Int(7));
}

#[test]
fn evaluates_modulo_and_fizzbuzz_logic() {
    let source = "\
fn classify(n: Int) -> Int:
  if n % 3 == 0:
    1
  else:
    0
";
    assert_eq!(
        compile_and_run(source, "classify", vec![Value::Int(9)]),
        Value::Int(1)
    );
    assert_eq!(
        compile_and_run(source, "classify", vec![Value::Int(7)]),
        Value::Int(0)
    );
}

#[test]
fn evaluates_less_than_comparison() {
    let source = "\
fn smaller(a: Int, b: Int) -> Bool:
  a < b
";
    assert_eq!(
        compile_and_run(source, "smaller", vec![Value::Int(3), Value::Int(9)]),
        Value::Bool(true)
    );
    assert_eq!(
        compile_and_run(source, "smaller", vec![Value::Int(9), Value::Int(3)]),
        Value::Bool(false)
    );
}

#[test]
fn evaluates_recursive_factorial() {
    let source = "\
fn factorial(n: i64) -> i64:
  if n == 0:
    1
  else:
    n * factorial(n - 1)
";
    let result = compile_and_run(source, "factorial", vec![Value::Int(6)]);
    assert_eq!(result, Value::Int(720));
}
