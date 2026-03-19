use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn readme_fmt_command_prints_formatted_source() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("fmt.lang");
    fs::write(
        &file,
        "\
fn max(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktfmt"))
        .arg(&file)
        .output()
        .expect("run arktfmt");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("fn max(a: Int, b: Int) -> Int:"));
}

#[test]
fn readme_fmt_write_command_updates_file() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("fmt-write.lang");
    fs::write(
        &file,
        "\
fn add(a: Int, b: Int) -> Int:
  a + b
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktfmt"))
        .arg(&file)
        .arg("--write")
        .output()
        .expect("run arktfmt --write");

    assert!(output.status.success(), "expected successful exit status");
    let content = fs::read_to_string(&file).expect("read file");
    assert!(content.contains("fn add(a: Int, b: Int) -> Int:"));
}
