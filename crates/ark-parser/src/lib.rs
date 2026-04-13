//! Parser for the Arukellt language.

pub mod ast;
pub mod fmt;
mod parser;

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
        assert!(module.docs.is_empty());
        assert_eq!(module.items.len(), 1);
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.name, "main");
            assert!(f.docs.is_empty());
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
        let (_module, sink) = parse_src("fn main() { if true { 1 } else { 2 } }");
        assert!(!sink.has_errors());
    }

    #[test]
    fn test_match_expr() {
        let (_module, sink) = parse_src(
            r#"
            fn main() {
                match x {
                    0 => "zero",
                    1 => "one",
                    _ => "other",
                }
            }
        "#,
        );
        assert!(!sink.has_errors());
    }

    #[test]
    fn test_while_loop() {
        let (_module, sink) = parse_src("fn main() { while i < 10 { i = i + 1 } }");
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
        let (_module, sink) = parse_src("fn main() { let f = |x| x + 1 }");
        assert!(!sink.has_errors());
    }

    #[test]
    fn test_trait_parsing() {
        let (module, sink) = parse_src("trait Foo { fn bar(self) -> i32 }");
        assert!(!sink.has_errors());
        assert!(
            module
                .items
                .iter()
                .any(|i| matches!(i, ast::Item::TraitDef(_)))
        );
    }

    #[test]
    fn test_import() {
        let (module, sink) = parse_src("import io\nfn main() {}");
        assert!(!sink.has_errors());
        assert!(module.docs.is_empty());
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0].module_name, "io");
    }

    #[test]
    fn test_module_and_item_doc_comments() {
        let (module, sink) = parse_src(
            "//! std::math helpers\n/// Adds two integers.\nfn add(a: i32, b: i32) -> i32 { a + b }",
        );
        assert!(!sink.has_errors());
        assert_eq!(module.docs, vec!["std::math helpers"]);
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.docs, vec!["Adds two integers."]);
        } else {
            panic!("expected FnDef");
        }
    }

    #[test]
    fn test_operator_precedence() {
        let (module, sink) = parse_src("fn main() { let x = 1 + 2 * 3 }");
        assert!(!sink.has_errors());
        // Should parse as 1 + (2 * 3), meaning top-level Binary is Add
        if let ast::Item::FnDef(f) = &module.items[0] {
            if let ast::Stmt::Let { init, .. } = &f.body.stmts[0]
                && let ast::Expr::Binary { op, .. } = init
            {
                assert_eq!(*op, ast::BinOp::Add);
            } else {
                panic!("expected Binary expr");
            }
        }
    }

    #[test]
    fn test_loop_expr() {
        let (module, sink) = parse_src("fn main() { loop { break 42 } }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.body.stmts.len(), 1);
            assert!(matches!(f.body.stmts[0], ast::Stmt::Loop { .. }));
        } else {
            panic!("expected FnDef");
        }
    }

    #[test]
    fn test_error_recovery() {
        // Invalid syntax inside a function should not prevent parsing the next item
        let (module, sink) = parse_src("fn a() { @ } fn b() { 1 }");
        assert!(sink.has_errors());
        // Should still parse at least one item despite the error
        assert!(!module.items.is_empty());
    }

    #[test]
    fn test_pattern_matching_variants() {
        let (module, sink) = parse_src(
            r#"
            fn test() {
                match x {
                    true => 1,
                    false => 2,
                    (a, b) => 3,
                    -1 => 4,
                    _ => 5,
                }
            }
        "#,
        );
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            if let Some(tail) = &f.body.tail_expr {
                if let ast::Expr::Match { arms, .. } = tail.as_ref() {
                    assert_eq!(arms.len(), 5);
                    assert!(matches!(
                        arms[0].pattern,
                        ast::Pattern::BoolLit { value: true, .. }
                    ));
                    assert!(matches!(arms[2].pattern, ast::Pattern::Tuple { .. }));
                    assert!(matches!(
                        arms[3].pattern,
                        ast::Pattern::IntLit { value: -1, .. }
                    ));
                    assert!(matches!(arms[4].pattern, ast::Pattern::Wildcard(_)));
                } else {
                    panic!("expected Match");
                }
            } else {
                panic!("expected tail expression");
            }
        }
    }

    #[test]
    fn test_array_and_tuple() {
        let (module, sink) = parse_src("fn main() { let a = [1, 2, 3]; let t = (1, 2) }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert_eq!(f.body.stmts.len(), 2);
        }
    }

    #[test]
    fn test_struct_init() {
        let (module, sink) = parse_src("fn main() { let p = Point { x: 1, y: 2 } }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            if let ast::Stmt::Let { init, .. } = &f.body.stmts[0]
                && let ast::Expr::StructInit { name, fields, .. } = init
            {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
            } else {
                panic!("expected StructInit");
            }
        }
    }

    #[test]
    fn test_import_alias() {
        let (module, sink) = parse_src("import foo as bar\nfn main() {}");
        assert!(!sink.has_errors());
        assert_eq!(module.imports[0].module_name, "foo");
        assert_eq!(module.imports[0].alias, Some("bar".to_string()));
        assert!(matches!(module.imports[0].kind, ast::ImportKind::Simple));
    }

    #[test]
    fn test_use_module_path_import() {
        // `use std::text::string` — module-path import
        let (module, sink) = parse_src("use std::text::string\nfn main() {}");
        assert!(
            !sink.has_errors(),
            "unexpected errors: {:?}",
            sink.diagnostics()
        );
        assert_eq!(module.imports.len(), 1);
        let imp = &module.imports[0];
        assert_eq!(imp.module_name, "std::text::string");
        assert!(imp.alias.is_none());
        assert!(matches!(imp.kind, ast::ImportKind::ModulePath));
    }

    #[test]
    fn test_pub_use_module_path_import() {
        let (module, sink) = parse_src("pub use std::text::string\nfn main() {}");
        assert!(
            !sink.has_errors(),
            "unexpected errors: {:?}",
            sink.diagnostics()
        );
        assert_eq!(module.imports.len(), 1);
        let imp = &module.imports[0];
        assert_eq!(imp.module_name, "std::text::string");
        assert!(imp.alias.is_none());
        assert!(matches!(imp.kind, ast::ImportKind::PublicModulePath));
    }

    #[test]
    fn test_use_destructure_import() {
        // `use std::collections::{vec, hash_map}` — destructuring import
        let (module, sink) = parse_src("use std::collections::{vec, hash_map}\nfn main() {}");
        assert!(
            !sink.has_errors(),
            "unexpected errors: {:?}",
            sink.diagnostics()
        );
        assert_eq!(module.imports.len(), 1);
        let imp = &module.imports[0];
        assert_eq!(imp.module_name, "std::collections");
        assert!(imp.alias.is_none());
        if let ast::ImportKind::DestructureImport { names } = &imp.kind {
            assert_eq!(names, &["vec", "hash_map"]);
        } else {
            panic!("expected DestructureImport, got {:?}", imp.kind);
        }
    }

    #[test]
    fn test_pub_fn() {
        let (module, sink) = parse_src("pub fn hello() {}");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            assert!(f.is_pub);
        } else {
            panic!("expected FnDef");
        }
    }

    #[test]
    fn test_impl_parsing() {
        let (module, sink) =
            parse_src("struct Foo { x: i32 }\nimpl Foo { fn get_x(self) -> i32 { self.x } }");
        assert!(!sink.has_errors());
        assert!(
            module
                .items
                .iter()
                .any(|i| matches!(i, ast::Item::ImplBlock(_)))
        );
    }

    #[test]
    fn test_impl_method_doc_comments() {
        let (module, sink) = parse_src(
            "impl Counter {\n    /// Advances the counter.\n    fn next(self) -> i32 { 1 }\n}",
        );
        assert!(!sink.has_errors());
        if let ast::Item::ImplBlock(block) = &module.items[0] {
            assert!(block.docs.is_empty());
            assert_eq!(block.methods[0].docs, vec!["Advances the counter."]);
        } else {
            panic!("expected ImplBlock");
        }
    }

    #[test]
    fn test_closure_typed_params() {
        let (module, sink) = parse_src("fn main() { let f = |x: i32, y: i32| x + y }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            if let ast::Stmt::Let { init, .. } = &f.body.stmts[0]
                && let ast::Expr::Closure { params, .. } = init
            {
                assert_eq!(params.len(), 2);
                assert!(params[0].ty.is_some());
            } else {
                panic!("expected Closure");
            }
        }
    }

    #[test]
    fn test_enum_with_generic() {
        let (module, sink) = parse_src("enum Option<T> { None, Some(T) }");
        assert!(!sink.has_errors());
        if let ast::Item::EnumDef(e) = &module.items[0] {
            assert_eq!(e.type_params, vec!["T"]);
            assert_eq!(e.variants.len(), 2);
        } else {
            panic!("expected EnumDef");
        }
    }

    #[test]
    fn test_nested_if_else() {
        let (module, sink) = parse_src("fn f() { if a { 1 } else if b { 2 } else { 3 } }");
        assert!(!sink.has_errors());
        if let ast::Item::FnDef(f) = &module.items[0] {
            if let Some(tail) = &f.body.tail_expr
                && let ast::Expr::If { else_block, .. } = tail.as_ref()
            {
                assert!(else_block.is_some());
            } else {
                panic!("expected If");
            }
        }
    }
}
