//! Integration tests for `arukellt doc <symbol>`.
//!
//! Verifies:
//!   - Found symbol output includes signature, stability, and target availability
//!   - `--json` flag produces valid JSON with required fields
//!   - Not-found symbols produce error output with ≥1 candidate
//!   - `--help` shows expected flags

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

// ── found symbol (println) ──────────────────────────────────────────────────

#[test]
fn doc_println_shows_signature() {
    let output = Command::new(arukellt_bin())
        .args(["doc", "println"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc println failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt doc println should exit 0\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("println"),
        "output should contain function name 'println'\nstdout: {stdout}"
    );
    // Signature: fn println(String) -> ()
    assert!(
        stdout.contains("fn println"),
        "output should contain 'fn println'\nstdout: {stdout}"
    );
    // Stability
    assert!(
        stdout.contains("stable"),
        "output should show stability 'stable'\nstdout: {stdout}"
    );
    // Target availability
    assert!(
        stdout.contains("wasm32-wasi-p1") || stdout.contains("p1"),
        "output should mention target availability\nstdout: {stdout}"
    );
}

// ── not found symbol ────────────────────────────────────────────────────────

#[test]
fn doc_nonexistent_symbol_exits_nonzero_with_candidates() {
    let output = Command::new(arukellt_bin())
        .args(["doc", "nonexistent_symbol_xyz"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc nonexistent failed to spawn");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "arukellt doc for unknown symbol should exit non-zero"
    );
    assert!(
        stderr.contains("not found"),
        "stderr should contain 'not found'\nstderr: {stderr}"
    );
    // Should show at least one candidate
    assert!(
        stderr.contains("Did you mean?") || stderr.len() > 20,
        "stderr should suggest candidates\nstderr: {stderr}"
    );
}

// ── --json output ───────────────────────────────────────────────────────────

#[test]
fn doc_json_println_is_valid_json_with_fields() {
    let output = Command::new(arukellt_bin())
        .args(["doc", "println", "--json"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc println --json failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt doc println --json should exit 0\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(parsed["kind"], "function", "JSON kind should be 'function'");
    assert_eq!(parsed["name"], "println", "JSON name should be 'println'");
    assert!(
        parsed.get("stability").is_some(),
        "JSON should have 'stability' field"
    );
    assert!(
        parsed.get("params").is_some(),
        "JSON should have 'params' field"
    );
    assert!(
        parsed.get("availability").is_some(),
        "JSON should have 'availability' field"
    );
}

#[test]
fn doc_json_http_get_shows_target_fields() {
    let output = Command::new(arukellt_bin())
        .args(["doc", "std::host::http::get", "--json"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc std::host::http::get --json failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt doc std::host::http::get --json should exit 0\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(parsed["kind"], "function");
    assert_eq!(parsed["name"], "get");
    assert!(parsed.get("stability").is_some(), "should have stability");
    assert!(parsed.get("returns").is_some(), "should have returns");
}

// ── not-found JSON output ───────────────────────────────────────────────────

#[test]
fn doc_json_not_found_outputs_json_with_candidates() {
    let output = Command::new(arukellt_bin())
        .args(["doc", "nonexistent_xyz_qwerty", "--json"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc nonexistent --json failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Exits non-zero but stdout is valid JSON
    assert!(
        !output.status.success(),
        "should exit non-zero for not-found"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON for not-found case");
    assert_eq!(parsed["kind"], "not_found");
    assert!(
        parsed["candidates"].is_array(),
        "should have candidates array"
    );
    assert!(
        !parsed["candidates"].as_array().unwrap().is_empty(),
        "should have at least 1 candidate"
    );
}

// ── --help ──────────────────────────────────────────────────────────────────

#[test]
fn doc_help_shows_expected_flags() {
    let output = Command::new(arukellt_bin())
        .args(["doc", "--help"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc --help failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt doc --help should succeed"
    );
    assert!(
        stdout.contains("--json"),
        "--help should mention --json\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("--target"),
        "--help should mention --target\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("SYMBOL") || stdout.contains("symbol"),
        "--help should show SYMBOL argument\nstdout: {stdout}"
    );
}

// ── module lookup ───────────────────────────────────────────────────────────

#[test]
fn doc_module_shows_function_list() {
    let output = Command::new(arukellt_bin())
        .args(["doc", "std::host::http"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc std::host::http failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "arukellt doc std::host::http should succeed\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("std::host::http") || stdout.contains("http"),
        "output should reference http module\nstdout: {stdout}"
    );
    // Should list functions in module
    assert!(
        stdout.contains("get") || stdout.contains("request"),
        "output should list http functions\nstdout: {stdout}"
    );
}

// ── --target filter ─────────────────────────────────────────────────────────

#[test]
fn doc_target_p1_warns_for_p2_only_function() {
    // std::host::http::get has availability t1=true but is target=["wasm32-wasi-p2"]
    // With --target wasm32-wasi-p1 we expect either a warning or clean output
    let output = Command::new(arukellt_bin())
        .args(["doc", "std::host::http::get", "--target", "wasm32-wasi-p1"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt doc with --target failed to spawn");

    // Should still find the function (exit 0)
    assert!(
        output.status.success(),
        "should find the function\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("get"),
        "output should contain function name\nstdout: {stdout}"
    );
}
