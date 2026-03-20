use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use tempfile::tempdir;

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

#[test]
fn matrix_lists_every_bundled_example() {
    let matrix = example_matrix();
    let mut actual = fs::read_dir(example_root())
        .expect("read example dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ar"))
        .map(|path| {
            path.file_name()
                .expect("example filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    actual.sort();

    let mut expected = matrix
        .iter()
        .map(|example| example.name.clone())
        .collect::<Vec<_>>();
    expected.sort();

    assert_eq!(actual, expected, "example matrix is out of sync");
}

#[test]
fn check_command_accepts_all_bundled_examples() {
    for example in example_matrix() {
        assert!(
            example.chef_run,
            "expected chef_run coverage for {}",
            example.name
        );
        assert!(
            example.chef_test,
            "expected chef_test coverage for {}",
            example.name
        );
        let path = example_root().join(&example.name);
        let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
            .arg("check")
            .arg(&path)
            .output()
            .unwrap_or_else(|error| panic!("failed to check {}: {error}", path.display()));

        assert_eq!(
            output.status.success(),
            example.arktc_check,
            "unexpected check status for {}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn build_command_matches_bundled_example_wasm_matrix() {
    for example in example_matrix() {
        assert_build_status(&example, "wasm-js", example.wasm_js_build);
        assert_build_status(&example, "wasm-wasi", example.wasm_wasi_build);
    }
}

fn assert_build_status(example: &ExampleExpectation, target: &str, expect_success: bool) {
    let path = example_root().join(&example.name);
    let dir = tempdir().expect("tempdir");
    let output_file = dir.path().join(format!("{}-{target}.wasm", example.name));
    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&path)
        .arg("--target")
        .arg(target)
        .arg("--output")
        .arg(&output_file)
        .output()
        .unwrap_or_else(|error| panic!("failed to build {} for {target}: {error}", path.display()));

    if expect_success {
        assert!(
            output.status.success(),
            "expected build success for {} ({target}) but got status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = fs::read(&output_file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", output_file.display()));
        assert!(
            bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]),
            "expected wasm header for {} ({target})",
            path.display()
        );
    } else {
        assert!(
            !output.status.success(),
            "expected build failure for {} ({target}) but got status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not yet supported")
                || stderr.contains("unsupported wasm")
                || stderr.contains("known Fn<A, B>")
                || stderr.contains("E_RETURN_MISMATCH"),
            "unexpected stderr for {} ({target}): {stderr}",
            path.display()
        );
    }
}
