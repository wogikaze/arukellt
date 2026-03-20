use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn scalar_source() -> &'static str {
    "\
fn main() -> Int:
  42
"
}

fn assert_wat_shape(stdout: &str, export_name: &str) {
    assert!(
        stdout.starts_with("(module"),
        "expected wat module on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!("(func ${export_name}")) || stdout.contains("(func $_start"),
        "expected function body in wat, got:\n{stdout}"
    );
}

fn build_and_run_wasi_source(source: &str) -> String {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("input.ar");
    let output_file = dir.path().join("out.wasm");
    fs::write(&file, source).expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-wasi")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(
        output.status.success(),
        "expected successful wasm-wasi build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new("wasmer")
        .arg(&output_file)
        .output()
        .expect("run wasmer");
    assert!(
        run.status.success(),
        "expected wasmer success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    String::from_utf8(run.stdout).expect("utf8 stdout")
}

fn build_and_run_js_source(source: &str) -> String {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("input.ar");
    let output_file = dir.path().join("out.wasm");
    fs::write(&file, source).expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-js")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(
        output.status.success(),
        "expected successful wasm-js build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let script = format!(
        r#"
import {{ readFileSync }} from 'node:fs';

const bytes = readFileSync({path:?});
let instance;
const lines = [];
const imports = {{
  arukellt_host: {{
    "console.println": (ptr, len) => {{
      const view = new Uint8Array(instance.exports.memory.buffer, ptr, len);
      lines.push(new TextDecoder().decode(view));
    }},
  }},
}};
({{ instance }} = await WebAssembly.instantiate(bytes, imports));
instance.exports.main();
process.stdout.write(lines.join("\n"));
"#,
        path = output_file.display().to_string(),
    );

    let run = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .output()
        .expect("run node");
    assert!(
        run.status.success(),
        "expected node success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    String::from_utf8(run.stdout).expect("utf8 stdout")
}

fn build_and_run_wasi_source_with_stdin(source: &str, stdin: &str) -> String {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("input.ar");
    let output_file = dir.path().join("out.wasm");
    fs::write(&file, source).expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-wasi")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(
        output.status.success(),
        "expected successful wasm-wasi build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut child = Command::new("wasmer")
        .arg(&output_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn wasmer");

    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");

    let run = child.wait_with_output().expect("wait for wasmer");
    assert!(
        run.status.success(),
        "expected wasmer success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    String::from_utf8(run.stdout).expect("utf8 stdout")
}

#[test]
fn check_json_emits_structured_diagnostics() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("broken.lang");
    fs::write(
        &file,
        "\
fn pick(flag: Bool, value: Int) -> Int:
  if flag:
    value
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("check")
        .arg(&file)
        .arg("--json")
        .output()
        .expect("run check");

    assert!(!output.status.success(), "expected failing exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["diagnostics"][0]["code"], "E_IF_ELSE_REQUIRED");
}

#[test]
fn build_command_writes_wasm_output() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("build.lang");
    let output_file = dir.path().join("out.wasm");
    fs::write(
        &file,
        "\
fn main(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-js")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(output.status.success(), "expected successful exit status");
    let bytes = fs::read(output_file).expect("read output wasm");
    assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert_eq!(export_names(&bytes), vec!["main"]);
}

#[test]
fn build_command_writes_a_wasi_entrypoint() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("wasi.lang");
    let output_file = dir.path().join("out.wasm");
    fs::write(
        &file,
        "\
fn main() -> Int:
  42
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-wasi")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(output.status.success(), "expected successful exit status");
    let bytes = fs::read(output_file).expect("read output wasm");
    assert_eq!(export_names(&bytes), vec!["_start"]);
}

#[test]
fn build_command_supports_all_target_emit_output_combinations() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("matrix.lang");
    fs::write(&file, scalar_source()).expect("write source");

    for (target, emit, extension, expected_export) in [
        ("wasm-js", "wasm", "wasm", "main"),
        ("wasm-js", "wat", "wat", "main"),
        ("wasm-js", "wat-min", "wat", "main"),
        ("wasm-wasi", "wasm", "wasm", "_start"),
        ("wasm-wasi", "wat", "wat", "_start"),
        ("wasm-wasi", "wat-min", "wat", "_start"),
    ] {
        let output_file = dir.path().join(format!("{target}-{emit}.{extension}"));
        let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
            .arg("build")
            .arg(&file)
            .arg("--target")
            .arg(target)
            .arg("--emit")
            .arg(emit)
            .arg("--output")
            .arg(&output_file)
            .output()
            .expect("run build");

        assert!(
            output.status.success(),
            "expected successful exit status for {target}/{emit}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        match emit {
            "wasm" => {
                let bytes = fs::read(&output_file).expect("read output wasm");
                assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
                assert_eq!(export_names(&bytes), vec![expected_export]);
            }
            "wat" => {
                let wat = fs::read_to_string(&output_file).expect("read output wat");
                assert_wat_shape(&wat, expected_export);
                assert!(
                    wat.contains('\n'),
                    "expected multi-line wat for {target}/{emit}"
                );
            }
            "wat-min" => {
                let wat = fs::read_to_string(&output_file).expect("read output wat");
                assert_wat_shape(&wat, expected_export);
                assert!(
                    !wat.contains('\n'),
                    "expected one-line wat-min output for {target}/{emit}, got:\n{wat}"
                );
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn build_command_prints_wat_to_stdout_without_output_path() {
    let file = repo_root().join("example/wasm_scalar.ar");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-js")
        .arg("--emit")
        .arg("wat")
        .output()
        .expect("run build");

    assert!(
        output.status.success(),
        "expected successful exit status\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_wat_shape(&stdout, "main");
    assert!(
        output.stderr.is_empty(),
        "expected no stderr when printing wat, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn build_command_supports_deprecated_target_wat_alias() {
    let file = repo_root().join("example/wasm_scalar.ar");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wat")
        .output()
        .expect("run build");

    assert!(
        output.status.success(),
        "expected successful exit status\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert_wat_shape(&stdout, "main");
    assert!(
        stderr.contains("deprecated") && stderr.contains("--target wasm-js --emit wat"),
        "expected deprecation warning, got:\n{stderr}"
    );
}

#[test]
fn build_command_prints_wat_min_to_stdout_without_output_path() {
    let file = repo_root().join("example/wasm_scalar.ar");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-wasi")
        .arg("--emit")
        .arg("wat-min")
        .output()
        .expect("run build");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_wat_shape(&stdout, "_start");
    assert!(!stdout.contains('\n'), "expected wat-min on one line");
}

#[test]
fn build_command_supports_string_literal_return_subset() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("string.lang");
    let output_file = dir.path().join("out.wasm");
    fs::write(
        &file,
        "\
fn main() -> String:
  \"hello\"
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-js")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(output.status.success(), "expected successful exit status");
    let bytes = fs::read(output_file).expect("read output wasm");
    let exports = export_names(&bytes);

    assert!(
        exports.iter().any(|name| name == "memory") && exports.iter().any(|name| name == "main"),
        "expected memory and main exports, got {exports:?}"
    );
}

#[test]
fn build_command_supports_fieldless_adt_match_subset() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("fieldless-match.lang");
    let output_file = dir.path().join("out.wasm");
    fs::write(
        &file,
        "\
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
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-js")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(output.status.success(), "expected successful exit status");
    let bytes = fs::read(output_file).expect("read output wasm");
    assert_eq!(export_names(&bytes), vec!["choose", "main"]);
}

#[test]
fn build_command_runs_closure_example_on_wasi() {
    let file = repo_root().join("example/closure.ar");
    let expected =
        fs::read_to_string(repo_root().join("example/meta/closure.stdout")).expect("stdout");
    let dir = tempdir().expect("tempdir");
    let output_file = dir.path().join("closure.wasm");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-wasi")
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("run build");

    assert!(
        output.status.success(),
        "expected successful exit status\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new("wasmer")
        .arg(&output_file)
        .output()
        .expect("run wasmer");
    assert!(
        run.status.success(),
        "expected wasmer success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).expect("utf8 stdout"),
        expected
    );
}

#[test]
fn build_command_preserves_nested_tuple_payload_base_pointers_on_wasi() {
    let stdout = build_and_run_wasi_source(
        "\
import console

type Pair =
  Pair(left: Int, right: Int)

type Payload =
  Wrapped(pair: Pair)

fn pair_sum(pair: Pair) -> Int:
  match pair:
    Pair(left, right) -> left + right

fn main():
  match Wrapped(Pair(20 + 1, 21)):
    Wrapped(pair) ->
      pair |> pair_sum |> string |> console.println
",
    );

    assert_eq!(stdout, "42\n");
}

#[test]
fn build_command_preserves_nested_list_payload_base_pointers_on_wasi() {
    let stdout = build_and_run_wasi_source(
        "\
import console

type Payload =
  Wrapped(values: List<Int>)

fn main():
  match Wrapped([20, 22]):
    Wrapped(values) ->
      values |> sum |> string |> console.println
",
    );

    assert_eq!(stdout, "42\n");
}

#[test]
fn build_command_reads_stdin_text_and_splits_whitespace_on_wasi() {
    let stdout = build_and_run_wasi_source_with_stdin(
        "\
import console
import stdin

fn main():
  stdin.read_text()
    .split_whitespace()
    .join(\",\")
    |> console.println
",
        "alpha beta\ngamma\n",
    );

    assert_eq!(stdout, "alpha,beta,gamma\n");
}

#[test]
fn build_command_reads_stdin_lines_on_wasi() {
    let stdout = build_and_run_wasi_source_with_stdin(
        "\
import console
import stdin

fn main():
  let first = stdin.read_line()
  let second = stdin.read_line()
  let parts = [first, second]
  join(parts, \"|\") |> console.println
",
        "alpha\nbeta\n",
    );

    assert_eq!(stdout, "alpha|beta\n");
}

#[test]
fn build_command_runs_strip_suffix_and_string_equality_on_wasi() {
    let stdout = build_and_run_wasi_source_with_stdin(
        "\
import console

fn main():
  match strip_suffix(\"dreamer\", \"er\"):
    Some(rest) ->
      if rest == \"dream\":
        \"YES\" |> console.println
      else:
        \"NO\" |> console.println
    None -> \"NO\" |> console.println
",
        "",
    );

    assert_eq!(stdout, "YES\n");
}

#[test]
fn build_command_grows_wasi_memory_for_large_list_pipelines() {
    let stdout = build_and_run_wasi_source_with_stdin(
        "\
import stdin
import console

fn parse_or_zero(text: String) -> Int:
  match parse.i64(text):
    Ok(value) -> value
    Err(_) -> 0

fn digit_sum(n: Int) -> Int:
  if n < 10:
    n
  else:
    n % 10 + digit_sum(n / 10)

fn count_if_digit_sum_matches(n: Int, target: Int) -> Int:
  if digit_sum(n) == target:
    1
  else:
    0

fn main():
  let tokens = stdin.read_text().split_whitespace()
  let n = parse_or_zero(tokens[0])
  let k = parse_or_zero(tokens[1])
  (1..=n)
    .map(value -> count_if_digit_sum_matches(value, k))
    .sum()
    |> string
    |> console.println
",
        "99999 45\n",
    );

    assert_eq!(stdout, "1\n");
}

#[test]
fn build_command_runs_joined_string_output_on_wasm_js() {
    let stdout = build_and_run_js_source(
        "\
import console

fn main():
  \"red blue green\".split_whitespace().join(\"-\") |> console.println
",
    );

    assert_eq!(stdout, "red-blue-green");
}

#[test]
fn build_command_runs_joined_string_output_on_wasm_wasi() {
    let stdout = build_and_run_wasi_source_with_stdin(
        "\
import console

fn main():
  \"red blue green\".split_whitespace().join(\"-\") |> console.println
",
        "",
    );

    assert_eq!(stdout, "red-blue-green\n");
}

#[test]
fn build_command_runs_string_output_through_console_on_wasm_js() {
    let stdout = build_and_run_js_source(
        "\
import console

fn main():
  42 |> string |> console.println
",
    );

    assert_eq!(stdout, "42");
}

#[test]
fn build_command_runs_hello_world_example_on_wasm_js() {
    let source = fs::read_to_string(repo_root().join("example/hello_world.ar")).expect("source");
    let stdout = build_and_run_js_source(&source);

    assert_eq!(stdout, "Hello, world!");
}

#[test]
fn build_command_runs_parse_i64_err_branch_on_wasm_js() {
    let stdout = build_and_run_js_source(
        "\
import console

fn main():
  match parse.i64(\"oops\"):
    Ok(value) -> value |> string |> console.println
    Err(_) -> 0 |> string |> console.println
",
    );

    assert_eq!(stdout, "0");
}

#[test]
fn build_command_rejects_unknown_target_values() {
    let file = repo_root().join("example/wasm_scalar.ar");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("bad-target")
        .output()
        .expect("run build");

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected clap usage failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("invalid value 'bad-target'") && stderr.contains("wasm-wasi"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn build_command_rejects_target_wat_when_emit_is_also_set() {
    let file = repo_root().join("example/wasm_scalar.ar");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wat")
        .arg("--emit")
        .arg("wat-min")
        .output()
        .expect("run build");

    assert!(!output.status.success(), "expected failing exit status");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("cannot be combined with `--emit`"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn build_command_succeeds_without_output_and_discards_wasm_bytes() {
    let file = repo_root().join("example/wasm_scalar.ar");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-js")
        .output()
        .expect("run build");

    assert!(
        output.status.success(),
        "expected successful exit status\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "expected no stdout when --output is omitted, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "expected no stderr when --output is omitted, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
