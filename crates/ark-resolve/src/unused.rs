//! Unused import detection.
//!
//! Walks the AST to find which import module names are actually referenced
//! via `QualifiedIdent` expressions or `TypeExpr::Qualified` types, then
//! reports unused imports as W0006 warnings.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_parser::ast;
use std::collections::HashSet;

/// Check for unused imports in the given module and emit W0006 warnings.
pub fn check_unused_imports(module: &ast::Module, sink: &mut DiagnosticSink) {
    if module.imports.is_empty() {
        return;
    }

    let mut used_modules = HashSet::new();
    for item in &module.items {
        collect_used_modules_in_item(item, &mut used_modules);
    }

    for import in &module.imports {
        let effective_name = import
            .alias
            .as_deref()
            .unwrap_or_else(|| {
                import
                    .module_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&import.module_name)
            });

        // Convention: `_` prefixed aliases suppress unused warnings
        if effective_name.starts_with('_') {
            continue;
        }

        if !used_modules.contains(effective_name) {
            sink.emit(
                Diagnostic::new(DiagnosticCode::W0006)
                    .with_message(format!("unused import `{}`", import.module_name))
                    .with_label(import.span, "this import is not used"),
            );
        }
    }
}

fn collect_used_modules_in_item(item: &ast::Item, used: &mut HashSet<String>) {
    match item {
        ast::Item::FnDef(f) => collect_used_modules_in_fn(f, used),
        ast::Item::StructDef(s) => {
            for field in &s.fields {
                collect_used_modules_in_type(&field.ty, used);
            }
        }
        ast::Item::EnumDef(e) => {
            for variant in &e.variants {
                match variant {
                    ast::Variant::Unit { .. } => {}
                    ast::Variant::Tuple { fields, .. } => {
                        for ty in fields {
                            collect_used_modules_in_type(ty, used);
                        }
                    }
                    ast::Variant::Struct { fields, .. } => {
                        for field in fields {
                            collect_used_modules_in_type(&field.ty, used);
                        }
                    }
                }
            }
        }
        ast::Item::TraitDef(t) => {
            for method in &t.methods {
                for p in &method.params {
                    collect_used_modules_in_type(&p.ty, used);
                }
                if let Some(ret) = &method.return_type {
                    collect_used_modules_in_type(ret, used);
                }
            }
        }
        ast::Item::ImplBlock(i) => {
            for method in &i.methods {
                collect_used_modules_in_fn(method, used);
            }
        }
    }
}

fn collect_used_modules_in_fn(f: &ast::FnDef, used: &mut HashSet<String>) {
    for param in &f.params {
        collect_used_modules_in_type(&param.ty, used);
    }
    if let Some(ret) = &f.return_type {
        collect_used_modules_in_type(ret, used);
    }
    collect_used_modules_in_block(&f.body, used);
}

fn collect_used_modules_in_block(block: &ast::Block, used: &mut HashSet<String>) {
    for stmt in &block.stmts {
        collect_used_modules_in_stmt(stmt, used);
    }
    if let Some(tail) = &block.tail_expr {
        collect_used_modules_in_expr(tail, used);
    }
}

fn collect_used_modules_in_type(ty: &ast::TypeExpr, used: &mut HashSet<String>) {
    match ty {
        ast::TypeExpr::Qualified { module, .. } => {
            used.insert(module.clone());
        }
        ast::TypeExpr::Generic { args, .. } => {
            for arg in args {
                collect_used_modules_in_type(arg, used);
            }
        }
        ast::TypeExpr::Array { elem, .. } | ast::TypeExpr::Slice { elem, .. } => {
            collect_used_modules_in_type(elem, used);
        }
        ast::TypeExpr::Function { params, ret, .. } => {
            for p in params {
                collect_used_modules_in_type(p, used);
            }
            collect_used_modules_in_type(ret, used);
        }
        ast::TypeExpr::Tuple(types, _) => {
            for t in types {
                collect_used_modules_in_type(t, used);
            }
        }
        ast::TypeExpr::Named { .. } | ast::TypeExpr::Unit(_) => {}
    }
}

