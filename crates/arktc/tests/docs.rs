use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct DocExample {
    id: String,
    doc: String,
    fixture: String,
    mode: String,
    error_code: Option<String>,
    error_substring: Option<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn load_manifest() -> Vec<DocExample> {
    let path = repo_root().join("docs/examples/manifest.json");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&source).expect("docs examples manifest")
}

fn read_repo_file(path: &str) -> String {
    let full_path = repo_root().join(path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", full_path.display()))
}

fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_string()
}

fn extract_snippet(doc: &str, id: &str) -> String {
    let marker = format!("<!-- snippet: {id} -->");
    let (_, after_marker) = doc
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing snippet marker {marker}"));
    let fence_start = after_marker
        .find("```")
        .unwrap_or_else(|| panic!("missing fenced block after {marker}"));
    let after_fence = &after_marker[fence_start + 3..];
    let newline = after_fence
        .find('\n')
        .unwrap_or_else(|| panic!("missing fenced block body after {marker}"));
    let body = &after_fence[newline + 1..];
    let fence_end = body
        .find("\n```")
        .unwrap_or_else(|| panic!("missing closing fence for {marker}"));
    normalized(&body[..fence_end])
}

fn run_arktfmt(path: &Path) -> String {
    let output = Command::new(env!("CARGO"))
        .current_dir(repo_root())
        .args(["run", "-p", "arktfmt", "--quiet", "--"])
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("failed to format {}: {error}", path.display()));
    assert!(
        output.status.success(),
        "expected fmt success for {}\nstdout:\n{}\nstderr:\n{}",
        path.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    normalized(&String::from_utf8(output.stdout).expect("utf8 fmt output"))
}

fn assert_success(output: &std::process::Output, description: &str) {
    assert!(
        output.status.success(),
        "expected success for {description}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn docs_snippets_match_fixtures_and_formatter_accepts_examples() {
    let manifest = load_manifest();

    for example in &manifest {
        let doc = read_repo_file(&example.doc);
        let snippet = extract_snippet(&doc, &example.id);
        let fixture = read_repo_file(&example.fixture);
        assert_eq!(
            snippet,
            normalized(&fixture),
            "markdown snippet drift for {}",
            example.id
        );

        if example.mode != "check_fail" {
            let _ = run_arktfmt(&repo_root().join(&example.fixture));
        }
    }
}

#[test]
fn docs_examples_compile_or_fail_as_documented() {
    let manifest = load_manifest();

    for example in &manifest {
        let path = repo_root().join(&example.fixture);
        match example.mode.as_str() {
            "check_ok" => {
                let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
                    .arg("check")
                    .arg(&path)
                    .output()
                    .unwrap_or_else(|error| panic!("failed to check {}: {error}", path.display()));
                assert_success(&output, &format!("arktc check {}", path.display()));
            }
            "check_fail" => {
                let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
                    .arg("check")
                    .arg("--json")
                    .arg(&path)
                    .output()
                    .unwrap_or_else(|error| panic!("failed to check {}: {error}", path.display()));
                assert!(
                    !output.status.success(),
                    "expected check failure for {}\nstdout:\n{}\nstderr:\n{}",
                    path.display(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                let json: Value = serde_json::from_slice(&output.stdout).expect("json diagnostics");
                assert_eq!(json["version"], "v0.1");
                assert_eq!(
                    json["diagnostics"][0]["code"],
                    example.error_code.as_deref().expect("error code")
                );
            }
            "build_wasi_ok" | "build_js_ok" => {
                let dir = tempdir().expect("tempdir");
                let target = if example.mode == "build_wasi_ok" {
                    "wasm-wasi"
                } else {
                    "wasm-js"
                };
                let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
                    .arg("build")
                    .arg(&path)
                    .arg("--target")
                    .arg(target)
                    .arg("--output")
                    .arg(dir.path().join("out.wasm"))
                    .output()
                    .unwrap_or_else(|error| panic!("failed to build {}: {error}", path.display()));
                assert_success(&output, &format!("arktc build {target} {}", path.display()));
            }
            "build_wasi_fail" | "build_js_fail" => {
                let dir = tempdir().expect("tempdir");
                let target = if example.mode == "build_wasi_fail" {
                    "wasm-wasi"
                } else {
                    "wasm-js"
                };
                let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
                    .arg("build")
                    .arg(&path)
                    .arg("--target")
                    .arg(target)
                    .arg("--output")
                    .arg(dir.path().join("out.wasm"))
                    .output()
                    .unwrap_or_else(|error| panic!("failed to build {}: {error}", path.display()));
                assert!(
                    !output.status.success(),
                    "expected wasm build failure for {}\nstdout:\n{}\nstderr:\n{}",
                    path.display(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
                assert!(
                    stderr.contains(
                        example
                            .error_substring
                            .as_deref()
                            .expect("wasm error substring")
                    ),
                    "unexpected wasm rejection message: {stderr}"
                );
            }
            "run" | "test_ok" => {}
            other => panic!("unsupported docs manifest mode: {other}"),
        }
    }
}

#[test]
fn docs_manifest_includes_a_positive_js_build_example() {
    let manifest = load_manifest();
    let example = manifest
        .iter()
        .find(|example| example.mode == "build_js_ok")
        .expect("missing build_js_ok example");

    assert_eq!(example.id, "std-wasm-js-scalar");
    assert_eq!(example.doc, "docs/std.md");
    assert_eq!(example.fixture, "docs/examples/std/06-wasm-js-scalar.ar");

    let doc = read_repo_file("docs/std.md");
    assert!(doc.contains("<!-- snippet: std-wasm-js-scalar -->"));
}

#[test]
fn docs_manifest_includes_a_fieldless_match_wasm_example() {
    let manifest = load_manifest();
    let example = manifest
        .iter()
        .find(|example| example.id == "std-wasm-js-fieldless-match")
        .expect("missing fieldless match wasm example");

    assert_eq!(example.doc, "docs/std.md");
    assert_eq!(
        example.fixture,
        "docs/examples/std/07-wasm-js-fieldless-match.ar"
    );
    assert_eq!(example.mode, "build_js_ok");

    let doc = read_repo_file("docs/std.md");
    assert!(doc.contains("<!-- snippet: std-wasm-js-fieldless-match -->"));
}
