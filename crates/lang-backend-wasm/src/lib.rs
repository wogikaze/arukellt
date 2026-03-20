pub(crate) use std::collections::{HashMap, HashSet};

pub(crate) use anyhow::{Result, anyhow, bail};
pub(crate) use lang_core::{Pattern, Type, compile_module};
pub(crate) use lang_ir::{
    HighExpr, HighExprKind, HighMatchArm, HighModule, WasmFunction, WasmFunctionBody, WasmModule,
    lower_to_high_ir, lower_to_wasm_ir, optimize_high_module,
};

mod abi_layout;
mod closure_callback_lowering;
mod helper_usage_analysis;
mod postprocess;
mod runtime_helpers;
mod target_contract;
mod wat_emitter;

pub use target_contract::WasmTarget;
pub use wat_emitter::emit_wat;

pub(crate) use abi_layout::*;
pub(crate) use closure_callback_lowering::*;
pub(crate) use helper_usage_analysis::*;
pub(crate) use runtime_helpers::*;
pub(crate) use target_contract::*;

pub fn build_module_from_source(source: &str, target: WasmTarget) -> Result<Vec<u8>> {
    let result = compile_module(source);
    if result.error_count() > 0 {
        bail!("{}", serde_json::to_string_pretty(&result.to_json()?)?);
    }
    let typed = result
        .module
        .ok_or_else(|| anyhow!("typed module missing"))?;
    let high = lower_to_high_ir(&typed);
    emit_wasm(&high, target)
}

pub fn emit_wasm(module: &HighModule, target: WasmTarget) -> Result<Vec<u8>> {
    let bytes = wat::parse_str(&emit_wat(module, target)?)?;
    postprocess::postprocess_wasm(&bytes)
}

pub fn postprocess_wasm(bytes: &[u8]) -> Result<Vec<u8>> {
    postprocess::postprocess_wasm(bytes)
}
