use lang_core::{Type, compile_module};
use lang_ir::{lower_to_high_ir, lower_to_low_ir};

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
