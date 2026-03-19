use std::fs;
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
fn build_command_rejects_unsupported_surface() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("unsupported.lang");
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
        .arg("wasm-wasi")
        .output()
        .expect("run build");

    assert!(!output.status.success(), "expected failing exit status");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("not yet supported") || stderr.contains("unsupported wasm"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn build_command_rejects_unsupported_bundled_example() {
    let file = repo_root().join("example/hello_world.ar");

    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&file)
        .arg("--target")
        .arg("wasm-wasi")
        .output()
        .expect("run build");

    assert!(!output.status.success(), "expected failing exit status");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("console.println") && stderr.contains("not yet supported"),
        "unexpected stderr: {stderr}"
    );
}
