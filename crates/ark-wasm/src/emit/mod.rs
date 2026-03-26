//! Per-target Wasm emitter dispatch.
//!
//! Routes MIR emission through the appropriate backend based on `TargetId`.
//! T1: linear memory + WASI p1 (fully implemented).
//! T3: Wasm GC + WASI p2 (in progress — currently delegates to T1).

pub mod t1_wasm32_p1;
pub mod t3_wasm_gc;

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::MirModule;
use ark_target::{EmitKind, TargetId};

/// Emit a Wasm module from MIR for the given target.
///
/// Currently only `Wasm32WasiP1` is implemented. Other targets will
/// return an error via the diagnostic sink once their emitters are added.
pub fn emit(mir: &MirModule, sink: &mut DiagnosticSink, target: TargetId) -> Vec<u8> {
    match target {
        TargetId::Wasm32WasiP1 => t1_wasm32_p1::emit(mir, sink),
        TargetId::Wasm32WasiP2 => t3_wasm_gc::emit(mir, sink),
        other => {
            panic!(
                "emitter for target `{}` ({}) is not yet implemented",
                other,
                other.tier()
            );
        }
    }
}

/// Validate that the requested emit kind is compatible with the target.
/// Returns an error message if incompatible.
pub fn validate_emit_kind(target: TargetId, emit_kind: EmitKind) -> Result<(), String> {
    let profile = target.profile();
    match (target, emit_kind) {
        // T1 only supports core-wasm
        (TargetId::Wasm32WasiP1, EmitKind::Component) => Err(format!(
            "target `{}` ({}) does not support component model output. \
             Use `--target wasm32-wasi-p2` for component model support.",
            target,
            target.tier()
        )),
        (TargetId::Wasm32WasiP1, EmitKind::Wit) => Err(format!(
            "target `{}` ({}) does not support WIT generation. \
             Use `--target wasm32-wasi-p2` for WIT support.",
            target,
            target.tier()
        )),
        // Only implemented targets
        _ if !profile.implemented => Err(format!(
            "target `{}` ({}) is not yet implemented [{}]",
            target,
            target.tier(),
            profile.status_label()
        )),
        _ => Ok(()),
    }
}
