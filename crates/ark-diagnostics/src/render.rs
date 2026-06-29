//! Rendering and formatting diagnostics for display.

use crate::sink::{Diagnostic, Severity, SourceMap};

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
    for fix_it in &diagnostic.fix_its {
        hints.push(fix_it.message.clone());
    }
    // suggestion is now also in helps, but we skip it here to avoid duplication
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

        for note in &diag.notes {
            out.push_str(&format!("   = note: {}\n", note));
        }
        // Render helps (distinct from notes). `with_suggestion()` now feeds into
        // `helps` as well, so we render from `helps` only — the `suggestion` field
        // is kept solely for backward-compat direct field access (e.g. playground-wasm).
        for help in &diag.helps {
            out.push_str(&format!("   = help: {}\n", help));
        }

        for fix_it in &diag.fix_its {
            let file = source_map.get_file(fix_it.span.file_id);
            let (line, _col) = file.offset_to_line_col(fix_it.span.start);
            out.push_str(&format!("fix: {}\n", fix_it.message));
            out.push_str("   |\n");
            out.push_str(&format!("{:>3} | {}\n", line, fix_it.replacement));
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
    for help in &diagnostic.helps {
        out.push_str(&format!("help={}\n", help));
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

#[cfg(test)]
mod tests {
    use crate::codes::DiagnosticCode;
    use crate::sink::{Diagnostic, SourceMap, Span};

    use super::*;

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
    fn snapshot_e0100_unresolved_name() {
        let mut sm = SourceMap::new();
        let fid = sm.add_file("test.ark".into(), "fn main() { foo(42) }".into());
        let diag = Diagnostic::new(DiagnosticCode::E0100)
            .with_label(Span::new(fid, 12, 15), "unresolved name `foo`")
            .with_note("names must be declared before use with `let`, `fn`, `use`, or `import`")
            .with_help("did you mean `foo_bar`?");
        let rendered = render_structured_snapshot(&diag, &sm);
        assert!(
            rendered.contains("code=E0100"),
            "should contain code=E0100\n{}",
            rendered
        );
        assert!(
            rendered.contains("note="),
            "should contain note\n{}",
            rendered
        );
        assert!(
            rendered.contains("help="),
            "should contain help\n{}",
            rendered
        );
        assert!(
            rendered.contains("did you mean `foo_bar`?"),
            "should contain suggestion text\n{}",
            rendered
        );
        // Also verify full render produces help: prefix
        let full = render_diagnostics(&[diag], &sm);
        assert!(
            full.contains("= note:"),
            "full render should have note prefix\n{}",
            full
        );
        assert!(
            full.contains("= help:"),
            "full render should have help prefix\n{}",
            full
        );
    }

    #[test]
    fn snapshot_e0200_type_mismatch() {
        let mut sm = SourceMap::new();
        let fid = sm.add_file("test.ark".into(), "let x: i32 = \"hello\"\n".into());
        let diag = Diagnostic::new(DiagnosticCode::E0200)
            .with_message("expected `i32`, found `String`")
            .with_label(Span::new(fid, 13, 20), "expected `i32`, found `String`")
            .with_note(
                "the declared type `i32` does not match the initializer type `String`",
            )
            .with_help(
                "ensure the initializer expression matches the declared type, or use `as` for explicit numeric conversion",
            );
        let rendered = render_structured_snapshot(&diag, &sm);
        assert!(
            rendered.contains("code=E0200"),
            "should contain code=E0200\n{}",
            rendered
        );
        assert!(
            rendered.contains("expected=i32"),
            "should contain expected\n{}",
            rendered
        );
        assert!(
            rendered.contains("actual=String"),
            "should contain actual\n{}",
            rendered
        );
        assert!(
            rendered.contains("note="),
            "should contain note\n{}",
            rendered
        );
        assert!(
            rendered.contains("help="),
            "should contain help\n{}",
            rendered
        );
    }

    #[test]
    fn snapshot_e0300_missing_field() {
        let mut sm = SourceMap::new();
        let fid = sm.add_file("test.ark".into(), "let v = point.z\n".into());
        let diag = Diagnostic::new(DiagnosticCode::E0300)
            .with_message("`Point` has no field `z`")
            .with_label(Span::new(fid, 8, 15), "unknown field `z`")
            .with_note("available fields: `x`, `y`")
            .with_help("check the struct definition of `Point`");
        let rendered = render_structured_snapshot(&diag, &sm);
        assert!(
            rendered.contains("code=E0300"),
            "should contain code=E0300\n{}",
            rendered
        );
        assert!(
            rendered.contains("note="),
            "should contain note\n{}",
            rendered
        );
        assert!(
            rendered.contains("help="),
            "should contain help\n{}",
            rendered
        );
        assert!(
            rendered.contains("available fields"),
            "note should list available fields\n{}",
            rendered
        );
        // Verify full render
        let full = render_diagnostics(&[diag], &sm);
        assert!(
            full.contains("= note:"),
            "full render should have note prefix\n{}",
            full
        );
        assert!(
            full.contains("= help:"),
            "full render should have help prefix\n{}",
            full
        );
    }

    #[test]
    fn test_note_and_help_render_distinct() {
        let mut sm = SourceMap::new();
        sm.add_file("test.ark".into(), "let x = 1\n".into());
        let diag = Diagnostic::new(DiagnosticCode::E0200)
            .with_note("this is a note")
            .with_help("this is a help");
        let rendered = render_diagnostics(&[diag], &sm);
        assert!(
            rendered.contains("= note: this is a note"),
            "note prefix\n{}",
            rendered
        );
        assert!(
            rendered.contains("= help: this is a help"),
            "help prefix\n{}",
            rendered
        );
    }

    #[test]
    fn test_with_suggestion_compat() {
        // with_suggestion() should still produce help output and populate suggestion field.
        let diag = Diagnostic::new(DiagnosticCode::E0100).with_suggestion("did you mean `bar`?");
        assert_eq!(diag.suggestion.as_deref(), Some("did you mean `bar`?"));
        assert_eq!(diag.helps.len(), 1);
        assert_eq!(diag.helps[0], "did you mean `bar`?");
    }
}
