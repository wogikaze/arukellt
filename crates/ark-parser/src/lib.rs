//! Parser for the Arukellt language.

pub mod ast;
pub mod parser;

use ark_diagnostics::DiagnosticSink;
use ark_lexer::Token;

/// Parse a stream of tokens into a Module AST.
pub fn parse(tokens: &[Token], sink: &mut DiagnosticSink) -> ast::Module {
    let mut p = parser::Parser::new(tokens, sink);
    p.parse_module()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_lexer::tokenize;

    fn parse_src(src: &str) -> (ast::Module, DiagnosticSink) {
        let (tokens, _) = tokenize(0, src);
        let mut sink = DiagnosticSink::new();
        let module = parse(&tokens, &mut sink);
        (module, sink)
    }

    #[test]
    fn test_empty_main() {
        let (module, sink) = parse_src("fn main() {}");
        assert!(!sink.has_errors());
        assert_eq!(module.items.len(), 1);
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.name, "main");
            assert!(f.params.is_empty());
            assert!(f.return_type.is_none());
        } else {
            panic!("expected FnDef");
        }
    }

    #[test]
    fn test_fn_with_params() {
        let (module, sink) = parse_src("fn add(x: i32, y: i32) -> i32 { x + y }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.name, "add");
            assert_eq!(f.params.len(), 2);
            assert!(f.return_type.is_some());
        } else {
            panic!("expected FnDef");
        }
    }

    #[test]
    fn test_struct_def() {
        let (module, sink) = parse_src("struct Point { x: f64, y: f64 }");
        assert!(!sink.has_errors());
        if let ast::Item::StructDef(s) = &module.items[0] {
            assert_eq!(s.name, "Point");
            assert_eq!(s.fields.len(), 2);
        } else {
            panic!("expected StructDef");
        }
    }

    #[test]
    fn test_enum_def() {
        let (module, sink) = parse_src("enum Color { Red, Green, Blue, Rgb(i32, i32, i32) }");
        assert!(!sink.has_errors());
        if let ast::Item::EnumDef(e) = &module.items[0] {
            assert_eq!(e.name, "Color");
            assert_eq!(e.variants.len(), 4);
        } else {
            panic!("expected EnumDef");
        }
    }

    #[test]
    fn test_let_stmt() {
        let (module, sink) = parse_src("fn main() { let x: i32 = 42; let mut y = 0 }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.body.stmts.len(), 2);
        } else {
            panic!("expected FnDef");
        }
    }

    #[test]
    fn test_if_expr() {
        let (module, sink) = parse_src("fn main() { if true { 1 } else { 2 } }");
        assert!(!sink.has_errors());
    }

    #[test]
    fn test_match_expr() {
        let (module, sink) = parse_src(r#"
            fn main() {
                match x {
                    0 => "zero",
                    1 => "one",
                    _ => "other",
                }
            }
        "#);
        assert!(!sink.has_errors());
    }

    #[test]
    fn test_while_loop() {
        let (module, sink) = parse_src("fn main() { while i < 10 { i = i + 1 } }");
        assert!(!sink.has_errors());
    }

    #[test]
    fn test_generic_fn() {
        let (module, sink) = parse_src("fn identity<T>(x: T) -> T { x }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.type_params, vec!["T"]);
        } else {
            panic!("expected FnDef");
        }
    }

    #[test]
    fn test_closure() {
        let (module, sink) = parse_src("fn main() { let f = |x| x + 1 }");
        assert!(!sink.has_errors());
    }

    #[test]
    fn test_for_rejected() {
        let (_, sink) = parse_src("for x in items { }");
        assert!(sink.has_errors());
        assert!(sink.diagnostics().iter().any(|d| d.code == ark_diagnostics::DiagnosticCode::E0303));
    }

    #[test]
    fn test_trait_rejected() {
        let (_, sink) = parse_src("trait Foo { }");
        assert!(sink.has_errors());
        assert!(sink.diagnostics().iter().any(|d| d.code == ark_diagnostics::DiagnosticCode::E0300));
    }

    #[test]
    fn test_import() {
        let (module, sink) = parse_src("import io\nfn main() {}");
        assert!(!sink.has_errors());
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0].module_name, "io");
    }

    #[test]
    fn test_operator_precedence() {
        let (module, sink) = parse_src("fn main() { let x = 1 + 2 * 3 }");
        assert!(!sink.has_errors());
        // Should parse as 1 + (2 * 3), meaning top-level Binary is Add
        if let ast::Item::FnDef(f) = &module.items[0] {
            if let ast::Stmt::Let { init, .. } = &f.body.stmts[0] {
                if let ast::Expr::Binary { op, .. } = init {
                    assert_eq!(*op, ast::BinOp::Add);
                } else {
                    panic!("expected Binary expr");
                }
            }
        }
    }
}
