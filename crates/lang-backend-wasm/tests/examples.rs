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
fn builds_bundled_examples_for_both_wasm_targets() {
    let example_dir = repo_root().join("example");
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
        let source = fs::read_to_string(example_dir.join(name)).expect("example source");
        build_module_from_source(&source, WasmTarget::JavaScriptHost)
            .unwrap_or_else(|error| panic!("wasm-js build failed for {name}: {error}"));
        build_module_from_source(&source, WasmTarget::Wasi)
            .unwrap_or_else(|error| panic!("wasm-wasi build failed for {name}: {error}"));
    }
}
