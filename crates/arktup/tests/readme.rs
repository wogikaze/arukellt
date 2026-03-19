use std::process::Command;

use tempfile::tempdir;

#[test]
fn readme_show_command_reports_empty_state() {
    let dir = tempdir().expect("tempdir");

    let output = Command::new(env!("CARGO_BIN_EXE_arktup"))
        .env("ARKTUP_HOME", dir.path())
        .arg("show")
        .output()
        .expect("run arktup show");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("default: <none>"));
    assert!(stdout.contains("installed: <none>"));
}

#[test]
fn readme_install_and_default_commands_update_local_state() {
    let dir = tempdir().expect("tempdir");

    let install = Command::new(env!("CARGO_BIN_EXE_arktup"))
        .env("ARKTUP_HOME", dir.path())
        .arg("install")
        .arg("v0.1.0")
        .output()
        .expect("run arktup install");
    assert!(install.status.success(), "expected successful install");

    let default = Command::new(env!("CARGO_BIN_EXE_arktup"))
        .env("ARKTUP_HOME", dir.path())
        .arg("default")
        .arg("v0.1.0")
        .output()
        .expect("run arktup default");
    assert!(default.status.success(), "expected successful default");

    let show = Command::new(env!("CARGO_BIN_EXE_arktup"))
        .env("ARKTUP_HOME", dir.path())
        .arg("show")
        .output()
        .expect("run arktup show");
    assert!(show.status.success(), "expected successful show");
    let stdout = String::from_utf8(show.stdout).expect("utf8 stdout");
    assert!(stdout.contains("default: v0.1.0"));
    assert!(stdout.contains("- v0.1.0"));
}
