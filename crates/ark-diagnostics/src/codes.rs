//! Diagnostic codes, specs, and the canonical registry.

use std::fmt;

use crate::sink::{DiagnosticPhase, Severity};

/// Canonical registry entry for one diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticSpec {
    pub code: DiagnosticCode,
    pub id: &'static str,
    pub message: &'static str,
    pub severity: Severity,
    pub phase: DiagnosticPhase,
}

/// Diagnostic error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    // E00xx: Syntax errors
    E0001,
    E0002,
    E0003,

    // E01xx: Name resolution
    E0100,
    E0101,
    E0102,
    E0103,
    /// Module not found
    E0104,

    // E02xx: Type errors
    E0200,
    E0201,
    E0202,
    E0203,
    E0204,
    E0205,
    E0206,
    E0207,
    E0208,
    E0209,
    E0210,
    E0211,

    // E03xx: Historical v0 constraints + target constraints
    E0300,
    E0301,
    E0302,
    E0303,
    E0304,
    E0305,
    E0306,
    E0307,

    // E04xx: Component Model / canonical ABI errors
    /// WIT flags type not supported in current version
    E0400,
    /// Canonical ABI not implemented for compound type (string, list, tuple, record, etc.)
    E0401,
    /// WIT resource type not implemented in current version
    E0402,

    // W0xxx: Warnings / validation gate
    W0001,
    W0002,
    W0003,
    W0004,
    /// Non-exportable function skipped from component exports
    W0005,
    /// Unused import
    W0006,
    /// Unused binding
    W0007,
}

pub const DIAGNOSTIC_CODES: &[DiagnosticCode] = &[
    DiagnosticCode::E0001,
    DiagnosticCode::E0002,
    DiagnosticCode::E0003,
    DiagnosticCode::E0100,
    DiagnosticCode::E0101,
    DiagnosticCode::E0102,
    DiagnosticCode::E0103,
    DiagnosticCode::E0104,
    DiagnosticCode::E0200,
    DiagnosticCode::E0201,
    DiagnosticCode::E0202,
    DiagnosticCode::E0203,
    DiagnosticCode::E0204,
    DiagnosticCode::E0205,
    DiagnosticCode::E0206,
    DiagnosticCode::E0207,
    DiagnosticCode::E0208,
    DiagnosticCode::E0209,
    DiagnosticCode::E0210,
    DiagnosticCode::E0211,
    DiagnosticCode::E0300,
    DiagnosticCode::E0301,
    DiagnosticCode::E0302,
    DiagnosticCode::E0303,
    DiagnosticCode::E0304,
    DiagnosticCode::E0305,
    DiagnosticCode::E0306,
    DiagnosticCode::E0307,
    DiagnosticCode::E0400,
    DiagnosticCode::E0401,
    DiagnosticCode::E0402,
    DiagnosticCode::W0001,
    DiagnosticCode::W0002,
    DiagnosticCode::W0003,
    DiagnosticCode::W0004,
    DiagnosticCode::W0005,
    DiagnosticCode::W0006,
    DiagnosticCode::W0007,
];

pub const INTERNAL_DIAGNOSTIC_IDS: &[&str] = &["ICE-PIPELINE", "ICE-MIR", "ICE-BACKEND"];
pub const PHASE_DUMP_ENV: &str = "ARUKELLT_DUMP_PHASES";
pub const DIAGNOSTIC_DUMP_ENV: &str = "ARUKELLT_DUMP_DIAGNOSTICS";
pub const PHASE_DUMP_ORDER: &[&str] = &[
    "parse",
    "resolve",
    "corehir",
    "mir",
    "optimized-mir",
    "backend-plan",
];

