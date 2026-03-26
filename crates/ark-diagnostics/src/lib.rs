//! Diagnostic system for the Arukellt compiler.
//!
//! All diagnostic codes (E00xx–E03xx, W0xxx) and structured error reporting.

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

/// Diagnostic error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    // E00xx: Syntax errors
    E0001, // unexpected token
    E0002, // missing token
    E0003, // invalid construct

    // E01xx: Name resolution
    E0100, // unresolved name
    E0101, // duplicate definition
    E0102, // private access
    E0103, // circular import

    // E02xx: Type errors
    E0200, // type mismatch
    E0201, // missing annotation
    E0202, // wrong argument count
    E0203, // invalid generic
    E0204, // non-exhaustive match
    E0205, // mismatched arms
    E0206, // invalid pattern
    E0207, // immutable mutation
    E0208, // missing return
    E0209, // unreachable pattern
    E0210, // ? operator type mismatch

    // E03xx: v0 constraints
    E0300, // trait not available
    E0301, // method syntax forbidden
    E0302, // nested generic forbidden
    E0303, // for loop forbidden
    E0304, // operator overload forbidden

    // W0xxx: Warnings
    W0001, // possible unintended sharing
}

impl DiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::E0001 => "E0001",
            Self::E0002 => "E0002",
            Self::E0003 => "E0003",
            Self::E0100 => "E0100",
            Self::E0101 => "E0101",
            Self::E0102 => "E0102",
            Self::E0103 => "E0103",
            Self::E0200 => "E0200",
            Self::E0201 => "E0201",
            Self::E0202 => "E0202",
            Self::E0203 => "E0203",
            Self::E0204 => "E0204",
            Self::E0205 => "E0205",
            Self::E0206 => "E0206",
            Self::E0207 => "E0207",
            Self::E0208 => "E0208",
            Self::E0209 => "E0209",
            Self::E0210 => "E0210",
            Self::E0300 => "E0300",
            Self::E0301 => "E0301",
            Self::E0302 => "E0302",
            Self::E0303 => "E0303",
            Self::E0304 => "E0304",
            Self::W0001 => "W0001",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::E0001 => "unexpected token",
            Self::E0002 => "missing token",
            Self::E0003 => "invalid construct",
            Self::E0100 => "unresolved name",
            Self::E0101 => "duplicate definition",
            Self::E0102 => "access to private symbol",
            Self::E0103 => "circular import",
            Self::E0200 => "type mismatch",
            Self::E0201 => "missing type annotation",
            Self::E0202 => "wrong number of arguments",
            Self::E0203 => "invalid generic usage",
            Self::E0204 => "non-exhaustive match",
            Self::E0205 => "mismatched match arm types",
            Self::E0206 => "invalid pattern",
            Self::E0207 => "cannot mutate immutable variable",
            Self::E0208 => "missing return value",
            Self::E0209 => "unreachable pattern",
            Self::E0210 => "incompatible error type for `?` operator",
            Self::E0300 => "traits are not available in v0",
            Self::E0301 => "method call syntax is not available in v0",
            Self::E0302 => "nested generics are not allowed in v0",
            Self::E0303 => "`for` loop is not available in v0",
            Self::E0304 => "operator overloading is not available in v0",
            Self::W0001 => "possible unintended sharing of reference type",
        }
    }

    pub fn severity(self) -> Severity {
        match self {
            Self::W0001 => Severity::Warning,
            _ => Severity::Error,
        }
    }
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
    pub labels: Vec<Label>,
    pub fix_its: Vec<FixIt>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn new(code: DiagnosticCode) -> Self {
        Self {
            message: code.message().to_string(),
            code,
            labels: Vec::new(),
            fix_its: Vec::new(),
            notes: Vec::new(),
        }
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

    pub fn severity(&self) -> Severity {
        self.code.severity()
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

/// Render diagnostics to a string (simple text renderer).
pub fn render_diagnostics(diagnostics: &[Diagnostic], source_map: &SourceMap) -> String {
    let mut out = String::new();
    for diag in diagnostics {
        let severity = match diag.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Help => "help",
        };
        out.push_str(&format!("{}[{}]: {}\n", severity, diag.code, diag.message));

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
        out.push('\n');
    }
    out
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
        assert!(rendered.contains("error[E0200]"));
        assert!(rendered.contains("test.ark:1:14"));
    }
}
