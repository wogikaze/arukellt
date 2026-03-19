use lang_backend_wasm::{WasmTarget, build_module_from_source};

#[test]
fn builds_a_valid_wasm_module_for_a_pure_function() {
    let source = "\
fn main(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
";

    let bytes = build_module_from_source(source, WasmTarget::JavaScriptHost).expect("wasm bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
}
