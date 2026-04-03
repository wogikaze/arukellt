//! Integration tests for `arukellt init --template`.
//!
//! Verifies that each template variant generates valid project files
//! and that `arukellt check` (and `arukellt test` for with-tests) pass
//! on the generated projects.

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

/// Workspace root (where `std/` lives) — used as `current_dir` for check/test
/// subprocesses so the stdlib resolver can find `std/host/*.ark`.
fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn run_init(dir: &tempfile::TempDir, template: &str) {
    let status = Command::new(arukellt_bin())
        .args(["init", dir.path().to_str().unwrap(), "--template", template])
        .status()
        .unwrap_or_else(|e| panic!("arukellt init failed to spawn: {e}"));
    assert!(
        status.success(),
        "arukellt init --template {template} exited non-zero"
    );
}

// --- minimal template ---

#[test]
fn template_minimal_generates_files() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "minimal");

    assert!(
        dir.path().join("ark.toml").exists(),
        "ark.toml should exist"
    );
    assert!(
        dir.path().join("src").join("main.ark").exists(),
        "src/main.ark should exist"
    );

    let main_ark = std::fs::read_to_string(dir.path().join("src").join("main.ark")).unwrap();
    assert!(
        main_ark.contains("Hello, Arukellt!"),
        "minimal template should contain Hello, Arukellt!"
    );
}

#[test]
fn template_minimal_check_passes() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "minimal");

    // Pass absolute path; use workspace root as CWD so `std/` is resolvable.
    let main_ark = dir.path().join("src").join("main.ark");
    let output = Command::new(arukellt_bin())
        .arg("check")
        .arg(&main_ark)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt check failed to spawn");

    assert!(
        output.status.success(),
        "arukellt check failed for minimal template:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn template_minimal_default_no_flag_equivalent() {
    // `arukellt init` without --template should produce the same structure as minimal
    let dir_default = tempfile::TempDir::new().unwrap();
    let dir_minimal = tempfile::TempDir::new().unwrap();

    let status = Command::new(arukellt_bin())
        .args(["init", dir_default.path().to_str().unwrap()])
        .status()
        .unwrap_or_else(|e| panic!("arukellt init (no flag) failed: {e}"));
    assert!(status.success(), "arukellt init (no flag) should succeed");

    run_init(&dir_minimal, "minimal");

    let main_default =
        std::fs::read_to_string(dir_default.path().join("src").join("main.ark")).unwrap();
    let main_minimal =
        std::fs::read_to_string(dir_minimal.path().join("src").join("main.ark")).unwrap();
    assert_eq!(
        main_default, main_minimal,
        "default init and --template minimal should generate identical src/main.ark"
    );
}

// --- cli template ---

#[test]
fn template_cli_generates_files() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "cli");

    assert!(
        dir.path().join("ark.toml").exists(),
        "ark.toml should exist"
    );
    assert!(
        dir.path().join("src").join("main.ark").exists(),
        "src/main.ark should exist"
    );

    let main_ark = std::fs::read_to_string(dir.path().join("src").join("main.ark")).unwrap();
    assert!(
        main_ark.contains("fn greet"),
        "cli template should contain greet function"
    );
    assert!(
        main_ark.contains("process::exit"),
        "cli template should contain process::exit"
    );
}

#[test]
fn template_cli_check_passes() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "cli");

    let main_ark = dir.path().join("src").join("main.ark");
    let output = Command::new(arukellt_bin())
        .arg("check")
        .arg(&main_ark)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt check failed to spawn");

    assert!(
        output.status.success(),
        "arukellt check failed for cli template:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

// --- with-tests template ---

#[test]
fn template_with_tests_generates_files() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "with-tests");

    assert!(
        dir.path().join("ark.toml").exists(),
        "ark.toml should exist"
    );
    assert!(
        dir.path().join("src").join("main.ark").exists(),
        "src/main.ark should exist"
    );

    let main_ark = std::fs::read_to_string(dir.path().join("src").join("main.ark")).unwrap();
    assert!(
        main_ark.contains("fn test_"),
        "with-tests template should contain at least one test_ function"
    );
    assert!(
        main_ark.contains("fn add"),
        "with-tests template should contain add function"
    );
}

#[test]
fn template_with_tests_check_passes() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "with-tests");

    let main_ark = dir.path().join("src").join("main.ark");
    let output = Command::new(arukellt_bin())
        .arg("check")
        .arg(&main_ark)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt check failed to spawn");

    assert!(
        output.status.success(),
        "arukellt check failed for with-tests template:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn template_with_tests_test_command_finds_tests() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "with-tests");

    // `arukellt test --list src/main.ark` should list test functions.
    // Use absolute path; workspace root as CWD so stdlib resolves.
    let main_ark = dir.path().join("src").join("main.ark");
    let output = Command::new(arukellt_bin())
        .arg("test")
        .arg("--list")
        .arg(&main_ark)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt test --list failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test_add"),
        "with-tests template should have test_add discoverable; stdout={stdout}"
    );
    assert!(
        stdout.contains("test_subtract"),
        "with-tests template should have test_subtract discoverable; stdout={stdout}"
    );
}

// --- wasi-host template ---

#[test]
fn template_wasi_host_generates_files() {
    let dir = tempfile::TempDir::new().unwrap();
    run_init(&dir, "wasi-host");

    assert!(
        dir.path().join("ark.toml").exists(),
        "ark.toml should exist"
    );
    assert!(
        dir.path().join("src").join("main.ark").exists(),
        "src/main.ark should exist"
    );

    let main_ark = std::fs::read_to_string(dir.path().join("src").join("main.ark")).unwrap();
    assert!(
        main_ark.contains("std::host::stdio"),
        "wasi-host template should use std::host::stdio"
    );

    let ark_toml = std::fs::read_to_string(dir.path().join("ark.toml")).unwrap();
    assert!(
        ark_toml.contains("wasm32-wasi-p2"),
        "wasi-host ark.toml should reference wasm32-wasi-p2 target"
    );
}

// --- error: init in existing project ---

#[test]
fn init_fails_if_ark_toml_exists() {
    let dir = tempfile::TempDir::new().unwrap();
    // Create ark.toml first
    std::fs::write(dir.path().join("ark.toml"), "[package]\nname = \"x\"\n").unwrap();

    let status = Command::new(arukellt_bin())
        .args(["init", dir.path().to_str().unwrap()])
        .status()
        .unwrap_or_else(|e| panic!("arukellt init failed to spawn: {e}"));

    assert!(
        !status.success(),
        "arukellt init should fail when ark.toml already exists"
    );
}

// --- error: invalid template name ---

#[test]
fn init_fails_with_invalid_template() {
    let dir = tempfile::TempDir::new().unwrap();
    let status = Command::new(arukellt_bin())
        .args([
            "init",
            dir.path().to_str().unwrap(),
            "--template",
            "invalid_template_name",
        ])
        .status()
        .unwrap_or_else(|e| panic!("arukellt init failed to spawn: {e}"));

    assert!(
        !status.success(),
        "arukellt init --template invalid_template_name should fail with clap error"
    );
}