impl DiagnosticCode {
    pub fn spec(self) -> DiagnosticSpec {
        match self {
            Self::E0001 => DiagnosticSpec {
                code: self,
                id: "E0001",
                message: "unexpected token",
                severity: Severity::Error,
                phase: DiagnosticPhase::Parse,
            },
            Self::E0002 => DiagnosticSpec {
                code: self,
                id: "E0002",
                message: "missing token",
                severity: Severity::Error,
                phase: DiagnosticPhase::Parse,
            },
            Self::E0003 => DiagnosticSpec {
                code: self,
                id: "E0003",
                message: "invalid construct",
                severity: Severity::Error,
                phase: DiagnosticPhase::Parse,
            },
            Self::E0100 => DiagnosticSpec {
                code: self,
                id: "E0100",
                message: "unresolved name",
                severity: Severity::Error,
                phase: DiagnosticPhase::Resolve,
            },
            Self::E0101 => DiagnosticSpec {
                code: self,
                id: "E0101",
                message: "duplicate definition",
                severity: Severity::Error,
                phase: DiagnosticPhase::Resolve,
            },
            Self::E0102 => DiagnosticSpec {
                code: self,
                id: "E0102",
                message: "access to private symbol",
                severity: Severity::Error,
                phase: DiagnosticPhase::Resolve,
            },
            Self::E0103 => DiagnosticSpec {
                code: self,
                id: "E0103",
                message: "circular import",
                severity: Severity::Error,
                phase: DiagnosticPhase::Resolve,
            },
            Self::E0104 => DiagnosticSpec {
                code: self,
                id: "E0104",
                message: "module not found",
                severity: Severity::Error,
                phase: DiagnosticPhase::Resolve,
            },
            Self::E0200 => DiagnosticSpec {
                code: self,
                id: "E0200",
                message: "type mismatch",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0201 => DiagnosticSpec {
                code: self,
                id: "E0201",
                message: "missing type annotation",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0202 => DiagnosticSpec {
                code: self,
                id: "E0202",
                message: "wrong number of arguments",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0203 => DiagnosticSpec {
                code: self,
                id: "E0203",
                message: "invalid generic usage",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0204 => DiagnosticSpec {
                code: self,
                id: "E0204",
                message: "non-exhaustive match",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0205 => DiagnosticSpec {
                code: self,
                id: "E0205",
                message: "mismatched match arm types",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0206 => DiagnosticSpec {
                code: self,
                id: "E0206",
                message: "invalid pattern",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0207 => DiagnosticSpec {
                code: self,
                id: "E0207",
                message: "cannot mutate immutable variable",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0208 => DiagnosticSpec {
                code: self,
                id: "E0208",
                message: "missing return value",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0209 => DiagnosticSpec {
                code: self,
                id: "E0209",
                message: "unreachable pattern",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0210 => DiagnosticSpec {
                code: self,
                id: "E0210",
                message: "incompatible error type for `?` operator",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0211 => DiagnosticSpec {
                code: self,
                id: "E0211",
                message: "module contains only unimplemented host stubs",
                severity: Severity::Error,
                phase: DiagnosticPhase::Resolve,
            },
            Self::E0300 => DiagnosticSpec {
                code: self,
                id: "E0300",
                message: "traits are not available in v0",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0301 => DiagnosticSpec {
                code: self,
                id: "E0301",
                message: "method call syntax is not available in v0",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0302 => DiagnosticSpec {
                code: self,
                id: "E0302",
                message: "nested generics are not allowed in v0",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0303 => DiagnosticSpec {
                code: self,
                id: "E0303",
                message: "`for` loop is not available in v0",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0304 => DiagnosticSpec {
                code: self,
                id: "E0304",
                message: "operator overloading is not available in v0",
                severity: Severity::Error,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::E0305 => DiagnosticSpec {
                code: self,
                id: "E0305",
                message: "unsupported target",
                severity: Severity::Error,
                phase: DiagnosticPhase::Target,
            },
            Self::E0306 => DiagnosticSpec {
                code: self,
                id: "E0306",
                message: "invalid emit kind for target",
                severity: Severity::Error,
                phase: DiagnosticPhase::Target,
            },
            Self::E0307 => DiagnosticSpec {
                code: self,
                id: "E0307",
                message: "feature not available for target",
                severity: Severity::Error,
                phase: DiagnosticPhase::Target,
            },
            Self::E0400 => DiagnosticSpec {
                code: self,
                id: "E0400",
                message: "WIT flags type not supported in current version",
                severity: Severity::Error,
                phase: DiagnosticPhase::BackendValidate,
            },
            Self::E0401 => DiagnosticSpec {
                code: self,
                id: "E0401",
                message: "canonical ABI not implemented for compound type in component export",
                severity: Severity::Error,
                phase: DiagnosticPhase::BackendValidate,
            },
            Self::E0402 => DiagnosticSpec {
                code: self,
                id: "E0402",
                message: "WIT resource type not implemented in current version",
                severity: Severity::Error,
                phase: DiagnosticPhase::BackendValidate,
            },
            Self::W0001 => DiagnosticSpec {
                code: self,
                id: "W0001",
                message: "possible unintended sharing of reference type",
                severity: Severity::Warning,
                phase: DiagnosticPhase::TypeCheck,
            },
            Self::W0002 => DiagnosticSpec {
                code: self,
                id: "W0002",
                message: "deprecated target alias",
                severity: Severity::Warning,
                phase: DiagnosticPhase::Target,
            },
            Self::W0003 => DiagnosticSpec {
                code: self,
                id: "W0003",
                message: "ambiguous import: local and std modules share the same name",
                severity: Severity::Warning,
                phase: DiagnosticPhase::Resolve,
            },
            // Current-first refactor policy: W0004 is now a hard error at backend validation.
            Self::W0004 => DiagnosticSpec {
                code: self,
                id: "W0004",
                message: "generated Wasm module failed validation",
                severity: Severity::Error,
                phase: DiagnosticPhase::BackendValidate,
            },
            Self::W0005 => DiagnosticSpec {
                code: self,
                id: "W0005",
                message: "function has non-exportable parameter type, skipped from component exports",
                severity: Severity::Warning,
                phase: DiagnosticPhase::BackendValidate,
            },
            Self::W0006 => DiagnosticSpec {
                code: self,
                id: "W0006",
                message: "unused import",
                severity: Severity::Warning,
                phase: DiagnosticPhase::Resolve,
            },
            Self::W0007 => DiagnosticSpec {
                code: self,
                id: "W0007",
                message: "unused binding",
                severity: Severity::Warning,
                phase: DiagnosticPhase::TypeCheck,
            },
        }
    }

    pub fn as_str(self) -> &'static str {
        self.spec().id
    }

    pub fn message(self) -> &'static str {
        self.spec().message
    }

    pub fn severity(self) -> Severity {
        self.spec().severity
    }

    pub fn phase(self) -> DiagnosticPhase {
        self.spec().phase
    }
}

pub fn diagnostic_registry() -> Vec<DiagnosticSpec> {
    DIAGNOSTIC_CODES
        .iter()
        .copied()
        .map(DiagnosticCode::spec)
        .collect()
}

pub fn internal_diagnostic_ids() -> &'static [&'static str] {
    INTERNAL_DIAGNOSTIC_IDS
}

pub fn render_diagnostic_registry() -> String {
    let mut out = String::new();
    for spec in diagnostic_registry() {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            spec.id,
            match spec.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Help => "help",
            },
            spec.phase.as_str(),
            spec.message
        ));
    }
    for id in INTERNAL_DIAGNOSTIC_IDS {
        out.push_str(&format!(
            "{}\terror\tinternal\tinternal compiler error\n",
            id
        ));
    }
    out
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_marks_w0004_as_backend_error() {
        let spec = DiagnosticCode::W0004.spec();
        assert_eq!(spec.severity, Severity::Error);
        assert_eq!(spec.phase, DiagnosticPhase::BackendValidate);
    }

    #[test]
    fn test_registry_output_contains_warnings_and_ice_ids() {
        let rendered = render_diagnostic_registry();
        assert!(rendered.contains("W0001\twarning\ttypecheck"));
        assert!(rendered.contains("W0002\twarning\ttarget"));
        assert!(rendered.contains("W0004\terror\tbackend-validate"));
        assert!(rendered.contains("ICE-MIR\terror\tinternal"));
    }
}
