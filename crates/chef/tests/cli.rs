use std::fs;
use std::io::Write;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn run_interpreter_executes_main_and_emits_step_trace() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("main.lang");
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

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("run")
        .arg(&file)
        .arg("--function")
        .arg("main")
        .arg("--args")
        .arg("4")
        .arg("9")
        .arg("--step")
        .output()
        .expect("run interpreter");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.ends_with("9\n"));
    assert!(stdout.contains("trace: if"));
}

#[test]
fn test_command_runs_test_functions() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("tests.lang");
    fs::write(
        &file,
        "\
fn test_truth() -> Bool:
  true
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("test")
        .arg(&file)
        .output()
        .expect("run test");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("all tests passed"));
}

#[test]
fn test_command_json_emits_versioned_result() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("tests.lang");
    fs::write(
        &file,
        "\
fn test_truth() -> Bool:
  true
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("test")
        .arg(&file)
        .arg("--json")
        .output()
        .expect("run test --json");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["tests"], serde_json::json!(["test_truth"]));
    assert_eq!(json["failures"], serde_json::json!([]));
}

#[test]
fn test_command_json_reports_compile_failures() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("broken.lang");
    fs::write(
        &file,
        "\
fn broken(flag: Bool, value: Int) -> Int:
  if flag:
    value
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("test")
        .arg(&file)
        .arg("--json")
        .output()
        .expect("run failing test --json");

    assert!(!output.status.success(), "expected failing exit status");
    assert!(
        output.stdout.is_empty(),
        "expected empty stdout, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let json: serde_json::Value = serde_json::from_str(&stderr).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["diagnostics"][0]["code"], "E_IF_ELSE_REQUIRED");
}

#[test]
fn test_command_prints_human_readable_compile_failures_without_json() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("broken.lang");
    fs::write(
        &file,
        "\
fn broken(flag: Bool, value: Int) -> Int:
  if flag:
    value
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("test")
        .arg(&file)
        .output()
        .expect("run failing test");

    assert!(!output.status.success(), "expected failing exit status");
    assert!(
        output.stdout.is_empty(),
        "expected empty stdout, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stdout.contains("[Parser] E_IF_ELSE_REQUIRED If expressions must include an else branch")
            && stdout.contains("Add an `else:` block with the fallback expression."),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn run_command_reports_compile_failures_on_stderr() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("broken-run.lang");
    fs::write(
        &file,
        "\
fn broken(flag: Bool, value: Int) -> Int:
  if flag:
    value
",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("run")
        .arg(&file)
        .output()
        .expect("run broken source");

    assert!(!output.status.success(), "expected failing exit status");
    assert!(
        output.stdout.is_empty(),
        "expected empty stdout, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let json: serde_json::Value = serde_json::from_str(&stderr).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["diagnostics"][0]["code"], "E_IF_ELSE_REQUIRED");
}

#[test]
fn run_command_reads_stdin_for_practicea_style_program() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("practicea.lang");
    fs::write(
        &file,
        "\
import stdin
import console

fn main():
  let input = stdin.read_text()
  if input == \"1\\n2 3\\ntest\\n\":
    \"6 test\" |> console.println
  else:
    \"unexpected input\" |> console.println
",
    )
    .expect("write source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("run")
        .arg(&file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn chef run");
    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(b"1\n2 3\ntest\n")
        .expect("write stdin");

    let output = child.wait_with_output().expect("wait for chef run");

    assert!(
        output.status.success(),
        "expected successful exit status\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8 stdout"),
        "6 test\n"
    );
}

#[test]
fn benchmark_command_reports_metrics() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("bench.json");
    fs::write(
        &file,
        r#"[
  {
    "name": "max",
    "source": "fn main(a: Int, b: Int) -> Int:\n  if a > b:\n    a\n  else:\n    b\n",
    "function": "main",
    "args": [3, 9],
    "expected": 9
  },
  {
    "name": "type_error_after_parse",
    "source": "fn main(a: Int, b: Int) -> Int:\n  true\n",
    "function": "main",
    "args": [3, 9],
    "expected": 9
  },
  {
    "name": "missing_else",
    "source": "fn main(a: Int, b: Int) -> Int:\n  if a > b:\n    a\n",
    "function": "main",
    "args": [3, 9],
    "expected": 9
  }
]"#,
    )
    .expect("write benchmark manifest");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("benchmark")
        .arg(&file)
        .output()
        .expect("run benchmark");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["total"], 3);
    assert_eq!(json["passed"], 1);
    assert_eq!(json["parse_success"], 2);
    assert_eq!(json["typecheck_success"], 1);
}

#[test]
fn benchmark_command_counts_parser_warnings_as_parse_success() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("bench-warning.json");
    fs::write(
        &file,
        r#"[
  {
    "name": "wildcard_warning",
    "source": "type Choice =\n  Left(value: Int)\n  Right(value: Int)\n\nfn main() -> Int:\n  match Right(7):\n    _ -> 0\n    Right(value) -> value\n",
    "function": "main",
    "args": [],
    "expected": 0
  }
]"#,
    )
    .expect("write benchmark manifest");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("benchmark")
        .arg(&file)
        .output()
        .expect("run benchmark");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["total"], 1);
    assert_eq!(json["parse_success"], 1);
    assert_eq!(json["typecheck_success"], 1);
    assert_eq!(json["execution_success"], 1);
    assert_eq!(json["passed"], 1);
}

#[test]
fn benchmark_command_accepts_tagged_object_adt_arguments() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("bench-adt.json");
    fs::write(
        &file,
        r#"[
  {
    "name": "unwrap_left",
    "source": "type Choice =\n  Left(value: Int)\n  Right(value: Int)\n\nfn main(choice: Choice) -> Int:\n  match choice:\n    Left(value) -> value\n    Right(value) -> value\n",
    "function": "main",
    "args": [{"tag":"Left","fields":[7]}],
    "expected": 7
  }
]"#,
    )
    .expect("write benchmark manifest");

    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("benchmark")
        .arg(&file)
        .output()
        .expect("run benchmark");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["passed"], 1);
    assert_eq!(json["execution_success"], 1);
}
