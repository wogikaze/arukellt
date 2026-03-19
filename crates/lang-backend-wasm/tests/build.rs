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
