//! Diagnostic system for the Arukellt compiler.
//!
//! Canonical diagnostic codes (E00xx–E03xx, W0xxx), simple text rendering,
//! and structured snapshots for tests/docs.

use std::fmt;

/// Byte offset range in source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file_id: u32,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file_id: u32, start: u32, end: u32) -> Self {
        Self {
            file_id,
            start,
            end,
        }
    }

    pub fn dummy() -> Self {
        Self {
            file_id: 0,
            start: 0,
            end: 0,
        }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            file_id: self.file_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Help,
}

/// Phase that originated a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticPhase {
    Parse,
    Resolve,
    TypeCheck,
    Target,
    BackendValidate,
    Internal,
}

impl DiagnosticPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Resolve => "resolve",
            Self::TypeCheck => "typecheck",
            Self::Target => "target",
            Self::BackendValidate => "backend-validate",
            Self::Internal => "internal",
        }
    }
}

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

/// A label attached to a span in the source.
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

/// A suggested fix.
#[derive(Debug, Clone)]
pub struct FixIt {
    pub span: Span,
    pub replacement: String,
    pub message: String,
}

/// A single diagnostic (error, warning, or help).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub phase_override: Option<DiagnosticPhase>,
    pub labels: Vec<Label>,
    pub fix_its: Vec<FixIt>,
    pub notes: Vec<String>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn new(code: DiagnosticCode) -> Self {
        Self {
            code,
            message: code.message().to_string(),
            phase_override: None,
            labels: Vec::new(),
            fix_its: Vec::new(),
            notes: Vec::new(),
            suggestion: None,
        }
    }

    pub fn with_phase(mut self, phase: DiagnosticPhase) -> Self {
        self.phase_override = Some(phase);
        self
    }

    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = msg.into();
        self
    }

    pub fn with_label(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: msg.into(),
        });
        self
    }

    pub fn with_fix_it(
        mut self,
        span: Span,
        replacement: impl Into<String>,
        msg: impl Into<String>,
    ) -> Self {
        self.fix_its.push(FixIt {
            span,
            replacement: replacement.into(),
            message: msg.into(),
        });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    pub fn phase(&self) -> DiagnosticPhase {
        self.phase_override.unwrap_or(self.code.phase())
    }

    pub fn is_error(&self) -> bool {
        self.severity() == Severity::Error
    }
}

/// Collects diagnostics during compilation.
#[derive(Debug, Default)]
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn emit(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity() == Severity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

/// Source file registry for span resolution.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

#[derive(Debug)]
pub struct SourceFile {
    pub name: String,
    pub source: String,
    line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(name: String, source: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| (i + 1) as u32))
            .collect();
        Self {
            name,
            source,
            line_starts,
        }
    }

    /// Convert byte offset to (line, column), both 1-based.
    pub fn offset_to_line_col(&self, offset: u32) -> (u32, u32) {
        let line = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let col = offset - self.line_starts[line];
        (line as u32 + 1, col + 1)
    }

    pub fn line_text(&self, line: u32) -> &str {
        let idx = (line - 1) as usize;
        if idx >= self.line_starts.len() {
            return "";
        }
        let start = self.line_starts[idx] as usize;
        let end = self
            .line_starts
            .get(idx + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.source.len());
        self.source[start..end]
            .trim_end_matches('\n')
            .trim_end_matches('\r')
    }
}

impl SourceMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_file(&mut self, name: String, source: String) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(SourceFile::new(name, source));
        id
    }

    pub fn get_file(&self, id: u32) -> &SourceFile {
        &self.files[id as usize]
    }
}

pub fn render_expected_actual(message: &str) -> Option<(String, String)> {
    let expected_marker = "expected `";
    let found_marker = "found `";
    let expected_start = message.find(expected_marker)? + expected_marker.len();
    let expected_end = message[expected_start..].find('`')? + expected_start;
    let found_start = message.find(found_marker)? + found_marker.len();
    let found_end = message[found_start..].find('`')? + found_start;
    Some((
        message[expected_start..expected_end].to_string(),
        message[found_start..found_end].to_string(),
    ))
}

