//! Arukellt playground Wasm module.
//!
//! Provides JS-callable APIs for parsing and formatting Arukellt source code
//! in the browser. Compiled to `wasm32-unknown-unknown` via `wasm-bindgen`.
//!
//! # Exported functions
//!
//! - [`parse`]: Tokenize + parse source → JSON with AST summary and diagnostics.
//! - [`format`]: Format source → JSON with formatted text or error.
//! - [`tokenize`]: Tokenize source → JSON with token stream and diagnostics.
//! - [`typecheck`]: Parse + resolve + type-check source → JSON with diagnostic array.
//! - [`version`]: Return the crate version string.

use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Serialisable response types (returned as JSON strings to JS)
// ---------------------------------------------------------------------------

use serde::Serialize;

/// A diagnostic serialised for the browser.
#[derive(Serialize)]
struct JsDiagnostic {
    code: String,
    severity: String,
    phase: String,
    message: String,
    labels: Vec<JsLabel>,
    notes: Vec<String>,
    suggestion: Option<String>,
}

/// A label (span + message) attached to a diagnostic.
#[derive(Serialize)]
struct JsLabel {
    file_id: u32,
    start: u32,
    end: u32,
    message: String,
}

/// A single token serialised for the browser.
#[derive(Serialize)]
struct JsToken {
    kind: String,
    text: String,
    start: u32,
    end: u32,
}

// ---------------------------------------------------------------------------
// AST summary types
// ---------------------------------------------------------------------------

/// Lightweight summary of a parsed module — enough for the playground UI
/// without exposing the full internal AST.
#[derive(Serialize)]
struct JsModule {
    docs: Vec<String>,
    imports: Vec<JsImport>,
    items: Vec<JsItem>,
}

#[derive(Serialize)]
struct JsImport {
    module_name: String,
    alias: Option<String>,
}

