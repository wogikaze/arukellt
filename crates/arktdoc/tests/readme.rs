use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn readme_arktdoc_command_example_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_arktdoc"))
        .arg(repo_root().join("example/hello_world.ar"))
        .arg("--format")
        .arg("json")
        .output()
        .expect("run arktdoc");

    assert!(output.status.success(), "expected successful exit status");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["version"], "v0.1");
    assert_eq!(json["functions"][0]["name"], "main");
}
