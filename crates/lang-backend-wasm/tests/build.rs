use lang_backend_wasm::{WasmTarget, build_module_from_source, emit_wasm};
use lang_core::{BinaryOp, Type};
use lang_ir::{HighExpr, HighExprKind, HighFunction, HighModule, HighParam};

fn export_names(bytes: &[u8]) -> Vec<String> {
    let mut cursor = 8;
    while cursor < bytes.len() {
        let section_id = bytes[cursor];
        cursor += 1;
        let section_size = read_u32_leb(bytes, &mut cursor) as usize;
        let section_end = cursor + section_size;
        if section_id == 7 {
            let mut section_cursor = cursor;
            let export_count = read_u32_leb(bytes, &mut section_cursor);
            let mut exports = Vec::new();
            for _ in 0..export_count {
                let name_len = read_u32_leb(bytes, &mut section_cursor) as usize;
                let name = std::str::from_utf8(&bytes[section_cursor..section_cursor + name_len])
                    .expect("export name utf8")
                    .to_owned();
                section_cursor += name_len;
                section_cursor += 1;
                let _ = read_u32_leb(bytes, &mut section_cursor);
                exports.push(name);
            }
            return exports;
        }
        cursor = section_end;
    }
    Vec::new()
}

fn read_u32_leb(bytes: &[u8], cursor: &mut usize) -> u32 {
    let mut value = 0_u32;
    let mut shift = 0_u32;
    loop {
        let byte = bytes[*cursor];
        *cursor += 1;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return value;
        }
        shift += 7;
    }
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

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
    assert_eq!(export_names(&bytes), vec!["main"]);
}

#[test]
fn wasi_target_emits_a_command_entrypoint() {
    let source = "\
fn helper(value: Int) -> Int:
  value + 1

fn main() -> Int:
  helper(41)
";

    let bytes = build_module_from_source(source, WasmTarget::Wasi).expect("wasi wasm bytes");
    let exports = export_names(&bytes);

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert_eq!(exports, vec!["_start"]);
}

#[test]
fn wasi_target_rejects_main_functions_with_parameters() {
    let source = "\
fn main(value: Int) -> Int:
  value
";

    let error = build_module_from_source(source, WasmTarget::Wasi)
        .expect_err("wasi build should reject parameterized main");

    let message = error.to_string();
    assert!(
        message.contains("main") && message.contains("parameter"),
        "unexpected error: {message}"
    );
}

#[test]
fn javascript_target_builds_string_literal_return_subset() {
    let source = "\
fn banner() -> String:
  \"hello wasm\"

fn main() -> String:
  banner()
";

    let bytes = build_module_from_source(source, WasmTarget::JavaScriptHost)
        .expect("wasm bytes for string subset");
    let exports = export_names(&bytes);

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(exports.iter().any(|name| name == "memory"));
    assert!(exports.iter().any(|name| name == "banner"));
    assert!(exports.iter().any(|name| name == "main"));
    assert!(
        contains_bytes(&bytes, b"hello wasm\0"),
        "expected string literal bytes in data section"
    );
}

