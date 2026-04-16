//! Integration tests for component-model CLI workflows.

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

    fn component_cli_available() -> bool {
        Command::new("wasm-tools")
        .arg("--version")
        .output()
        .is_ok()
    }

fn write_wit_import_fixture(dir: &tempfile::TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    let ark_file = dir.path().join("host_import.ark");
    let wit_file = dir.path().join("host.wit");

    std::fs::write(
        &ark_file,
        "fn main() {\n    add(1, 2)\n}\n",
    )
    .unwrap();
    std::fs::write(
        &wit_file,
        "package test:host;\n\ninterface host-fns {\n    add: func(a: s32, b: s32) -> s32;\n}\n",
    )
    .unwrap();

    (ark_file, wit_file)
}

#[test]
fn component_help_shows_subcommands() {
    let output = Command::new(arukellt_bin())
        .args(["component", "--help"])
        .current_dir(workspace_root())
        .output()
        .expect("arukellt component --help failed to spawn");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "arukellt component --help should succeed");
    assert!(
        stdout.contains("build"),
        "component --help should list the build subcommand\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("inspect"),
        "component --help should list the inspect subcommand\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("validate"),
        "component --help should list the validate subcommand\nstdout: {stdout}"
    );
}

#[test]
fn compile_component_accepts_wit_import_calls() {
    if !component_cli_available() {
        eprintln!("skipping component CLI test: wasm-tools not installed");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let (ark_file, wit_file) = write_wit_import_fixture(&dir);

    let output = Command::new(arukellt_bin())
        .args(["compile", "--target", "wasm32-wasi-p2", "--emit", "component", "--wit"])
        .arg(&wit_file)
        .arg(&ark_file)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt compile --emit component failed to spawn");

    assert!(
        output.status.success(),
        "component compile with --wit should succeed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        ark_file.with_extension("component.wasm").exists(),
        "component output should be written next to the source file"
    );
}

#[test]
fn emit_all_preserves_wit_imports_for_component_output() {
    if !component_cli_available() {
        eprintln!("skipping component CLI test: wasm-tools not installed");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let (ark_file, wit_file) = write_wit_import_fixture(&dir);

    let output = Command::new(arukellt_bin())
        .args(["compile", "--target", "wasm32-wasi-p2", "--emit", "all", "--wit"])
        .arg(&wit_file)
        .arg(&ark_file)
        .current_dir(workspace_root())
        .output()
        .expect("arukellt compile --emit all failed to spawn");

    assert!(
        output.status.success(),
        "emit all with --wit should succeed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        ark_file.with_extension("wasm").exists(),
        "core Wasm output should be written for --emit all"
    );
    assert!(
        ark_file.with_extension("component.wasm").exists(),
        "component output should be written for --emit all"
    );
}