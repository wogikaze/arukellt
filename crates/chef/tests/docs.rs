use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DocExample {
    id: String,
    doc: String,
    fixture: String,
    mode: String,
    stdout_fixture: Option<String>,
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

fn fixture_dir(path: &Path) -> &Path {
    path.parent()
        .unwrap_or_else(|| panic!("missing parent for {}", path.display()))
}

#[test]
fn docs_snippets_match_fixtures() {
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
    }
}

#[test]
fn docs_examples_run_and_test_as_documented() {
    let manifest = load_manifest();

    for example in &manifest {
        let path = repo_root().join(&example.fixture);
        match example.mode.as_str() {
            "run" => {
                let output = Command::new(env!("CARGO_BIN_EXE_chef"))
                    .arg("run")
                    .arg(&path)
                    .current_dir(fixture_dir(&path))
                    .output()
                    .unwrap_or_else(|error| panic!("failed to run {}: {error}", path.display()));
                assert!(
                    output.status.success(),
                    "expected run success for {}\nstdout:\n{}\nstderr:\n{}",
                    path.display(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
                assert_eq!(
                    normalized(&stdout),
                    normalized(&read_repo_file(
                        example.stdout_fixture.as_deref().expect("stdout fixture")
                    )),
                    "stdout mismatch for {}",
                    example.id
                );
            }
            "test_ok" => {
                let output = Command::new(env!("CARGO_BIN_EXE_chef"))
                    .arg("test")
                    .arg(&path)
                    .current_dir(fixture_dir(&path))
                    .output()
                    .unwrap_or_else(|error| panic!("failed to test {}: {error}", path.display()));
                assert!(
                    output.status.success(),
                    "expected test success for {}\nstdout:\n{}\nstderr:\n{}",
                    path.display(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            _ => {}
        }
    }
}
