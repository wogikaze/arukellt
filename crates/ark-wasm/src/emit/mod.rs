//! Per-target Wasm emitter dispatch.
//!
//! Routes MIR emission through the appropriate backend based on `TargetId`.
//! T1: linear memory + WASI p1 (fully implemented).
//! T3: Wasm GC + WASI p2 (in progress — currently delegates to T1).

pub mod t1_wasm32_p1;
pub mod t3_wasm_gc;

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_mir::mir::MirModule;
use ark_target::{EmitKind, TargetId};

/// Validate generated Wasm module bytes using `wasmparser`.
///
/// Returns `Ok(())` if the module is valid, or an error string describing the
/// validation failure.
fn validate_wasm(bytes: &[u8]) -> Result<(), String> {
    let mut validator = wasmparser::Validator::new();
    validator
        .validate_all(bytes)
        .map(|_| ())
        .map_err(|e| format!("internal error: generated invalid Wasm module: {e}"))
}

/// Emit a Wasm module from MIR for the given target.
///
/// Currently only `Wasm32WasiP1` is implemented. Other targets will
/// return an error via the diagnostic sink once their emitters are added.
///
/// After target-specific emission, the output is validated with `wasmparser`.
/// Validation failures are currently reported as warnings (W0004) because
/// the emitters are still maturing.
// TODO: promote W0004 to an error once all emitters produce valid Wasm.
pub fn emit(mir: &MirModule, sink: &mut DiagnosticSink, target: TargetId) -> Vec<u8> {
    let bytes = match target {
        TargetId::Wasm32WasiP1 => t1_wasm32_p1::emit(mir, sink),
        TargetId::Wasm32WasiP2 => t3_wasm_gc::emit(mir, sink),
        other => {
            panic!(
                "emitter for target `{}` ({}) is not yet implemented",
                other,
                other.tier()
            );
        }
    };

    // Don't bother validating if emission already produced errors.
    if sink.has_errors() || bytes.is_empty() {
        return bytes;
    }

    if let Err(msg) = validate_wasm(&bytes) {
        eprintln!("warning: {msg}");
        sink.emit(
            Diagnostic::new(DiagnosticCode::W0004)
                .with_note(msg),
        );
    }

    bytes
}

/// Validate that the requested emit kind is compatible with the target.
/// Returns an error message if incompatible.
pub fn validate_emit_kind(target: TargetId, emit_kind: EmitKind) -> Result<(), String> {
    let profile = target.profile();

    // Component model output is not yet implemented for any target.
    if emit_kind == EmitKind::Component {
        return Err(
            "--emit component is not yet implemented. Only core Wasm modules are \
             currently supported. Use --emit core-wasm instead."
                .to_string(),
        );
    }
    if emit_kind == EmitKind::All {
        return Err(
            "--emit all is not yet supported because component model output is not \
             implemented. Use --emit core-wasm instead."
                .to_string(),
        );
    }

    match (target, emit_kind) {
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
