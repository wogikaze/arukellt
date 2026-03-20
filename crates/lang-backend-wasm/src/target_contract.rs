use std::collections::HashSet;

use lang_ir::HighModule;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmTarget {
    JavaScriptHost,
    JavaScriptHostGc,
    Wasi,
}

pub(crate) fn wasm_entry_roots(module: &HighModule, target: WasmTarget) -> HashSet<String> {
    match target {
        WasmTarget::JavaScriptHost | WasmTarget::JavaScriptHostGc => module
            .functions
            .iter()
            .map(|function| function.name.clone())
            .collect(),
        WasmTarget::Wasi => HashSet::from([String::from("main")]),
    }
}