pub fn render_expected_actual_for_diagnostic(diagnostic: &Diagnostic) -> Option<(String, String)> {
    render_expected_actual(&diagnostic.message).or_else(|| {
        diagnostic
            .labels
            .iter()
            .find_map(|label| render_expected_actual(&label.message))
    })
}

pub fn render_fix_hints(diagnostic: &Diagnostic) -> Vec<String> {
    let mut hints = Vec::new();
    if let Some(suggestion) = &diagnostic.suggestion {
        hints.push(suggestion.clone());
    }
    for fix_it in &diagnostic.fix_its {
        hints.push(fix_it.message.clone());
    }
    hints
}

/// Render diagnostics to a string.
pub fn render_diagnostics(diagnostics: &[Diagnostic], source_map: &SourceMap) -> String {
    let mut out = String::new();
    for diag in diagnostics {
        let severity = match diag.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Help => "help",
        };
        out.push_str(&format!(
            "{}[{}|{}]: {}\n",
            severity,
            diag.code,
            diag.phase().as_str(),
            diag.message
        ));

        if let Some((expected, actual)) = render_expected_actual_for_diagnostic(diag) {
            out.push_str(&format!("   = expected: {}\n", expected));
            out.push_str(&format!("   = actual: {}\n", actual));
        }

        for label in &diag.labels {
            let file = source_map.get_file(label.span.file_id);
            let (line, col) = file.offset_to_line_col(label.span.start);
            out.push_str(&format!("  --> {}:{}:{}\n", file.name, line, col));
            out.push_str("   |\n");
            let line_text = file.line_text(line);
            out.push_str(&format!("{:>3} | {}\n", line, line_text));
            let underline_start = (col - 1) as usize;
            let underline_len = (label.span.end - label.span.start).max(1) as usize;
            out.push_str(&format!(
                "   | {}{} {}\n",
                " ".repeat(underline_start),
                "^".repeat(underline_len),
                label.message
            ));
        }

        for fix_it in &diag.fix_its {
            let file = source_map.get_file(fix_it.span.file_id);
            let (line, _col) = file.offset_to_line_col(fix_it.span.start);
            out.push_str(&format!("help: {}\n", fix_it.message));
            out.push_str("   |\n");
            out.push_str(&format!("{:>3} | {}\n", line, fix_it.replacement));
        }

        for note in &diag.notes {
            out.push_str(&format!("   = note: {}\n", note));
        }
        if let Some(suggestion) = &diag.suggestion {
            out.push_str(&format!("   = help: {}\n", suggestion));
        }
        out.push('\n');
    }
    out
}

pub fn render_structured_snapshot(diagnostic: &Diagnostic, source_map: &SourceMap) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "code={} severity={} phase={} message={}\n",
        diagnostic.code.as_str(),
        match diagnostic.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Help => "help",
        },
        diagnostic.phase().as_str(),
        diagnostic.message
    ));
    if let Some((expected, actual)) = render_expected_actual_for_diagnostic(diagnostic) {
        out.push_str(&format!("expected={} actual={}\n", expected, actual));
    }
    for label in &diagnostic.labels {
        let file = source_map.get_file(label.span.file_id);
        let (line, col) = file.offset_to_line_col(label.span.start);
        out.push_str(&format!(
            "label={}::{}:{}:{} {}\n",
            file.name,
            line,
            col,
            label.span.end.saturating_sub(label.span.start).max(1),
            label.message
        ));
    }
    for hint in render_fix_hints(diagnostic) {
        out.push_str(&format!("fix_hint={}\n", hint));
    }
    for note in &diagnostic.notes {
        out.push_str(&format!("note={}\n", note));
    }
    out
}

