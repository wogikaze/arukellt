use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStage {
    Lexer,
    Parser,
    Typecheck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RelatedInformation {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub level: DiagnosticLevel,
    pub stage: DiagnosticStage,
    pub range: Span,
    pub expected: String,
    pub actual: String,
    pub cause: String,
    pub related: Vec<RelatedInformation>,
    pub suggested_fix: String,
    pub alternatives: Vec<String>,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompileResult<T> {
    pub module: Option<T>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> CompileResult<T> {
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
            .count()
    }

    pub fn to_json(&self) -> serde_json::Result<Value> {
        Ok(json!({
            "version": "v0.1",
            "error_count": self.error_count(),
            "warning_count": self
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.level == DiagnosticLevel::Warning)
                .count(),
            "diagnostics": self.diagnostics,
        }))
    }
}
