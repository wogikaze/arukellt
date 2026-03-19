use std::fs;
use std::path::PathBuf;

use lang_backend_wasm::{WasmTarget, build_module_from_source};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn rejects_unsupported_bundled_examples_for_both_wasm_targets() {
    let example_dir = repo_root().join("example");
    // These examples still use features outside the current wasm subset.
    // that are not yet supported on either target.
    let examples: [&str; 0] = [];

    for name in examples {
        let source = fs::read_to_string(example_dir.join(name)).expect("example source");
        let js_error = build_module_from_source(&source, WasmTarget::JavaScriptHost)
            .expect_err("wasm-js build should fail for unsupported example");
        let wasi_error = build_module_from_source(&source, WasmTarget::Wasi)
            .expect_err("wasm-wasi build should fail for unsupported example");
        let js_message = js_error.to_string();
        let wasi_message = wasi_error.to_string();
        assert!(
            js_message.contains("not yet supported")
                || js_message.contains("unsupported wasm")
                || js_message.contains("known Fn<A, B>")
                || js_message.contains("E_RETURN_MISMATCH")
                || js_message.contains("Unknown")
                || js_message.contains("Result<")
                || js_message.contains("match is not yet supported"),
            "unexpected wasm-js error for {name}: {js_message}"
        );
        assert!(
            wasi_message.contains("not yet supported")
                || wasi_message.contains("unsupported wasm")
                || wasi_message.contains("known Fn<A, B>")
                || wasi_message.contains("E_RETURN_MISMATCH")
                || wasi_message.contains("Unknown")
                || wasi_message.contains("Result<")
                || wasi_message.contains("match is not yet supported"),
            "unexpected wasm-wasi error for {name}: {wasi_message}"
        );
    }
}

#[test]
fn wasi_console_println_examples_build_and_have_correct_magic() {
    let example_dir = repo_root().join("example");
    // These examples use console.println (and optionally string()/closures) which are now
    // supported on the wasm-wasi target.
    let examples = [
        "closure.ar",
        "infinite_iter.ar",
        "map_filter_sum.ar",
        "powers.ar",
        "fizz_buzz.ar",
        "file_read.ar",
        "result_error_handling.ar",
        "hello_world.ar",
        "factorial.ar",
        "fibonacci.ar",
    ];

    for name in examples {
        let source = fs::read_to_string(example_dir.join(name)).expect("example source");

        // wasm-js still doesn't support console.println
        build_module_from_source(&source, WasmTarget::JavaScriptHost)
            .expect_err(&format!("wasm-js build should still fail for {name}"));

        // wasm-wasi should now succeed
        let bytes = build_module_from_source(&source, WasmTarget::Wasi)
            .unwrap_or_else(|e| panic!("wasm-wasi build should succeed for {name}: {e}"));
        assert!(
            bytes.starts_with(&[0x00, 0x61, 0x73, 0x6d]),
            "expected wasm magic for {name}"
        );
        // The WASM module must export _start (WASI command ABI)
        assert!(
            bytes.windows(6).any(|w| w == b"_start"),
            "expected _start export in wasm bytes for {name}"
        );
    }
}
