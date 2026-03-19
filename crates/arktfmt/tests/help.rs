use std::process::Command;

#[test]
fn help_describes_write_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_arktfmt"))
        .arg("--help")
        .output()
        .expect("run arktfmt --help");

    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("arukellt source formatter"));
    assert!(stdout.contains("Write the formatter output back"));
}
