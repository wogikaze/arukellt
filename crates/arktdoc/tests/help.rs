use std::process::Command;

#[test]
fn help_mentions_json_only_output_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_arktdoc"))
        .arg("--help")
        .output()
        .expect("run arktdoc --help");

    assert!(output.status.success(), "expected help success");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("arukellt documentation generator"));
    assert!(stdout.contains("only json is implemented today"));
    assert!(stdout.contains("json"));
    assert!(stdout.contains("markdown"));
}
