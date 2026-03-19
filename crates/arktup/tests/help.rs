use std::process::Command;

fn run_help(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_arktup"))
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to run arktup {args:?}: {error}"))
}

#[test]
fn top_level_help_lists_public_subcommands() {
    let output = run_help(&["--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("arukellt toolchain manager"));
    assert!(stdout.contains("show"));
    assert!(stdout.contains("install"));
    assert!(stdout.contains("default"));
}

#[test]
fn show_help_mentions_local_state_location() {
    let output = run_help(&["show", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("local toolchain state"));
    assert!(stdout.contains("ARKTUP_HOME"));
}

#[test]
fn install_help_mentions_metadata_recording() {
    let output = run_help(&["install", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("Record a locally installed toolchain version"));
    assert!(stdout.contains("version label"));
}

#[test]
fn default_help_mentions_installed_version_requirement() {
    let output = run_help(&["default", "--help"]);
    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("previously installed version"));
    assert!(stdout.contains("default"));
}
