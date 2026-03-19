use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn example_root() -> PathBuf {
    repo_root().join("example")
}

fn stdout_fixture(path: &Path) -> String {
    let fixture = path.with_extension("stdout");
    fs::read_to_string(&fixture).unwrap_or_else(|error| {
        panic!("failed to read fixture {}: {error}", fixture.display())
    })
}

#[test]
fn run_command_matches_example_stdout_fixtures() {
    let examples = [
        "hello_world.ar",
        "factorial.ar",
        "fibonacci.ar",
        "closure.ar",
        "map_filter_sum.ar",
        "result_error_handling.ar",
        "fizz_buzz.ar",
        "infinite_iter.ar",
        "file_read.ar",
    ];

    for name in examples {
        let path = example_root().join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_lang"))
            .arg("run")
            .arg(&path)
            .current_dir(example_root())
            .output()
            .unwrap_or_else(|error| panic!("failed to run {}: {error}", path.display()));

        assert!(
            output.status.success(),
            "expected success for {} but got status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
        assert_eq!(stdout, stdout_fixture(&path), "stdout mismatch for {}", name);
    }
}

#[test]
fn test_command_uses_example_stdout_snapshots() {
    let examples = [
        "hello_world.ar",
        "factorial.ar",
        "fibonacci.ar",
        "closure.ar",
        "map_filter_sum.ar",
        "result_error_handling.ar",
        "fizz_buzz.ar",
        "infinite_iter.ar",
        "file_read.ar",
    ];

    for name in examples {
        let path = example_root().join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_lang"))
            .arg("test")
            .arg(&path)
            .current_dir(example_root())
            .output()
            .unwrap_or_else(|error| panic!("failed to test {}: {error}", path.display()));

        assert!(
            output.status.success(),
            "expected success for {} but got status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
