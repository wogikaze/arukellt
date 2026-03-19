use std::fs;
use std::process::Command;

use tempfile::tempdir;

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
}
