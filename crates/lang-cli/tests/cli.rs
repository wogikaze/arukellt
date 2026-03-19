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

    let output = Command::new(env!("CARGO_BIN_EXE_lang"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_lang"))
        .arg("run")
        .arg(&file)
        .arg("--function")
        .arg("main")
        .arg("--args")
        .arg("4")
        .arg("9")
        .arg("--interpreter")
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

    let output = Command::new(env!("CARGO_BIN_EXE_lang"))
        .arg("test")
        .arg(&file)
        .output()
        .expect("run test");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("all tests passed"));
}

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

    let output = Command::new(env!("CARGO_BIN_EXE_lang"))
        .arg("fmt")
        .arg(&file)
        .output()
        .expect("run fmt");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("fn max(a: Int, b: Int) -> Int:"));
    assert!(stdout.contains("else:"));
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

    let output = Command::new(env!("CARGO_BIN_EXE_lang"))
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
    "name": "missing_else",
    "source": "fn main(a: Int, b: Int) -> Int:\n  if a > b:\n    a\n",
    "function": "main",
    "args": [3, 9],
    "expected": 9
  }
]"#,
    )
    .expect("write benchmark manifest");

    let output = Command::new(env!("CARGO_BIN_EXE_lang"))
        .arg("benchmark")
        .arg(&file)
        .output()
        .expect("run benchmark");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["total"], 2);
    assert_eq!(json["passed"], 1);
    assert_eq!(json["parse_success"], 1);
    assert_eq!(json["typecheck_success"], 1);
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

    let output = Command::new(env!("CARGO_BIN_EXE_lang"))
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
