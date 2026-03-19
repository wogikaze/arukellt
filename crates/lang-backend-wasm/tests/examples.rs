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
    let examples = [
        "hello_world.ar",
        "closure.ar",
        "result_error_handling.ar",
        "infinite_iter.ar",
        "file_read.ar",
    ];

    for name in examples {
        let source = fs::read_to_string(example_dir.join(name)).expect("example source");
        let js_error = build_module_from_source(&source, WasmTarget::JavaScriptHost)
            .expect_err("wasm-js build should fail for unsupported example");
        let wasi_error = build_module_from_source(&source, WasmTarget::Wasi)
            .expect_err("wasm-wasi build should fail for unsupported example");
        let js_message = js_error.to_string();
        let wasi_message = wasi_error.to_string();
        assert!(
            js_message.contains("not yet supported") || js_message.contains("unsupported wasm"),
            "unexpected wasm-js error for {name}: {js_message}"
        );
        assert!(
            wasi_message.contains("not yet supported") || wasi_message.contains("unsupported wasm"),
            "unexpected wasm-wasi error for {name}: {wasi_message}"
        );
    }
}
