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
