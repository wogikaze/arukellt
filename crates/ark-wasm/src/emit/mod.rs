//! Per-target Wasm emitter dispatch.
//!
//! Routes MIR emission through the appropriate backend based on `TargetId`.
//! Currently only T1 (`wasm32-wasi-p1`) is implemented.

pub mod t1_wasm32_p1;

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::MirModule;
use ark_target::TargetId;

/// Emit a Wasm module from MIR for the given target.
///
/// Currently only `Wasm32WasiP1` is implemented. Other targets will
/// return an error via the diagnostic sink once their emitters are added.
pub fn emit(mir: &MirModule, sink: &mut DiagnosticSink, target: TargetId) -> Vec<u8> {
    match target {
        TargetId::Wasm32WasiP1 => t1_wasm32_p1::emit(mir, sink),
        other => {
            // Future backends: T3 Wasm GC, T2 freestanding, T4 native
            panic!(
                "emitter for target `{}` ({}) is not yet implemented",
                other,
                other.tier()
            );
        }
    }
}
