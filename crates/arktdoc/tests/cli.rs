use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn cli_emits_json_for_a_valid_module() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("doc.ar");
    fs::write(
        &file,
        "\
pub fn add(a: Int, b: Int) -> Int:
  a + b
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktdoc"))
        .arg(&file)
        .output()
        .expect("run arktdoc");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["file"], file.display().to_string());
    assert_eq!(json["functions"][0]["name"], "add");
    assert_eq!(json["functions"][0]["public"], true);
    assert_eq!(json["functions"][0]["params"][0]["name"], "a");
    assert_eq!(json["functions"][0]["params"][0]["type"], "Int");
    assert_eq!(json["functions"][0]["return_type"], "Int");
}

#[test]
fn cli_rejects_unsupported_non_json_format() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("doc.ar");
    fs::write(
        &file,
        "\
pub fn add(a: Int, b: Int) -> Int:
  a + b
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktdoc"))
        .arg(&file)
        .arg("--format")
        .arg("markdown")
        .output()
        .expect("run arktdoc");

    assert!(!output.status.success(), "expected failing exit status");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("output format `Markdown` is not supported yet")
            && stderr.contains("--format json"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn cli_fails_when_the_module_does_not_compile() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("broken.ar");
    fs::write(
        &file,
        "\
fn broken(flag: Bool, value: Int) -> Int:
  if flag:
    value
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktdoc"))
        .arg(&file)
        .output()
        .expect("run arktdoc");

    assert!(!output.status.success(), "expected failing exit status");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("arktdoc: compilation failed"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn cli_reports_compile_failure_before_unsupported_format_errors() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("broken.ar");
    fs::write(
        &file,
        "\
fn broken(flag: Bool, value: Int) -> Int:
  if flag:
    value
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktdoc"))
        .arg(&file)
        .arg("--format")
        .arg("markdown")
        .output()
        .expect("run arktdoc with broken source and unsupported format");

    assert!(!output.status.success(), "expected failing exit status");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("arktdoc: compilation failed")
            && !stderr.contains("output format `Markdown` is not supported yet"),
        "unexpected stderr: {stderr}"
    );
}
