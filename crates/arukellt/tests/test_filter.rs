//! Integration tests for `arukellt test --filter <name>`.
//!
//! Verifies:
//!   - `--filter` narrows test execution to matching test name(s)
//!   - `--filter` with no match produces "no tests found" output
//!   - `--help` shows the `--filter` flag

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

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

// A minimal .ark source with two test_ functions.
const MULTI_TEST_SOURCE: &str = r#"use std::host::stdio

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

fn test_add() {
    let result = add(2, 3)
    stdio::println(i32_to_string(result))
}

fn test_subtract() {
    let result = subtract(10, 4)
    stdio::println(i32_to_string(result))
}
"#;

// ── --filter reduces test list ──────────────────────────────────────────────

#[test]
fn filter_runs_only_matching_test() {
    let dir = tempfile::TempDir::new().unwrap();
    let ark_file = dir.path().join("multi_test.ark");
    std::fs::write(&ark_file, MULTI_TEST_SOURCE).unwrap();

    // --list --filter test_add should only list test_add, not test_subtract.
    let output = Command::new(arukellt_bin())
        .args(["test", "--list", "--filter", "test_add"])
        .arg(&ark_file)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt test --list --filter failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt test --list --filter test_add should exit 0\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("test_add"),
        "--filter test_add should include test_add; stdout={stdout}"
    );
    assert!(
        !stdout.contains("test_subtract"),
        "--filter test_add should exclude test_subtract; stdout={stdout}"
    );
}

// ── --filter with no match produces empty / no-tests output ────────────────

#[test]
fn filter_no_match_reports_no_tests() {
    let dir = tempfile::TempDir::new().unwrap();
    let ark_file = dir.path().join("multi_test.ark");
    std::fs::write(&ark_file, MULTI_TEST_SOURCE).unwrap();

    let output = Command::new(arukellt_bin())
        .args(["test", "--filter", "test_nonexistent_xyz"])
        .arg(&ark_file)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt test --filter no_match failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt test with no matching filter should exit 0\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("no tests found") || stdout.contains("0 passed"),
        "--filter with no match should report no tests; stdout={stdout}"
    );
}

// ── --help shows --filter flag ──────────────────────────────────────────────

#[test]
fn test_help_shows_filter_flag() {
    let output = Command::new(arukellt_bin())
        .args(["test", "--help"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt test --help failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--filter"),
        "arukellt test --help should show --filter flag\nstdout: {stdout}"
    );
}
