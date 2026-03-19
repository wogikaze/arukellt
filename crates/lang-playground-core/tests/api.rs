use lang_playground_core::{analyze_source_json, run_source_json};

#[test]
fn analyze_source_returns_versioned_diagnostics_json() {
    let json = analyze_source_json(
        "\
fn pick(flag: Bool, value: Int) -> Int:
  if flag:
    value
",
    )
    .expect("analysis json");

    let value: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(value["version"], "v0.1");
    assert_eq!(value["diagnostics"][0]["code"], "E_IF_ELSE_REQUIRED");
}

#[test]
fn run_source_returns_result_and_trace_json() {
    let json = run_source_json(
        "\
fn main(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
",
        "main",
        "[3, 8]",
        true,
    )
    .expect("run json");

    let value: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(value["result"], 8);
    assert!(
        value["trace"]
            .as_array()
            .expect("trace array")
            .iter()
            .any(|step| step == "if")
    );
}

#[test]
fn run_source_accepts_tagged_object_adt_arguments() {
    let json = run_source_json(
        "\
type Choice =
  Left(value: Int)
  Right(value: Int)

fn main(choice: Choice) -> Int:
  match choice:
    Left(value) -> value
    Right(value) -> value
",
        "main",
        r#"[{"tag":"Left","fields":[7]}]"#,
        false,
    )
    .expect("run json");

    let value: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(value["result"], 7);
}

#[test]
fn run_source_returns_diagnostics_json_for_compile_failures() {
    let json = run_source_json(
        "\
type Choice =
  Left(value: Int)
  Right(value: Int)

fn main(choice: Choice) -> Int:
  match choice:
    Left(value) -> value
",
        "main",
        r#"[{"tag":"Left","fields":[7]}]"#,
        false,
    )
    .expect("diagnostics json");

    let value: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(value["version"], "v0.1");
    assert_eq!(value["diagnostics"][0]["code"], "E_MATCH_NOT_EXHAUSTIVE");
}