#[derive(Serialize)]
struct JsItem {
    kind: String,
    name: String,
    is_pub: bool,
    docs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Response envelopes
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ParseResponse {
    ok: bool,
    module: Option<JsModule>,
    diagnostics: Vec<JsDiagnostic>,
    error_count: usize,
}

#[derive(Serialize)]
struct FormatResponse {
    ok: bool,
    formatted: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct TokenizeResponse {
    ok: bool,
    tokens: Vec<JsToken>,
    diagnostics: Vec<JsDiagnostic>,
}

#[derive(Serialize)]
struct TypecheckResponse {
    ok: bool,
    diagnostics: Vec<JsDiagnostic>,
    error_count: usize,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

fn convert_diagnostic(d: &ark_diagnostics::Diagnostic) -> JsDiagnostic {
    JsDiagnostic {
        code: d.code.as_str().to_string(),
        severity: match d.severity() {
            ark_diagnostics::Severity::Error => "error".into(),
            ark_diagnostics::Severity::Warning => "warning".into(),
            ark_diagnostics::Severity::Help => "help".into(),
        },
        phase: d.phase().as_str().to_string(),
        message: d.message.clone(),
        labels: d
            .labels
            .iter()
            .map(|l| JsLabel {
                file_id: l.span.file_id,
                start: l.span.start,
                end: l.span.end,
                message: l.message.clone(),
            })
            .collect(),
        notes: d.notes.clone(),
        suggestion: d.suggestion.clone(),
    }
}

fn convert_module(m: &ark_parser::ast::Module) -> JsModule {
    JsModule {
        docs: m.docs.clone(),
        imports: m
            .imports
            .iter()
            .map(|imp| JsImport {
                module_name: imp.module_name.clone(),
                alias: imp.alias.clone(),
            })
            .collect(),
        items: m.items.iter().map(convert_item).collect(),
    }
}

fn convert_item(item: &ark_parser::ast::Item) -> JsItem {
    match item {
        ark_parser::ast::Item::FnDef(f) => JsItem {
            kind: "fn".into(),
            name: f.name.clone(),
            is_pub: f.is_pub,
            docs: f.docs.clone(),
        },
        ark_parser::ast::Item::StructDef(s) => JsItem {
            kind: "struct".into(),
            name: s.name.clone(),
            is_pub: s.is_pub,
            docs: s.docs.clone(),
        },
        ark_parser::ast::Item::EnumDef(e) => JsItem {
            kind: "enum".into(),
            name: e.name.clone(),
            is_pub: e.is_pub,
            docs: e.docs.clone(),
        },
        ark_parser::ast::Item::TraitDef(t) => JsItem {
            kind: "trait".into(),
            name: t.name.clone(),
            is_pub: t.is_pub,
            docs: t.docs.clone(),
        },
        ark_parser::ast::Item::ImplBlock(i) => JsItem {
            kind: "impl".into(),
            name: i.target_type.clone(),
            is_pub: false,
            docs: i.docs.clone(),
        },
    }
}

// ---------------------------------------------------------------------------
// Exported wasm-bindgen functions
// ---------------------------------------------------------------------------

/// Parse Arukellt source code, returning a JSON object with the AST summary
/// and any diagnostics.
///
/// # Returns
///
/// JSON string with shape:
/// ```json
/// {
///   "ok": true,
///   "module": { "docs": [], "imports": [], "items": [] },
///   "diagnostics": [],
///   "error_count": 0
/// }
/// ```
#[wasm_bindgen]
pub fn parse(source: &str) -> String {
    let (tokens, lex_diagnostics) = ark_lexer::tokenize(0, source);
    let mut sink = ark_diagnostics::DiagnosticSink::new();
    let module = ark_parser::parse(&tokens, &mut sink);

    let mut all_diagnostics: Vec<JsDiagnostic> =
        lex_diagnostics.iter().map(convert_diagnostic).collect();
    all_diagnostics.extend(sink.diagnostics().iter().map(convert_diagnostic));

    let error_count = lex_diagnostics.iter().filter(|d| d.is_error()).count() + sink.error_count();

    let resp = ParseResponse {
        ok: error_count == 0,
        module: Some(convert_module(&module)),
        diagnostics: all_diagnostics,
        error_count,
    };
    serde_json::to_string(&resp)
        .unwrap_or_else(|e| format!(r#"{{"ok":false,"error":"serialization failed: {}"}}"#, e))
}

/// Format Arukellt source code.
///
/// Returns `None`-equivalent (`ok: false`) if the source has syntax errors.
///
/// # Returns
///
/// JSON string with shape:
/// ```json
/// { "ok": true, "formatted": "fn main() {\n}\n" }
/// ```
/// or on error:
/// ```json
/// { "ok": false, "error": "source contains syntax errors" }
/// ```
#[wasm_bindgen]
pub fn format(source: &str) -> String {
    match ark_parser::fmt::format_source(source) {
        Some(formatted) => {
            let resp = FormatResponse {
                ok: true,
                formatted: Some(formatted),
                error: None,
            };
            serde_json::to_string(&resp).unwrap_or_else(|e| {
                format!(r#"{{"ok":false,"error":"serialization failed: {}"}}"#, e)
            })
        }
        None => {
            let resp = FormatResponse {
                ok: false,
                formatted: None,
                error: Some("source contains syntax errors".into()),
            };
            serde_json::to_string(&resp).unwrap_or_else(|e| {
                format!(r#"{{"ok":false,"error":"serialization failed: {}"}}"#, e)
            })
        }
    }
}

/// Tokenize Arukellt source code, returning a JSON array of tokens and
/// any lexer diagnostics.
///
/// # Returns
///
/// JSON string with shape:
/// ```json
/// {
///   "ok": true,
///   "tokens": [{ "kind": "Fn", "text": "fn", "start": 0, "end": 2 }, ...],
///   "diagnostics": []
/// }
/// ```
#[wasm_bindgen]
pub fn tokenize(source: &str) -> String {
    let (tokens, lex_diagnostics) = ark_lexer::tokenize(0, source);

    let js_tokens: Vec<JsToken> = tokens
        .iter()
        .map(|t| JsToken {
            kind: format!("{:?}", t.kind),
            text: source
                .get(t.span.start as usize..t.span.end as usize)
                .unwrap_or("")
                .to_string(),
            start: t.span.start,
            end: t.span.end,
        })
        .collect();

    let diagnostics: Vec<JsDiagnostic> = lex_diagnostics.iter().map(convert_diagnostic).collect();

    let resp = TokenizeResponse {
        ok: lex_diagnostics.iter().all(|d| !d.is_error()),
        tokens: js_tokens,
        diagnostics,
    };
    serde_json::to_string(&resp)
        .unwrap_or_else(|e| format!(r#"{{"ok":false,"error":"serialization failed: {}"}}"#, e))
}

/// Type-check Arukellt source code, returning a JSON object with diagnostics.
///
/// Runs the full parse → resolve → type-check pipeline and returns all
/// diagnostics from all phases.
///
/// # Returns
///
/// JSON string with shape:
/// ```json
/// {
///   "ok": true,
///   "diagnostics": [],
///   "error_count": 0
/// }
/// ```
/// or on errors:
/// ```json
/// {
///   "ok": false,
///   "diagnostics": [{"code": "E0100", "severity": "error", "phase": "typecheck", "message": "...", "labels": [], "notes": [], "suggestion": null}],
///   "error_count": 1
/// }
/// ```
#[wasm_bindgen]
pub fn typecheck(source: &str) -> String {
    let (tokens, lex_diagnostics) = ark_lexer::tokenize(0, source);
    let mut sink = ark_diagnostics::DiagnosticSink::new();
    let module = ark_parser::parse(&tokens, &mut sink);

    let mut all_diagnostics: Vec<JsDiagnostic> =
        lex_diagnostics.iter().map(convert_diagnostic).collect();
    all_diagnostics.extend(sink.diagnostics().iter().map(convert_diagnostic));

    let lex_parse_error_count =
        lex_diagnostics.iter().filter(|d| d.is_error()).count() + sink.error_count();

    if lex_parse_error_count > 0 {
        let resp = TypecheckResponse {
            ok: false,
            diagnostics: all_diagnostics,
            error_count: lex_parse_error_count,
        };
        return serde_json::to_string(&resp).unwrap_or_else(|e| {
            format!(r#"{{"ok":false,"error":"serialization failed: {}"}}"#, e)
        });
    }

    let resolved = ark_resolve::resolve_module(module, &mut sink);
    all_diagnostics.extend(sink.diagnostics().iter().map(convert_diagnostic));
    let resolve_error_count = sink.error_count();

    if resolve_error_count > 0 {
        let resp = TypecheckResponse {
            ok: false,
            diagnostics: all_diagnostics,
            error_count: resolve_error_count,
        };
        return serde_json::to_string(&resp).unwrap_or_else(|e| {
            format!(r#"{{"ok":false,"error":"serialization failed: {}"}}"#, e)
        });
    }

    let mut tc_sink = ark_diagnostics::DiagnosticSink::new();
    let mut checker = ark_typecheck::TypeChecker::new();
    checker.register_builtins();
    checker.check_core_hir_module(&resolved, &mut tc_sink);

    let tc_diagnostics: Vec<JsDiagnostic> =
        tc_sink.diagnostics().iter().map(convert_diagnostic).collect();
    let tc_error_count = tc_sink.error_count();

    all_diagnostics.extend(tc_diagnostics);
    let total_errors = lex_parse_error_count + resolve_error_count + tc_error_count;

    let resp = TypecheckResponse {
        ok: total_errors == 0,
        diagnostics: all_diagnostics,
        error_count: total_errors,
    };
    serde_json::to_string(&resp)
        .unwrap_or_else(|e| format!(r#"{{"ok":false,"error":"serialization failed: {}"}}"#, e))
}

/// Return the crate version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ---------------------------------------------------------------------------
// Native-target tests (run with `cargo test -p ark-playground-wasm`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_source() {
        let json = parse("fn main() {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["error_count"], 0);
        assert_eq!(v["module"]["items"][0]["kind"], "fn");
        assert_eq!(v["module"]["items"][0]["name"], "main");
    }

    #[test]
    fn parse_invalid_source() {
        let json = parse("fn {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v["error_count"].as_u64().unwrap() > 0);
        assert!(!v["diagnostics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn format_valid_source() {
        let json = format("fn  main( ) {  }");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        assert!(v["formatted"].as_str().unwrap().contains("fn main()"));
    }

    #[test]
    fn format_invalid_source() {
        let json = format("fn {{}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v["error"].as_str().unwrap().contains("syntax errors"));
    }

    #[test]
    fn tokenize_valid_source() {
        let json = tokenize("fn main() {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        assert!(!v["tokens"].as_array().unwrap().is_empty());
    }

    #[test]
    fn version_returns_value() {
        let v = version();
        assert!(!v.is_empty());
    }

    #[test]
    fn parse_struct_and_enum() {
        let json = parse("struct Point { x: f64, y: f64 }\nenum Color { Red, Blue }");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        let items = v["module"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["kind"], "struct");
        assert_eq!(items[0]["name"], "Point");
        assert_eq!(items[1]["kind"], "enum");
        assert_eq!(items[1]["name"], "Color");
    }

    #[test]
    fn parse_with_imports() {
        let json = parse("import io\nimport math as m\nfn main() {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        let imports = v["module"]["imports"].as_array().unwrap();
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0]["module_name"], "io");
        assert_eq!(imports[1]["module_name"], "math");
        assert_eq!(imports[1]["alias"], "m");
    }

    #[test]
    fn parse_pub_fn() {
        let json = parse("pub fn hello() {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["module"]["items"][0]["is_pub"], true);
    }

    #[test]
    fn format_roundtrip() {
        let source = "fn main() {\n    let x = 42\n}\n";
        let json = format(source);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        // Formatted output should be parseable
        let formatted = v["formatted"].as_str().unwrap();
        let json2 = parse(formatted);
        let v2: serde_json::Value = serde_json::from_str(&json2).unwrap();
        assert_eq!(v2["ok"], true);
    }

    #[test]
    fn typecheck_valid_source() {
        let json = typecheck("fn main() {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true, "typecheck of valid source should succeed: {}", json);
        assert_eq!(v["error_count"], 0);
        assert!(v["diagnostics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn typecheck_returns_json_array_of_diagnostics() {
        let json = typecheck("fn main() {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["diagnostics"].is_array(), "diagnostics should be a JSON array");
    }

    #[test]
    fn typecheck_parse_error_propagates() {
        let json = typecheck("fn {}");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v["error_count"].as_u64().unwrap() > 0);
        assert!(!v["diagnostics"].as_array().unwrap().is_empty());
    }
}
