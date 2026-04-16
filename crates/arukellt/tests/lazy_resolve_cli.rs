//! CLI flags for lazy multi-module resolve (`--lazy-resolve` / `--no-lazy-resolve`).

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

#[test]
fn compile_help_shows_lazy_resolve_flags() {
    let output = Command::new(arukellt_bin())
        .args(["compile", "--help"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt compile --help failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "compile --help should succeed\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("--lazy-resolve") && stdout.contains("--no-lazy-resolve"),
        "compile --help should document lazy resolve flags\nstdout: {stdout}"
    );
}

#[test]
fn compile_lazy_resolve_smoke() {
    let fixture = workspace_root().join("tests/fixtures/t3/hello.ark");
    let output = Command::new(arukellt_bin())
        .args(["compile", "--lazy-resolve"])
        .arg(&fixture)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt compile --lazy-resolve failed to spawn");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "compile with --lazy-resolve should succeed for hello.ark\nstderr: {stderr}"
    );
}
