use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

struct ExampleExpectation {
    name: &'static str,
    wasm_js_builds: bool,
    wasm_wasi_builds: bool,
}

const EXAMPLE_MATRIX: &[ExampleExpectation] = &[
    ExampleExpectation {
        name: "closure.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "factorial.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "fibonacci.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "file_read.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "fizz_buzz.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "hello_world.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "infinite_iter.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "map_filter_sum.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "powers.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
    ExampleExpectation {
        name: "result_error_handling.ar",
        wasm_js_builds: false,
        wasm_wasi_builds: false,
    },
];

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

#[test]
fn matrix_lists_every_bundled_example() {
    let mut actual = fs::read_dir(example_root())
        .expect("read example dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ar"))
        .map(|path| {
            path.file_name()
                .expect("example filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    actual.sort();

    let mut expected = EXAMPLE_MATRIX
        .iter()
        .map(|example| example.name.to_owned())
        .collect::<Vec<_>>();
    expected.sort();

    assert_eq!(actual, expected, "example matrix is out of sync");
}

#[test]
fn check_command_accepts_all_bundled_examples() {
    for example in EXAMPLE_MATRIX {
        let path = example_root().join(example.name);
        let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
            .arg("check")
            .arg(&path)
            .output()
            .unwrap_or_else(|error| panic!("failed to check {}: {error}", path.display()));

        assert!(
            output.status.success(),
            "expected check success for {} but got status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn build_command_matches_bundled_example_wasm_matrix() {
    for example in EXAMPLE_MATRIX {
        assert_build_status(example, "wasm-js", example.wasm_js_builds);
        assert_build_status(example, "wasm-wasi", example.wasm_wasi_builds);
    }
}

fn assert_build_status(example: &ExampleExpectation, target: &str, expect_success: bool) {
    let path = example_root().join(example.name);
    let dir = tempdir().expect("tempdir");
    let output_file = dir.path().join(format!("{}-{target}.wasm", example.name));
    let output = Command::new(env!("CARGO_BIN_EXE_arktc"))
        .arg("build")
        .arg(&path)
        .arg("--target")
        .arg(target)
        .arg("--output")
        .arg(&output_file)
        .output()
        .unwrap_or_else(|error| panic!("failed to build {} for {target}: {error}", path.display()));

    if expect_success {
        assert!(
            output.status.success(),
            "expected build success for {} ({target}) but got status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = fs::read(&output_file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", output_file.display()));
        assert!(
            bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]),
            "expected wasm header for {} ({target})",
            path.display()
        );
    } else {
        assert!(
            !output.status.success(),
            "expected build failure for {} ({target}) but got status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not yet supported") || stderr.contains("unsupported wasm"),
            "unexpected stderr for {} ({target}): {stderr}",
            path.display()
        );
    }
}
