use lang_backend_wasm::{WasmTarget, build_module_from_source};

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
    assert_eq!(export_names(&wasi_bytes), vec!["_start"]);
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

    let bytes = build_module_from_source(source, WasmTarget::Wasi)
        .expect("wasi string builtin bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|n| n == "_start"));
    assert!(export_names(&bytes).iter().any(|n| n == "memory"));
    assert!(
        contains_bytes(&bytes, b"fd_write"),
        "expected fd_write import in wasm bytes"
    );
}
