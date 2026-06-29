//! Integration tests for package/workspace/manifest functionality.
//!
//! Tests `arukellt build` with ark.toml project structure,
//! manifest validation, and workspace resolution.

use std::path::Path;
use std::process::Command;

fn arukellt_bin() -> std::path::PathBuf {
    let bin = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("debug")
        .join("arukellt");
    assert!(bin.exists(), "arukellt binary not found at {bin:?}");
    bin
}

#[test]
fn basic_project_builds() {
    let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("package-workspace")
        .join("basic-project");

    let output = Command::new(arukellt_bin())
        .arg("build")
        .current_dir(&project_dir)
        .output()
        .expect("failed to run arukellt build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "arukellt build failed in basic-project:\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("Compiled"),
        "expected 'Compiled' in stderr: {stderr}"
    );

    // Verify the output binary exists
    let wasm = project_dir.join("basic-project.wasm");
    assert!(wasm.exists(), "expected basic-project.wasm to be created");
    std::fs::remove_file(&wasm).ok();
}

#[test]
fn basic_project_runs() {
    let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("package-workspace")
        .join("basic-project");

    let output = Command::new(arukellt_bin())
        .arg("run")
        .arg(project_dir.join("src").join("main.ark"))
        .output()
        .expect("failed to run arukellt run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout.trim(), "ok");
}

#[test]
fn missing_ark_toml_gives_error() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let output = Command::new(arukellt_bin())
        .arg("build")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run arukellt build");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ark.toml not found") || stderr.contains("not found"),
        "expected ark.toml error, got: {stderr}"
    );
}

#[test]
fn invalid_toml_gives_error() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    std::fs::write(tmp.path().join("ark.toml"), "this is not valid toml [[[")
        .expect("failed to write");
    let output = Command::new(arukellt_bin())
        .arg("build")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run arukellt build");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("TOML") || stderr.contains("toml") || stderr.contains("parse"),
        "expected TOML parse error, got: {stderr}"
    );
}

#[test]
fn missing_package_name_gives_error() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    std::fs::write(
        tmp.path().join("ark.toml"),
        "[package]\nversion = \"0.1.0\"\n",
    )
    .expect("failed to write");
    let output = Command::new(arukellt_bin())
        .arg("build")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run arukellt build");

    assert!(!output.status.success());
}

#[test]
fn init_creates_project() {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let project_dir = tmp.path().join("test-project");

    let output = Command::new(arukellt_bin())
        .arg("init")
        .arg(&project_dir)
        .output()
        .expect("failed to run arukellt init");

    assert!(
        output.status.success(),
        "arukellt init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        project_dir.join("ark.toml").exists(),
        "ark.toml not created"
    );
    assert!(
        project_dir.join("src").join("main.ark").exists(),
        "src/main.ark not created"
    );

    // Verify ark.toml is valid TOML with required fields
    let manifest_content =
        std::fs::read_to_string(project_dir.join("ark.toml")).expect("can't read ark.toml");
    assert!(
        manifest_content.contains("[package]"),
        "ark.toml missing [package] section"
    );
    assert!(
        manifest_content.contains("name"),
        "ark.toml missing name field"
    );
}