pub fn render_structured_snapshots(diagnostics: &[Diagnostic], source_map: &SourceMap) -> String {
    diagnostics
        .iter()
        .map(|diag| render_structured_snapshot(diag, source_map))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_minimal_diag_manifest(diagnostics: &[Diagnostic], source_map: &SourceMap) -> String {
    diagnostics
        .iter()
        .map(|diag| {
            let primary = diag
                .labels
                .first()
                .map(|label| {
                    let file = source_map.get_file(label.span.file_id);
                    let (line, col) = file.offset_to_line_col(label.span.start);
                    format!("{}:{}:{}", file.name, line, col)
                })
                .unwrap_or_else(|| "<none>".to_string());
            format!(
                "{}\t{}\t{}\t{}\t{}",
                diag.code.as_str(),
                match diag.severity() {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Help => "help",
                },
                diag.phase().as_str(),
                primary,
                diag.labels
                    .first()
                    .map(|label| label.message.clone())
                    .unwrap_or_else(|| diag.message.clone())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn baseline_records_json(diagnostics: &[Diagnostic], source_map: &SourceMap) -> String {
    let rows = diagnostics
        .iter()
        .map(|diag| {
            let primary_span = diag
                .labels
                .first()
                .map(|label| {
                    let file = source_map.get_file(label.span.file_id);
                    let (line, col) = file.offset_to_line_col(label.span.start);
                    format!("{}:{}:{}", file.name, line, col)
                })
                .unwrap_or_default();
            let primary_message = diag
                .labels
                .first()
                .map(|label| label.message.clone())
                .unwrap_or_else(|| diag.message.clone())
                .replace('"', "\\\"");
            format!(
                "{{\"code\":\"{}\",\"severity\":\"{}\",\"phase\":\"{}\",\"primary_span\":\"{}\",\"primary_message\":\"{}\"}}",
                diag.code.as_str(),
                match diag.severity() {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Help => "help",
                },
                diag.phase().as_str(),
                primary_span.replace('"', "\\\""),
                primary_message
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}

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
    use super::*;

    #[test]
    fn test_span_merge() {
        let a = Span::new(0, 5, 10);
        let b = Span::new(0, 8, 15);
        let merged = a.merge(b);
        assert_eq!(merged.start, 5);
        assert_eq!(merged.end, 15);
    }

    #[test]
    fn test_source_file_line_col() {
        let src = SourceFile::new("test.ark".into(), "fn main() {\n    let x = 42\n}\n".into());
        assert_eq!(src.offset_to_line_col(0), (1, 1));
        assert_eq!(src.offset_to_line_col(12), (2, 1));
        assert_eq!(src.offset_to_line_col(16), (2, 5));
    }

    #[test]
    fn test_diagnostic_builder() {
        let diag = Diagnostic::new(DiagnosticCode::E0200)
            .with_label(Span::new(0, 10, 17), "expected `i32`, found `String`")
            .with_fix_it(Span::new(0, 4, 7), "String", "change the type annotation");
        assert!(diag.is_error());
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.fix_its.len(), 1);
    }

    #[test]
    fn test_diagnostic_sink() {
        let mut sink = DiagnosticSink::new();
        sink.emit(Diagnostic::new(DiagnosticCode::E0001));
        sink.emit(Diagnostic::new(DiagnosticCode::W0001));
        assert!(sink.has_errors());
        assert_eq!(sink.error_count(), 1);
        assert_eq!(sink.diagnostics().len(), 2);
    }

    #[test]
    fn test_render_diagnostic() {
        let mut sm = SourceMap::new();
        sm.add_file("test.ark".into(), "let x: i32 = \"hello\"\n".into());
        let diag = Diagnostic::new(DiagnosticCode::E0200)
            .with_label(Span::new(0, 13, 20), "expected `i32`, found `String`");
        let rendered = render_diagnostics(&[diag], &sm);
        assert!(rendered.contains("error[E0200|typecheck]"));
        assert!(rendered.contains("test.ark:1:14"));
        assert!(rendered.contains("expected: i32"));
        assert!(rendered.contains("actual: String"));
    }

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

    #[test]
    fn test_structured_snapshot() {
        let mut sm = SourceMap::new();
        sm.add_file("test.ark".into(), "let x: i32 = \"hello\"\n".into());
        let diag = Diagnostic::new(DiagnosticCode::E0200)
            .with_label(Span::new(0, 13, 20), "expected `i32`, found `String`")
            .with_fix_it(Span::new(0, 4, 7), "String", "change the type annotation");
        let rendered = render_structured_snapshot(&diag, &sm);
        assert!(rendered.contains("code=E0200"));
        assert!(rendered.contains("phase=typecheck"));
        assert!(rendered.contains("fix_hint=change the type annotation"));
    }

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
