//! Type inference and conversion helpers for MIR lowering.

use ark_parser::ast;

/// Check if an expression is void (should be emitted as statement, not value).
pub(crate) fn is_void_expr(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Call { callee, .. } => {
            if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                matches!(
                    name.as_str(),
                    "println"
                        | "print"
                        | "eprintln"
                        | "panic"
                        | "assert"
                        | "assert_eq"
                        | "assert_ne"
                        | "assert_eq_str"
                        | "assert_eq_i64"
                        | "push"
                        | "push_char"
                        | "set"
                        | "sort_i32"
                        | "sort_String"
                        | "sort_i64"
                        | "sort_f64"
                        | "reverse_i32"
                        | "reverse_String"
                        | "remove_i32"
                )
            } else if let ast::Expr::QualifiedIdent { name, .. } = callee.as_ref() {
                // Module-qualified void calls
                matches!(
                    name.as_str(),
                    // std::test
                    "assert_true"
                        | "assert_false"
                        | "assert_eq_i32"
                        | "assert_eq_i64"
                        | "assert_eq_f64"
                        | "assert_eq_string"
                        | "assert_eq_bool"
                        | "assert_ne_i32"
                        | "assert_ne_string"
                        | "expect_none_i32"
                        // std::host::stdio
                        | "print"
                        | "println"
                        | "eprintln"
                        // std::host::process
                        | "exit"
                        | "abort"
                        // std::collections
                        | "hashmap_set"
                        | "deque_push_back"
                        | "deque_push_front"
                        | "sorted_map_insert"
                        | "bitset_set"
                        | "bitset_mark"
                        | "bitset_unmark"
                        | "bitset_clear"
                        | "pq_push"
                        | "bytes_push"
                )
            } else {
                false
            }
        }
        ast::Expr::Assign { .. } => true,
        ast::Expr::Block(block) => match &block.tail_expr {
            None => true,
            Some(tail) => is_void_expr(tail),
        },
        ast::Expr::If { then_block, .. } => match &then_block.tail_expr {
            None => true,
            Some(tail) => is_void_expr(tail),
        },
        ast::Expr::Match { arms, .. } => {
            // Match is void if its first arm body is void
            arms.first().is_none_or(|arm| is_void_expr(&arm.body))
        }
        _ => false,
    }
}

pub(crate) fn is_string_type(ty: &ast::TypeExpr) -> bool {
    matches!(ty, ast::TypeExpr::Named { name, .. } if name == "String")
}

/// Convert an AST TypeExpr to the corresponding checker Type.
pub(crate) fn lower_type_expr_to_type(ty: &ast::TypeExpr) -> ark_typecheck::types::Type {
    match ty {
        ast::TypeExpr::Named { name, .. } => match name.as_str() {
            "i32" | "u32" | "i16" | "u16" | "i8" | "u8" => ark_typecheck::types::Type::I32,
            "i64" | "u64" => ark_typecheck::types::Type::I64,
            "f64" => ark_typecheck::types::Type::F64,
            "f32" => ark_typecheck::types::Type::F32,
            "bool" => ark_typecheck::types::Type::Bool,
            "char" => ark_typecheck::types::Type::Char,
            "String" => ark_typecheck::types::Type::String,
            _ => ark_typecheck::types::Type::I32,
        },
        ast::TypeExpr::Generic { name, args, .. } if name == "Vec" => {
            let elem = args
                .first()
                .map(lower_type_expr_to_type)
                .unwrap_or(ark_typecheck::types::Type::I32);
            ark_typecheck::types::Type::Vec(Box::new(elem))
        }
        ast::TypeExpr::Unit(_) => ark_typecheck::types::Type::Unit,
        _ => ark_typecheck::types::Type::I32,
    }
}

pub(crate) fn type_expr_name(ty: &ast::TypeExpr) -> String {
    match ty {
        ast::TypeExpr::Named { name, .. } => name.clone(),
        ast::TypeExpr::Generic { name, args, .. } => {
            if args.is_empty() {
                name.clone()
            } else {
                let arg_names: Vec<String> = args.iter().map(type_expr_name).collect();
                format!("{}<{}>", name, arg_names.join(", "))
            }
        }
        ast::TypeExpr::Unit(_) => "()".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Detect specialized Result enum name for concrete i64/f64 payloads.
/// Returns Some("Result_i64_String") for Result<i64, String>, etc.
fn nominalize_type_expr(ty: &ast::TypeExpr) -> Option<String> {
    match ty {
        ast::TypeExpr::Named { name, .. } => Some(name.clone()),
        ast::TypeExpr::Unit(_) => Some("Unit".to_string()),
        ast::TypeExpr::Generic { name, args, .. } => {
            let mut parts = vec![name.clone()];
            for arg in args {
                parts.push(nominalize_type_expr(arg)?);
            }
            Some(parts.join("_"))
        }
        _ => None,
    }
}

pub(crate) fn detect_specialized_result(type_expr: &ast::TypeExpr) -> Option<String> {
    if let ast::TypeExpr::Generic { name, .. } = type_expr
        && name == "Result"
    {
        let specialized = nominalize_type_expr(type_expr)?;
        return matches!(
            specialized.as_str(),
            "Result_i64_String" | "Result_f64_String" | "Result_String_String"
        )
        .then_some(specialized);
    }
    None
}
