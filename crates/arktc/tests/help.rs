use std::process::Command;

fn run_help(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_arktc"))
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to run arktc {args:?}: {error}"))
}

#[test]
fn top_level_help_lists_public_subcommands() {
    let output = run_help(&["--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("arukellt compiler"));
    assert!(stdout.contains("check"));
    assert!(stdout.contains("build"));
}

#[test]
fn check_help_documents_json_output() {
    let output = run_help(&["check", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("Parse and typecheck a source file"));
    assert!(stdout.contains("structured diagnostics JSON"));
}

#[test]
fn build_help_mentions_supported_wasm_subset() {
    let output = run_help(&["build", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("Compile a source file to WebAssembly"));
    assert!(stdout.contains("supported prototype subset"));
    assert!(stdout.contains("--emit"));
    assert!(stdout.contains("wat-min"));
    assert!(stdout.contains("wat"));
    assert!(stdout.contains("wasm-js"));
    assert!(stdout.contains("wasm-wasi"));
}