#[test]
fn fieldless_adt_match_subset_builds_for_both_wasm_targets() {
    let source = "\
type Choice =
  Left
  Right

fn choose(flag: Bool) -> Choice:
  if flag:
    Left
  else:
    Right

fn main() -> Int:
  match choose(false):
    Left -> 1
    Right -> 2
";

    let js_bytes =
        build_module_from_source(source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");
    let wasi_bytes = build_module_from_source(source, WasmTarget::Wasi).expect("wasi bytes");

    assert!(js_bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(wasi_bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert_eq!(export_names(&js_bytes), vec!["choose", "main"]);
    assert!(
        export_names(&wasi_bytes)
            .iter()
            .any(|name| name == "_start")
    );
    assert!(
        export_names(&wasi_bytes)
            .iter()
            .any(|name| name == "memory")
    );
}

#[test]
fn rejects_payload_carrying_adt_shapes_outside_the_supported_subset() {
    let source = "\
type Choice =
  Left(value: Int)
  Right(value: Int)

fn choose(flag: Bool) -> Choice:
  if flag:
    Left(1)
  else:
    Right(2)

fn main() -> Int:
  match choose(false):
    Left(value) -> value
    Right(value) -> value
";

    let error = build_module_from_source(source, WasmTarget::JavaScriptHost)
        .expect_err("payload-carrying ADTs should stay unsupported in wasm backend");
    let message = error.to_string();

    assert!(
        message.contains("payload fields") && message.contains("not yet supported"),
        "unexpected error: {message}"
    );
}

#[test]
fn wasi_target_supports_payload_carrying_result_matches() {
    let source = "\
import console

type Error =
  DivisionByZero

fn divide(a: Int, b: Int) -> Result<Int, Error>:
  if b == 0:
    Err(DivisionByZero)
  else:
    Ok(a / b)

fn main():
  match divide(10, 0):
    Ok(value) -> value |> string |> console.println
    Err(error) ->
      match error:
        DivisionByZero -> \"error\" |> console.println
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi payload result bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
    assert!(contains_bytes(&bytes, b"fd_write"));
}

#[test]
fn rejects_builtin_string_calls_outside_the_supported_subset() {
    let source = "\
fn main() -> String:
  string(42)
";

    let error = build_module_from_source(source, WasmTarget::JavaScriptHost)
        .expect_err("string builtin should stay unsupported in wasm backend");
    let message = error.to_string();

    assert!(
        message.contains("calls to `string`") && message.contains("not yet supported"),
        "unexpected error: {message}"
    );
}

#[test]
fn wasi_target_supports_console_println_with_string_literal() {
    let source = "\
import console

fn main():
  \"Hello, world!\" |> console.println
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi console.println bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    // Must export _start and memory
    assert!(export_names(&bytes).iter().any(|n| n == "_start"));
    assert!(export_names(&bytes).iter().any(|n| n == "memory"));
    // String literal must appear in the data section
    assert!(
        contains_bytes(&bytes, b"Hello, world!\0"),
        "expected string literal in data section"
    );
    // fd_write import from WASI must be present
    assert!(
        contains_bytes(&bytes, b"fd_write"),
        "expected fd_write import in wasm bytes"
    );
}

#[test]
fn wasi_target_supports_string_builtin_for_int_to_string_conversion() {
    let source = "\
import console

fn main():
  42 |> string |> console.println
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi string builtin bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|n| n == "_start"));
    assert!(export_names(&bytes).iter().any(|n| n == "memory"));
    assert!(
        contains_bytes(&bytes, b"fd_write"),
        "expected fd_write import in wasm bytes"
    );
}

#[test]
fn wasi_target_supports_nested_let_bindings() {
    let source = "\
import console

fn main():
  let base = 6
  let doubled = base * 2
  let rendered = doubled |> string
  rendered |> console.println
";

    let bytes = build_module_from_source(source, WasmTarget::Wasi).expect("wasi let-binding bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
    assert!(
        contains_bytes(&bytes, b"fd_write"),
        "expected fd_write import in wasm bytes"
    );
}

#[test]
fn wasi_target_supports_int_list_literals() {
    let source = "\
fn main() -> Int:
  let values = [1, 2, 3]
  99
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi list-literal bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
}

#[test]
fn wasi_target_supports_range_inclusive_lists() {
    let source = "\
fn main() -> Int:
  let values = 1..=4
  7
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi range-inclusive bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
}

#[test]
fn wasi_target_supports_map_filter_sum_for_int_lists() {
    let source = "\
fn double(n: Int) -> Int:
  n * 2

fn divisible_by_three(n: Int) -> Bool:
  n % 3 == 0

fn main() -> Int:
  (1..=10).map(double).filter(divisible_by_three).sum()
";

    let bytes = build_module_from_source(source, WasmTarget::Wasi)
        .expect("map/filter/sum pipeline should build on wasi");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
}

#[test]
fn wasi_target_supports_filter_with_lambda_callbacks() {
    let source = "\
fn main() -> Int:
  (1..=8).filter(n -> n % 2 == 0).sum()
";

    let bytes = build_module_from_source(source, WasmTarget::Wasi)
        .expect("filter with lambda callback should build on wasi");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
}

#[test]
fn wasi_target_supports_named_function_callback_values_via_apply() {
    let source = "\
fn double(value: Int) -> Int:
  value * 2

fn main() -> Int:
  __apply(double, 21)
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi named callback bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
}

#[test]
fn wasi_target_supports_lambda_callback_values_via_apply() {
    let source = "\
fn main() -> Int:
  __apply(n -> n + 1, 41)
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi lambda callback bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
}

#[test]
fn wasi_target_rejects_non_unary_named_function_callbacks() {
    let source = "\
fn pair(left: Int, right: Int) -> Int:
  left + right

fn main() -> Int:
  __apply(pair, 21)
";

    let error = build_module_from_source(source, WasmTarget::Wasi)
        .expect_err("non-unary named callbacks should stay unsupported");
    let message = error.to_string();

    assert!(
        message.contains("only unary named function references") && message.contains("callbacks"),
        "unexpected error: {message}"
    );
}

#[test]
fn wasi_target_rejects_higher_order_named_function_callbacks() {
    let callback_ty = Type::Fn(Box::new(Type::Int), Box::new(Type::Int));
    let higher_order_ty = Type::Fn(Box::new(callback_ty.clone()), Box::new(callback_ty.clone()));
    let module = HighModule {
        imports: Vec::new(),
        types: Vec::new(),
        functions: vec![
            HighFunction {
                public: false,
                name: "apply_twice".to_owned(),
                params: vec![HighParam {
                    name: "f".to_owned(),
                    ty: callback_ty.clone(),
                }],
                return_type: callback_ty.clone(),
                body: HighExpr {
                    kind: HighExprKind::Ident("f".to_owned()),
                    ty: callback_ty.clone(),
                },
            },
            HighFunction {
                public: false,
                name: "main".to_owned(),
                params: Vec::new(),
                return_type: callback_ty.clone(),
                body: HighExpr {
                    kind: HighExprKind::Call {
                        callee: "__apply".to_owned(),
                        args: vec![
                            HighExpr {
                                kind: HighExprKind::Ident("apply_twice".to_owned()),
                                ty: higher_order_ty,
                            },
                            HighExpr {
                                kind: HighExprKind::Lambda {
                                    param: "n".to_owned(),
                                    body: Box::new(HighExpr {
                                        kind: HighExprKind::Binary {
                                            op: BinaryOp::Add,
                                            left: Box::new(HighExpr {
                                                kind: HighExprKind::Ident("n".to_owned()),
                                                ty: Type::Int,
                                            }),
                                            right: Box::new(HighExpr {
                                                kind: HighExprKind::Int(1),
                                                ty: Type::Int,
                                            }),
                                        },
                                        ty: Type::Int,
                                    }),
                                },
                                ty: callback_ty.clone(),
                            },
                        ],
                    },
                    ty: callback_ty,
                },
            },
        ],
    };

    let error = emit_wasm(&module, WasmTarget::Wasi)
        .expect_err("higher-order named callbacks should stay unsupported");
    let message = error.to_string();

    assert!(
        message.contains("higher-order function references") && message.contains("callbacks"),
        "unexpected error: {message}"
    );
}
