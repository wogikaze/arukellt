use lang_core::compile_module;
use lang_interp::{Interpreter, Value};
use lang_ir::lower_to_high_ir;

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
