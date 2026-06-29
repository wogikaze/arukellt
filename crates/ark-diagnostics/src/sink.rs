//! Core diagnostic types: spans, severity, phases, diagnostics, and the sink.

use crate::codes::DiagnosticCode;

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
    /// How-to-fix suggestions (rendered as `help:` lines, distinct from `note:`).
    pub helps: Vec<String>,
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
            helps: Vec::new(),
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

    /// Add a how-to-fix help message (rendered as `help:`, distinct from `note:`).
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    /// Compat shim: equivalent to `with_help()`. Kept for backward compatibility.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        let s: String = suggestion.into();
        // Keep suggestion field populated for any code that reads it directly
        // (e.g., ark-playground-wasm serialisation).
        self.suggestion = Some(s.clone());
        self.helps.push(s);
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
}