fn collect_used_modules_in_stmt(stmt: &ast::Stmt, used: &mut HashSet<String>) {
    match stmt {
        ast::Stmt::Let { init, ty, .. } => {
            if let Some(t) = ty {
                collect_used_modules_in_type(t, used);
            }
            collect_used_modules_in_expr(init, used);
        }
        ast::Stmt::Expr(expr) => {
            collect_used_modules_in_expr(expr, used);
        }
        ast::Stmt::While { cond, body, .. } => {
            collect_used_modules_in_expr(cond, used);
            collect_used_modules_in_block(body, used);
        }
        ast::Stmt::Loop { body, .. } => {
            collect_used_modules_in_block(body, used);
        }
        ast::Stmt::For { iter, body, .. } => {
            match iter {
                ast::ForIter::Range { start, end } => {
                    collect_used_modules_in_expr(start, used);
                    collect_used_modules_in_expr(end, used);
                }
                ast::ForIter::Values(expr) | ast::ForIter::Iter(expr) => {
                    collect_used_modules_in_expr(expr, used);
                }
            }
            collect_used_modules_in_block(body, used);
        }
    }
}

fn collect_used_modules_in_expr(expr: &ast::Expr, used: &mut HashSet<String>) {
    match expr {
        ast::Expr::QualifiedIdent { module, .. } => {
            used.insert(module.clone());
        }
        ast::Expr::Call {
            callee, args, ..
        } => {
            collect_used_modules_in_expr(callee, used);
            for arg in args {
                collect_used_modules_in_expr(arg, used);
            }
        }
        ast::Expr::Binary { left, right, .. } => {
            collect_used_modules_in_expr(left, used);
            collect_used_modules_in_expr(right, used);
        }
        ast::Expr::Unary { operand, .. } => {
            collect_used_modules_in_expr(operand, used);
        }
        ast::Expr::FieldAccess { object, .. } => {
            collect_used_modules_in_expr(object, used);
        }
        ast::Expr::Index { object, index, .. } => {
            collect_used_modules_in_expr(object, used);
            collect_used_modules_in_expr(index, used);
        }
        ast::Expr::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            collect_used_modules_in_expr(cond, used);
            collect_used_modules_in_block(then_block, used);
            if let Some(else_blk) = else_block {
                collect_used_modules_in_block(else_blk, used);
            }
        }
        ast::Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_used_modules_in_expr(scrutinee, used);
            for arm in arms {
                collect_used_modules_in_pattern(&arm.pattern, used);
                if let Some(guard) = &arm.guard {
                    collect_used_modules_in_expr(guard, used);
                }
                collect_used_modules_in_expr(&arm.body, used);
            }
        }
        ast::Expr::Block(block) => {
            collect_used_modules_in_block(block, used);
        }
        ast::Expr::Tuple { elements, .. } | ast::Expr::Array { elements, .. } => {
            for e in elements {
                collect_used_modules_in_expr(e, used);
            }
        }
        ast::Expr::ArrayRepeat { value, count, .. } => {
            collect_used_modules_in_expr(value, used);
            collect_used_modules_in_expr(count, used);
        }
        ast::Expr::StructInit { fields, base, .. } => {
            for (_name, val) in fields {
                collect_used_modules_in_expr(val, used);
            }
            if let Some(b) = base {
                collect_used_modules_in_expr(b, used);
            }
        }
        ast::Expr::Closure { body, params, .. } => {
            for p in params {
                if let Some(ty) = &p.ty {
                    collect_used_modules_in_type(ty, used);
                }
            }
            collect_used_modules_in_expr(body, used);
        }
        ast::Expr::Return { value, .. } => {
            if let Some(v) = value {
                collect_used_modules_in_expr(v, used);
            }
        }
        ast::Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_used_modules_in_expr(v, used);
            }
        }
        ast::Expr::Try { expr, .. } => {
            collect_used_modules_in_expr(expr, used);
        }
        ast::Expr::Assign { target, value, .. } => {
            collect_used_modules_in_expr(target, used);
            collect_used_modules_in_expr(value, used);
        }
        ast::Expr::Loop { body, .. } => {
            collect_used_modules_in_block(body, used);
        }
        ast::Expr::IntLit { .. }
        | ast::Expr::FloatLit { .. }
        | ast::Expr::StringLit { .. }
        | ast::Expr::CharLit { .. }
        | ast::Expr::BoolLit { .. }
        | ast::Expr::Ident { .. }
        | ast::Expr::Continue { .. } => {}
    }
}

