//! Per-target Wasm emitter dispatch.
//!
//! Routes MIR emission through the appropriate backend based on `TargetId`.
//! T1: linear memory + WASI p1 (fully implemented).
//! T2: freestanding Wasm GC scaffold (minimal core Wasm module only).
//! T3: Wasm GC + WASI p2 (in progress — currently delegates to T1).

pub mod t1;
pub use t1 as t1_wasm32_p1;
pub mod t2_freestanding;
pub mod t3_wasm_gc;

use ark_diagnostics::{
    DiagnosticSink, wasi_p2_native_emit_limitation_diagnostic, wasm_validation_diagnostic,
};
use ark_mir::mir::MirModule;
use ark_target::{
    BackendPlan, EmitCapability, EmitKind, RuntimeModel, TargetId, WasiVersion,
    build_backend_plan,
};

/// Validate generated Wasm module bytes using `wasmparser`.
///
/// Returns `Ok(())` if the module is valid, or an error string describing the
/// validation failure.
fn validate_wasm(bytes: &[u8]) -> Result<(), String> {
    let mut validator = wasmparser::Validator::new();
    let result = validator
        .validate_all(bytes)
        .map(|_| ())
        .map_err(|e| format!("internal error: generated invalid Wasm module: {e}"));
    if result.is_err()
        && let Ok(path) = std::env::var("ARUKELLT_DUMP_INVALID_WASM")
    {
        let _ = std::fs::write(&path, bytes);
    }
    result
}

/// Emit a Wasm module from MIR for the given target.
///
/// Builds a backend plan first, then routes emission through the plan consumer.
pub fn emit(
    mir: &MirModule,
    sink: &mut DiagnosticSink,
    target: TargetId,
    opt_level: u8,
) -> Vec<u8> {
    match build_backend_plan(target, target.profile().default_emit_kind) {
        Ok(plan) => emit_with_plan(mir, sink, &plan, opt_level, false, WasiVersion::P1),
        Err(message) => {
            sink.emit(wasm_validation_diagnostic(message));
            Vec::new()
        }
    }
}

pub fn emit_with_plan(
    mir: &MirModule,
    sink: &mut DiagnosticSink,
    plan: &BackendPlan,
    opt_level: u8,
    strip_debug: bool,
    wasi_version: WasiVersion,
) -> Vec<u8> {
    if wasi_version == WasiVersion::P2 && plan.runtime_model == RuntimeModel::T3WasmGcP2 {
        sink.emit(wasi_p2_native_emit_limitation_diagnostic());
    }
    let bytes = match plan.runtime_model {
        RuntimeModel::T1LinearP1 => t1_wasm32_p1::emit(mir, sink),
        RuntimeModel::T2Freestanding => t2_freestanding::emit(mir, sink),
        RuntimeModel::T3WasmGcP2 => t3_wasm_gc::emit(mir, sink, opt_level, strip_debug),
        RuntimeModel::T4LlvmScaffold => {
            sink.emit(wasm_validation_diagnostic(
                "native backend plan cannot be emitted via ark-wasm".to_string(),
            ));
            Vec::new()
        }
    };

    if plan.requires_backend_validation && std::env::var("ARK_SKIP_VALIDATE").is_err() {
        backend_validate(&bytes, sink);
    }

    // Debug dump for analysis (ARK_DUMP_WASM=/tmp/out.wasm)
    if let Ok(dump_path) = std::env::var("ARK_DUMP_WASM") {
        let _ = std::fs::write(&dump_path, &bytes);
    }

    bytes
}

pub fn backend_validate(bytes: &[u8], sink: &mut DiagnosticSink) {
    if sink.has_errors() || bytes.is_empty() {
        return;
    }

    if let Err(msg) = validate_wasm(bytes) {
        sink.emit(wasm_validation_diagnostic(msg));
    }
}

/// Validate that the requested emit kind is compatible with the target.
/// Returns an error message if incompatible.
pub fn validate_emit_kind(target: TargetId, emit_kind: EmitKind) -> Result<(), String> {
    let plan = build_backend_plan(target, emit_kind)?;
    match plan.capability {
        EmitCapability::CoreWasm | EmitCapability::Wit | EmitCapability::Component => Ok(()),
        EmitCapability::NativeBinary => Err(
            "native emission must go through the LLVM backend, not the Wasm backend".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_mir::{MirModule, runtime_entry_name};

    #[test]
    fn component_emit_is_accepted_for_t3() {
        assert!(validate_emit_kind(TargetId::Wasm32WasiP2, EmitKind::Component).is_ok());
    }

    #[test]
    fn component_emit_rejected_for_t1() {
        assert!(validate_emit_kind(TargetId::Wasm32WasiP1, EmitKind::Component).is_err());
    }

    #[test]
    fn backend_plan_exports_runtime_entry_for_t1_and_t3() {
        let t1 = build_backend_plan(TargetId::Wasm32WasiP1, EmitKind::CoreWasm).unwrap();
        let t3 = build_backend_plan(TargetId::Wasm32WasiP2, EmitKind::CoreWasm).unwrap();
        assert!(t1.exports.iter().any(|export| export.name == "_start"));
        assert!(t3.exports.iter().any(|export| export.name == "_start"));
    }

    #[test]
    fn t2_core_wasm_plan_is_scaffolded_and_compile_only() {
        let t2 = build_backend_plan(TargetId::Wasm32Freestanding, EmitKind::CoreWasm).unwrap();
        assert_eq!(t2.runtime_model, RuntimeModel::T2Freestanding);
        assert!(t2.exports.iter().any(|export| export.name == "_start"));
        assert!(t2.exports.iter().any(|export| export.name == "memory"));
        assert!(t2.profile.implemented);
        assert!(!t2.profile.run_supported);
    }

    #[test]
    fn runtime_entry_helper_matches_backend_plan_convention() {
        let mut module = MirModule::new();
        module.entry_fn = None;
        assert!(runtime_entry_name(&module).is_none());
    }
}
