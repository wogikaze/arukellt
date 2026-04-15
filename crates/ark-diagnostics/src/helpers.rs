//! Convenience diagnostic builders and phase-dump utilities.

use std::fmt;

use crate::codes::{DIAGNOSTIC_DUMP_ENV, DiagnosticCode, PHASE_DUMP_ENV};
use crate::render::render_structured_snapshots;
use crate::sink::{Diagnostic, DiagnosticPhase, SourceMap};

pub fn alias_warning_diagnostic(used_alias: &str, canonical_name: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::W0002)
        .with_phase(DiagnosticPhase::Target)
        .with_note(format!(
            "target alias `{}` is deprecated; use `{}` instead",
            used_alias, canonical_name
        ))
}

pub fn wasm_validation_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::W0004)
        .with_phase(DiagnosticPhase::BackendValidate)
        .with_note(message.into())
}

pub fn non_exportable_function_diagnostic(func_name: &str, reason: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::W0005)
        .with_phase(DiagnosticPhase::BackendValidate)
        .with_note(format!(
            "function `{}` {}, skipped from component exports",
            func_name, reason
        ))
}

pub fn component_unsupported_flags_diagnostic(func_name: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0400)
        .with_phase(DiagnosticPhase::BackendValidate)
        .with_note(format!(
            "function `{}` uses a type that maps to WIT flags, which is not supported in the current version",
            func_name
        ))
}

pub fn wit_flags_v2_diagnostic(path: &str, func_name: &str, type_desc: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0090)
        .with_phase(DiagnosticPhase::BackendValidate)
        .with_note(format!(
            "WIT file `{}` declares function `{}` with unsupported flags type `{}`",
            path, func_name, type_desc
        ))
        .with_help(
            "replace the flags type with individual bool parameters until WIT flags lowering lands",
        )
}

pub fn component_compound_type_diagnostic(func_name: &str, type_desc: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0401)
        .with_phase(DiagnosticPhase::BackendValidate)
        .with_note(format!(
            "function `{}` exports compound type `{}` which requires canonical ABI lift/lower not yet implemented",
            func_name, type_desc
        ))
}

pub fn component_resource_diagnostic(func_name: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0402)
        .with_phase(DiagnosticPhase::BackendValidate)
        .with_note(format!(
            "function `{}` uses a WIT resource type, which is not implemented in the current version",
            func_name
        ))
}

pub fn stable_debug_dump<T: fmt::Debug>(value: &T) -> String {
    format!("{:#?}", value)
}

pub fn dump_named_phase<T: fmt::Debug>(phase: &str, value: &T) -> String {
    format!("== {} ==\n{}", phase, stable_debug_dump(value))
}

pub fn requested_dump_phases(var_name: &str) -> Option<Vec<String>> {
    let raw = std::env::var(var_name).ok()?;
    let phases = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if phases.is_empty() {
        None
    } else {
        Some(phases)
    }
}

pub fn should_dump_phase(var_name: &str, phase: &str) -> bool {
    requested_dump_phases(var_name)
        .map(|phases| {
            phases
                .iter()
                .any(|candidate| candidate == phase || candidate == "all")
        })
        .unwrap_or(false)
}

pub fn emit_phase_dump_if_requested<T: fmt::Debug>(phase: &str, value: &T) {
    if should_dump_phase(PHASE_DUMP_ENV, phase) {
        eprintln!("{}", dump_named_phase(phase, value));
    }
}

pub fn emit_diagnostics_dump_if_requested(diagnostics: &[Diagnostic], source_map: &SourceMap) {
    if std::env::var_os(DIAGNOSTIC_DUMP_ENV).is_some() {
        eprintln!("{}", render_structured_snapshots(diagnostics, source_map));
    }
}

#[cfg(test)]
mod tests {
    use crate::codes::PHASE_DUMP_ENV;

    use super::*;

    #[test]
    fn test_requested_dump_phases() {
        unsafe {
            std::env::set_var(PHASE_DUMP_ENV, "parse,mir");
        }
        let phases = requested_dump_phases(PHASE_DUMP_ENV).unwrap();
        assert_eq!(phases, vec!["parse".to_string(), "mir".to_string()]);
        unsafe {
            std::env::remove_var(PHASE_DUMP_ENV);
        }
    }
}