fn collect_used_modules_in_pattern(pattern: &ast::Pattern, used: &mut HashSet<String>) {
    match pattern {
        ast::Pattern::Enum {
            path, fields, ..
        } => {
            // path like "module::Type" — check if first segment is a module
            if path.contains("::") {
                if let Some(module) = path.split("::").next() {
                    if module.chars().next().is_some_and(|c| c.is_lowercase()) {
                        used.insert(module.to_string());
                    }
                }
            }
            for f in fields {
                collect_used_modules_in_pattern(f, used);
            }
        }
        ast::Pattern::Tuple { elements, .. } => {
            for p in elements {
                collect_used_modules_in_pattern(p, used);
            }
        }
        ast::Pattern::Or { patterns, .. } => {
            for p in patterns {
                collect_used_modules_in_pattern(p, used);
            }
        }
        ast::Pattern::Struct { fields, .. } => {
            for (_name, pat) in fields {
                if let Some(p) = pat {
                    collect_used_modules_in_pattern(p, used);
                }
            }
        }
        ast::Pattern::Wildcard(_)
        | ast::Pattern::Ident { .. }
        | ast::Pattern::IntLit { .. }
        | ast::Pattern::FloatLit { .. }
        | ast::Pattern::StringLit { .. }
        | ast::Pattern::CharLit { .. }
        | ast::Pattern::BoolLit { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_check(source: &str) -> Vec<String> {
        let (tokens, _) = ark_lexer::Lexer::new(0, source).tokenize();
        let mut sink = DiagnosticSink::new();
        let module = ark_parser::parse(&tokens, &mut sink);
        let mut warn_sink = DiagnosticSink::new();
        check_unused_imports(&module, &mut warn_sink);
        warn_sink
            .diagnostics()
            .iter()
            .map(|d| d.message.clone())
            .collect()
    }

    #[test]
    fn no_imports_no_warnings() {
        let warnings = parse_and_check("fn main() { println(42) }");
        assert!(warnings.is_empty());
    }

    #[test]
    fn used_import_no_warning() {
        let warnings = parse_and_check(
            "use std::math\nfn main() {\n    let x = math::sqrt(4.0)\n    println(x)\n}",
        );
        assert!(warnings.is_empty(), "got: {:?}", warnings);
    }

    #[test]
    fn unused_import_warns() {
        let warnings = parse_and_check(
            "use std::math\nfn main() {\n    println(42)\n}",
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unused import"));
        assert!(warnings[0].contains("std::math"));
    }

    #[test]
    fn multiple_imports_partial_use() {
        let warnings = parse_and_check(
            "use std::math\nuse std::string\nfn main() {\n    let x = math::sqrt(4.0)\n    println(x)\n}",
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("std::string"));
    }

    #[test]
    fn aliased_import_used() {
        let warnings = parse_and_check(
            "use std::math as m\nfn main() {\n    let x = m::sqrt(4.0)\n    println(x)\n}",
        );
        assert!(warnings.is_empty(), "got: {:?}", warnings);
    }

    #[test]
    fn aliased_import_unused() {
        let warnings = parse_and_check(
            "use std::math as m\nfn main() {\n    println(42)\n}",
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("std::math"));
    }

    #[test]
    fn underscore_prefix_suppresses() {
        let warnings = parse_and_check(
            "use std::math as _m\nfn main() {\n    println(42)\n}",
        );
        assert!(warnings.is_empty(), "_ prefix should suppress: {:?}", warnings);
    }
}
