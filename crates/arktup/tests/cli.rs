use std::process::Command;

use tempfile::tempdir;

fn run_arktup(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_arktup"))
        .env("ARKTUP_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to run arktup {args:?}: {error}"))
}

#[test]
fn show_reports_an_empty_local_toolchain_state() {
    let dir = tempdir().expect("tempdir");

    let output = run_arktup(dir.path(), &["show"]);

    assert!(
        output.status.success(),
        "expected show success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(
        stdout,
        format!(
            "arktup {}\nhome: {}\ndefault: <none>\ninstalled: <none>\n",
            env!("CARGO_PKG_VERSION"),
            dir.path().display()
        )
    );
}

#[test]
fn install_and_default_persist_local_toolchain_state() {
    let dir = tempdir().expect("tempdir");

    let install = run_arktup(dir.path(), &["install", "v0.1.0"]);
    assert!(
        install.status.success(),
        "expected install success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );
    assert_eq!(
        String::from_utf8(install.stdout).expect("utf8 stdout"),
        "installed toolchain: v0.1.0\n"
    );

    let default = run_arktup(dir.path(), &["default", "v0.1.0"]);
    assert!(
        default.status.success(),
        "expected default success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&default.stdout),
        String::from_utf8_lossy(&default.stderr)
    );
    assert_eq!(
        String::from_utf8(default.stdout).expect("utf8 stdout"),
        "default toolchain: v0.1.0\n"
    );

    let show = run_arktup(dir.path(), &["show"]);
    assert!(
        show.status.success(),
        "expected show success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&show.stdout),
        String::from_utf8_lossy(&show.stderr)
    );
    let stdout = String::from_utf8(show.stdout).expect("utf8 stdout");
    assert_eq!(
        stdout,
        format!(
            "arktup {}\nhome: {}\ndefault: v0.1.0\ninstalled:\n- v0.1.0\n",
            env!("CARGO_PKG_VERSION"),
            dir.path().display()
        )
    );
}

#[test]
fn default_rejects_versions_that_were_not_installed() {
    let dir = tempdir().expect("tempdir");

    let output = run_arktup(dir.path(), &["default", "v9.9.9"]);

    assert!(
        !output.status.success(),
        "expected default failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("utf8 stderr"),
        "arktup: cannot set default to `v9.9.9` because it is not installed\n"
    );
}
