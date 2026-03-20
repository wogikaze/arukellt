use std::fs;
use std::path::PathBuf;
use std::process::Command;

use lang_backend_wasm::{WasmTarget, build_module_from_source, emit_wasm};
use lang_core::{BinaryOp, Type};
use lang_ir::{HighExpr, HighExprKind, HighFunction, HighModule, HighParam};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

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

fn run_wasm_js_i32(bytes: &[u8], function: &str) -> i32 {
    let dir = tempdir().expect("tempdir");
    let wasm = dir.path().join("module.wasm");
    fs::write(&wasm, bytes).expect("write wasm");
    let script = format!(
        r#"
import fs from 'node:fs/promises';
const bytes = await fs.readFile({wasm:?});
const {{ instance }} = await WebAssembly.instantiate(bytes, {{}});
console.log(instance.exports[{function:?}]());
"#
    );
    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .output()
        .expect("run node wasm-js");
    assert!(
        output.status.success(),
        "expected node wasm-js success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8 stdout")
        .trim()
        .parse()
        .expect("parse i32 result")
}

fn run_wasm_wasi_stdout(bytes: &[u8]) -> String {
    let dir = tempdir().expect("tempdir");
    let wasm = dir.path().join("module.wasm");
    fs::write(&wasm, bytes).expect("write wasm");
    let script = format!(
        r#"
import fs from 'node:fs/promises';
import {{ WASI }} from 'node:wasi';
const wasi = new WASI({{ version: 'preview1' }});
const bytes = await fs.readFile({wasm:?});
const {{ instance }} = await WebAssembly.instantiate(bytes, wasi.getImportObject());
wasi.start(instance);
"#
    );
    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .output()
        .expect("run node wasi");
    assert!(
        output.status.success(),
        "expected node wasi success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
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
fn wasi_target_supports_fs_read_text_result_payloads() {
    let source = fs::read_to_string(repo_root().join("example/file_read.ar"))
        .expect("file_read example source");

    let bytes =
        build_module_from_source(&source, WasmTarget::Wasi).expect("wasi fs.read_text bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
    assert!(contains_bytes(&bytes, b"path_open"));
    assert!(contains_bytes(&bytes, b"fd_read"));
    assert!(contains_bytes(&bytes, b"fd_close"));
}

#[test]
fn wasi_target_supports_stdin_read_text_split_whitespace_pipeline() {
    let source = "\
import console
import stdin

fn main():
  stdin.read_text()
    .split_whitespace()
    .join(\",\")
    |> console.println
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("wasi stdin.read_text bytes");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
    assert!(contains_bytes(&bytes, b"fd_read"));
    assert!(contains_bytes(&bytes, b"split_whitespace"));
}

#[test]
fn join_runs_on_both_wasm_targets() {
    let js_source = "\
import console

fn main():
  \"red blue green\".split_whitespace().join(\"-\") |> console.println
";
    let wasi_source = "\
import console

fn main():
  \"red blue green\".split_whitespace().join(\"-\") |> console.println
";

    let js_bytes =
        build_module_from_source(js_source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");
    let wasi_bytes = build_module_from_source(wasi_source, WasmTarget::Wasi).expect("wasi bytes");

    assert!(contains_bytes(&js_bytes, b"__list_join_strings"));
    assert!(contains_bytes(&wasi_bytes, b"__list_join_strings"));
}

#[test]
fn wasi_target_supports_join_with_dynamic_string_items() {
    let source = "\
import console

fn main():
  let s = \"test\"
  let result = [string(6), s].join(\" \")
  result |> console.println
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("dynamic string join on wasi");

    assert_eq!(run_wasm_wasi_stdout(&bytes), "6 test\n");
}

#[test]
fn javascript_target_supports_console_println_with_string_literals() {
    let source = "\
import console

fn main():
  \"hello js\" |> console.println
";

    let bytes = build_module_from_source(source, WasmTarget::JavaScriptHost)
        .expect("console.println literal should build on wasm-js");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
    assert!(export_names(&bytes).iter().any(|name| name == "main"));
    assert!(contains_bytes(&bytes, b"console.println"));
}

#[test]
fn javascript_target_supports_string_builtin_for_int_to_string_conversion() {
    let source = "\
import console

fn main():
  42 |> string |> console.println
";

    let bytes = build_module_from_source(source, WasmTarget::JavaScriptHost)
        .expect("string builtin should build on wasm-js");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "memory"));
    assert!(export_names(&bytes).iter().any(|name| name == "main"));
    assert!(contains_bytes(&bytes, b"console.println"));
}

#[test]
fn wasi_target_supports_stdin_read_line() {
    let source = "\
import stdin
import console

fn main():
  let first = stdin.read_line()
  let second = stdin.read_line()
  let parts = [first, second]
  join(parts, \"|\") |> console.println
";

    let bytes =
        build_module_from_source(source, WasmTarget::Wasi).expect("stdin.read_line should build");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(contains_bytes(&bytes, b"stdin.read_line"));
}

#[test]
fn unsupported_builtin_errors_point_to_the_target_support_matrix() {
    let source = "\
import stdin

fn main() -> String:
  stdin.read_line()
";

    let error = build_module_from_source(source, WasmTarget::JavaScriptHost)
        .expect_err("stdin.read_line should stay unsupported on wasm-js");
    let message = error.to_string();

    assert!(
        message.contains("calls to `stdin.read_line`")
            && message.contains("wasm-js")
            && message.contains("docs/std.md#target-support-matrix"),
        "unexpected error: {message}"
    );
}

#[test]
fn parse_i64_runs_on_both_wasm_targets() {
    let js_source = "\
fn main() -> Int:
  match parse.i64(\"41\"):
    Ok(value) -> value + 1
    Err(_) -> 0
";
    let wasi_source = "\
import console

fn main():
  match parse.i64(\"41\"):
    Ok(value) -> string(value + 1) |> console.println
    Err(_) -> \"0\" |> console.println
";

    let js_bytes =
        build_module_from_source(js_source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");
    let wasi_bytes = build_module_from_source(wasi_source, WasmTarget::Wasi).expect("wasi bytes");

    assert_eq!(run_wasm_js_i32(&js_bytes, "main"), 42);
    assert_eq!(run_wasm_wasi_stdout(&wasi_bytes), "42\n");
}

#[test]
fn parse_bool_runs_on_both_wasm_targets() {
    let js_source = "\
fn main() -> Bool:
  match parse.bool(\"false\"):
    Ok(value) -> value
    Err(_) -> true
";
    let wasi_source = "\
import console

fn main():
  match parse.bool(\"false\"):
    Ok(value) ->
      if value:
        \"true\" |> console.println
      else:
        \"false\" |> console.println
    Err(_) -> \"err\" |> console.println
";

    let js_bytes =
        build_module_from_source(js_source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");
    let wasi_bytes = build_module_from_source(wasi_source, WasmTarget::Wasi).expect("wasi bytes");

    assert_eq!(run_wasm_js_i32(&js_bytes, "main"), 0);
    assert_eq!(run_wasm_wasi_stdout(&wasi_bytes), "false\n");
}

#[test]
fn split_whitespace_runs_on_both_wasm_targets() {
    let js_source = "\
fn main() -> Int:
  match parse.i64(\"41 beta\".split_whitespace()[0]):
    Ok(value) -> value + 1
    Err(_) -> 0
";
    let wasi_source = "\
import console

fn main():
  \"alpha beta\".split_whitespace()[0] |> console.println
";

    let js_bytes =
        build_module_from_source(js_source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");
    let wasi_bytes = build_module_from_source(wasi_source, WasmTarget::Wasi).expect("wasi bytes");

    assert_eq!(run_wasm_js_i32(&js_bytes, "main"), 42);
    assert_eq!(run_wasm_wasi_stdout(&wasi_bytes), "alpha\n");
}

#[test]
fn dynamic_list_index_builds_for_wasi() {
    let source = "\
import stdin

fn main() -> Int:
  let tokens = stdin.read_text().split_whitespace()
  match parse.i64(tokens[1]):
    Ok(value) -> value
    Err(_) -> 0
";

    let bytes = build_module_from_source(source, WasmTarget::Wasi)
        .expect("dynamic list indexing should build on wasi");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(contains_bytes(&bytes, b"__list_get"));
}

#[test]
fn strip_suffix_builds_for_wasi() {
    let source = "\
fn main() -> Int:
  match strip_suffix(\"dreamer\", \"er\"):
    Ok(rest) ->
      if rest == \"dream\":
        1
      else:
        0
    Err(_) -> 0
";

    let bytes = build_module_from_source(source, WasmTarget::Wasi)
        .expect("strip_suffix should build on wasi");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(contains_bytes(&bytes, b"strip_suffix"));
}

#[test]
fn javascript_target_supports_int_list_literals() {
    let source = "\
fn main() -> Int:
  let values = [20, 22]
  values[0] + values[1]
";

    let bytes =
        build_module_from_source(source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");

    assert_eq!(run_wasm_js_i32(&bytes, "main"), 42);
}

#[test]
fn javascript_target_supports_range_inclusive_lists() {
    let source = "\
fn main() -> Int:
  (1..=6).sum()
";

    let bytes =
        build_module_from_source(source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");

    assert_eq!(run_wasm_js_i32(&bytes, "main"), 21);
}

#[test]
fn javascript_target_supports_map_filter_sum_for_int_lists() {
    let source = "\
fn double(n: Int) -> Int:
  n * 2

fn divisible_by_three(n: Int) -> Bool:
  n % 3 == 0

fn main() -> Int:
  (1..=10).map(double).filter(divisible_by_three).sum()
";

    let bytes =
        build_module_from_source(source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");

    assert_eq!(run_wasm_js_i32(&bytes, "main"), 36);
}

#[test]
fn javascript_target_supports_iter_unfold_take_for_int_sequences() {
    let source = "\
fn main() -> Int:
  iter.unfold(1, n ->
    Next(n, n + 1)
  ).take(3).sum()
";

    let bytes =
        build_module_from_source(source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");

    assert_eq!(run_wasm_js_i32(&bytes, "main"), 6);
}

#[test]
fn invalid_parse_i64_matches_err_shape_on_both_wasm_targets() {
    let js_source = "\
fn main() -> Int:
  match parse.i64(\"abc\"):
    Ok(_) -> 0
    Err(_) -> 2
";
    let wasi_source = "\
import console

fn main():
  match parse.i64(\"abc\"):
    Ok(_) -> \"0\" |> console.println
    Err(_) -> \"2\" |> console.println
";

    let js_bytes =
        build_module_from_source(js_source, WasmTarget::JavaScriptHost).expect("wasm-js bytes");
    let wasi_bytes = build_module_from_source(wasi_source, WasmTarget::Wasi).expect("wasi bytes");

    assert_eq!(run_wasm_js_i32(&js_bytes, "main"), 2);
    assert_eq!(run_wasm_wasi_stdout(&wasi_bytes), "2\n");
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

#[test]
fn wasi_target_supports_iter_unfold_take_with_tuple_state() {
    let source = "\
import console

fn fibonacci_iter() -> Iter<i64>:
  iter.unfold((0, 1), state ->
    Next(state[0], (state[1], state[0] + state[1]))
  )

fn main():
  fibonacci_iter()
    .take(10)
    .map(string)
    .join(\", \")
    |> console.println
";

    let bytes = build_module_from_source(source, WasmTarget::Wasi)
        .expect("wasi bytes for iter.unfold/take");

    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert!(export_names(&bytes).iter().any(|name| name == "_start"));
}
