use std::process::Command;

fn run_help(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_chef"))
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to run chef {args:?}: {error}"))
}

#[test]
fn top_level_help_lists_public_subcommands() {
    let output = run_help(&["--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("arukellt project manager"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("test"));
    assert!(stdout.contains("build"));
    assert!(stdout.contains("benchmark"));
}

#[test]
fn run_help_describes_trace_and_function_options() {
    let output = run_help(&["run", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("Run a function through the interpreter"));
    assert!(stdout.contains("Function name to call"));
    assert!(stdout.contains("execution trace"));
    assert!(stdout.contains("stdin.read_text()"));
    assert!(stdout.contains("pipe data into `chef run`"));
}

#[test]
fn test_help_describes_snapshot_and_json_modes() {
    let output = run_help(&["test", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("snapshot-check main"));
    assert!(stdout.contains("versioned JSON"));
}

#[test]
fn benchmark_help_mentions_manifest_input() {
    let output = run_help(&["benchmark", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("benchmark manifest"));
}

#[test]
fn build_help_mentions_wasm_targets_and_emit_modes() {
    let output = run_help(&["build", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("Compile a source file to WebAssembly"));
    assert!(stdout.contains("wasm-js"));
    assert!(stdout.contains("wasm-js-gc"));
    assert!(stdout.contains("wasm-component-js"));
    assert!(stdout.contains("wat-min"));
    assert!(stdout.contains("Write the build output"));
    assert!(stdout.contains("docs/std.md#target-support-matrix"));
}
