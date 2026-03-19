use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn fmt_command_emits_canonical_source() {
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
        .expect("run fmt");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("fn max(a: Int, b: Int) -> Int:"));
    assert!(stdout.contains("else:"));
}

#[test]
fn fmt_command_normalises_imports() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("import.lang");
    fs::write(
        &file,
        "\
import console

fn main():
  \"hi\" |> console.println
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktfmt"))
        .arg(&file)
        .output()
        .expect("run fmt");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("import console\n"));
    assert!(stdout.contains("fn main():"));
}

#[test]
fn fmt_command_renders_type_declarations() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("types.lang");
    fs::write(
        &file,
        "\
type Shape =
  Circle(radius: Int)
  Rect(width: Int, height: Int)

fn area(s: Shape) -> Int:
  match s:
    Circle(r) -> r
    Rect(w, h) -> w
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_arktfmt"))
        .arg(&file)
        .output()
        .expect("run fmt");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("type Shape ="));
    assert!(stdout.contains("  Circle(radius: Int)"));
    assert!(stdout.contains("  Rect(width: Int, height: Int)"));
}

#[test]
fn fmt_write_flag_updates_file_in_place() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("rewrite.lang");
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
        .expect("run fmt --write");

    assert!(output.status.success(), "expected successful exit status");
    let content = fs::read_to_string(&file).expect("read back file");
    assert!(content.contains("fn add(a: Int, b: Int) -> Int:"));
}

#[test]
fn fmt_idempotent_on_hello_world() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("hello.lang");
    let source = "import console\n\nfn main():\n  \"Hello, world!\" |> console.println\n";
    fs::write(&file, source).expect("write source");

    let first = Command::new(env!("CARGO_BIN_EXE_arktfmt"))
        .arg(&file)
        .output()
        .expect("first fmt")
        .stdout;

    let file2 = dir.path().join("hello2.lang");
    fs::write(&file2, &first).expect("write formatted");

    let second = Command::new(env!("CARGO_BIN_EXE_arktfmt"))
        .arg(&file2)
        .output()
        .expect("second fmt")
        .stdout;

    assert_eq!(first, second, "formatter must be idempotent");
}
