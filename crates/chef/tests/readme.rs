use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn readme_run_command_example_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("run")
        .arg(repo_root().join("example/hello_world.ar"))
        .output()
        .expect("run chef run");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(stdout, "Hello, world!\n");
}

#[test]
fn readme_test_command_example_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("test")
        .arg(repo_root().join("example/hello_world.ar"))
        .output()
        .expect("run chef test");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("all tests passed"));
}

#[test]
fn readme_benchmark_command_example_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("benchmark")
        .arg(repo_root().join("benchmarks/pure_logic.json"))
        .output()
        .expect("run chef benchmark");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["total"], 5);
    assert_eq!(json["passed"], 5);
}

#[test]
fn readme_build_command_example_succeeds() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output_path = temp.path().join("out.wasm");
    let output = Command::new(env!("CARGO_BIN_EXE_chef"))
        .arg("build")
        .arg(repo_root().join("example/hello_world.ar"))
        .arg("--target")
        .arg("wasm-wasi")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("run chef build");

    assert!(output.status.success(), "expected successful exit status");
    let bytes = std::fs::read(output_path).expect("read wasm");
    assert!(bytes.starts_with(b"\0asm"), "expected wasm magic");
}
