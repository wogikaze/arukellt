use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExampleExpectation {
    name: String,
    arktc_check: bool,
    chef_run: bool,
    chef_test: bool,
    wasm_js_build: bool,
    wasm_wasi_build: bool,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn example_root() -> PathBuf {
    repo_root().join("example")
}

fn example_meta_root() -> PathBuf {
    example_root().join("meta")
}

fn example_matrix() -> Vec<ExampleExpectation> {
    let path = example_meta_root().join("matrix.json");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn stdout_fixture(path: &Path) -> String {
    let fixture = example_meta_root().join(format!(
        "{}.stdout",
        path.file_stem().expect("example stem").to_string_lossy()
    ));
    fs::read_to_string(&fixture)
        .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", fixture.display()))
}

#[test]
fn run_command_matches_example_stdout_fixtures() {
    for example in example_matrix() {
        assert!(
            example.arktc_check,
            "expected arktc_check coverage for {}",
            example.name
        );
        let _ = (example.wasm_js_build, example.wasm_wasi_build);
        let path = example_root().join(&example.name);
        let output = Command::new(env!("CARGO_BIN_EXE_chef"))
            .arg("run")
            .arg(&path)
            .current_dir(example_root())
            .output()
            .unwrap_or_else(|error| panic!("failed to run {}: {error}", path.display()));

        assert_eq!(
            output.status.success(),
            example.chef_run,
            "unexpected run status for {}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
        assert_eq!(
            stdout,
            stdout_fixture(&path),
            "stdout mismatch for {}",
            example.name
        );
    }
}

#[test]
fn test_command_uses_example_stdout_snapshots() {
    for example in example_matrix() {
        let path = example_root().join(&example.name);
        let output = Command::new(env!("CARGO_BIN_EXE_chef"))
            .arg("test")
            .arg(&path)
            .current_dir(example_root())
            .output()
            .unwrap_or_else(|error| panic!("failed to test {}: {error}", path.display()));

        assert_eq!(
            output.status.success(),
            example.chef_test,
            "unexpected test status for {}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
