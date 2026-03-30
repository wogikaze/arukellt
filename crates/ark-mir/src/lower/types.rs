//! Type inference and conversion helpers for MIR lowering.

use std::collections::HashSet;

use ark_parser::ast;

use crate::mir::*;

use super::LowerCtx;

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

pub(crate) fn type_expr_name(ty: &ast::TypeExpr) -> String {
    match ty {
        ast::TypeExpr::Named { name, .. } => name.clone(),
        ast::TypeExpr::Generic { name, .. } => name.clone(),
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

impl LowerCtx {
    pub(super) fn infer_struct_type(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Ident { name, .. } => {
                let local_id = self.lookup_local(name)?;
                self.struct_typed_locals.get(&local_id.0).cloned()
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                // Chained field access: get parent struct, look up field type
                let parent_struct = self.infer_struct_type(object)?;
                let fields = self.struct_defs.get(&parent_struct)?;
                let field_type = fields
                    .iter()
                    .find(|(fname, _)| fname == field)
                    .map(|(_, ftype)| ftype.clone())?;
                // The field type is the struct name for the nested struct
                if self.struct_defs.contains_key(&field_type) {
                    Some(field_type)
                } else {
                    None
                }
            }
            ast::Expr::Call { callee, .. } => {
                // For method calls returning struct, check return type
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    let struct_name = self.infer_struct_type(object)?;
                    let mangled = format!("{}__{}", struct_name, field);
                    if let Some(ast::TypeExpr::Named { name, .. }) =
                        self.fn_return_types.get(&mangled)
                        && self.struct_defs.contains_key(name.as_str())
                    {
                        return Some(name.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Infer struct name from an init expression (e.g., StructInit or function call returning struct)
    pub(super) fn infer_struct_from_init(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::StructInit { name, .. } => {
                if self.struct_defs.contains_key(name.as_str()) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            ast::Expr::Call { callee, .. } => {
                // Check if function returns a struct type (unqualified call)
                if let ast::Expr::Ident { name, .. } = callee.as_ref()
                    && let Some(ast::TypeExpr::Named { name: tname, .. }) =
                        self.fn_return_types.get(name)
                    && self.struct_defs.contains_key(tname.as_str())
                {
                    return Some(tname.clone());
                }
                // Check qualified call (module::FnName) — look up by plain name
                if let ast::Expr::QualifiedIdent { name, .. } = callee.as_ref() {
                    let lookup = self.fn_return_types.get(name.as_str());
                    if let Some(ast::TypeExpr::Named { name: tname, .. }) = lookup
                        && self.struct_defs.contains_key(tname.as_str())
                    {
                        return Some(tname.clone());
                    }
                }
                // Check method call returning struct
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref()
                    && let Some(struct_name) = self.infer_struct_type(object)
                {
                    let mangled = format!("{}__{}", struct_name, field);
                    if let Some(ast::TypeExpr::Named { name: tname, .. }) =
                        self.fn_return_types.get(&mangled)
                        && self.struct_defs.contains_key(tname.as_str())
                    {
                        return Some(tname.clone());
                    }
                }
                None
            }
            ast::Expr::Binary { span, .. } => {
                // Check if operator overloading returns a struct
                if let Some((mangled, _)) = self.method_resolutions.get(&span.start)
                    && let Some(ast::TypeExpr::Named { name: tname, .. }) =
                        self.fn_return_types.get(mangled)
                    && self.struct_defs.contains_key(tname.as_str())
                {
                    return Some(tname.clone());
                }
                None
            }
            _ => None,
        }
    }

    /// Infer the return TypeExpr for a call expression by looking up fn_return_types.
    pub(super) fn infer_return_type_expr(&self, expr: &ast::Expr) -> Option<ast::TypeExpr> {
        match expr {
            ast::Expr::Call { callee, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    return self.fn_return_types.get(name).cloned();
                }
                if let ast::Expr::QualifiedIdent { module, name, .. } = callee.as_ref() {
                    let qualified = format!("{}::{}", module, name);
                    if let Some(ret) = self.fn_return_types.get(&qualified).cloned() {
                        return Some(ret);
                    }
                    return self.fn_return_types.get(name.as_str()).cloned();
                }
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref()
                    && let Some(struct_name) = self.infer_struct_type(object)
                {
                    let mangled = format!("{}__{}", struct_name, field);
                    return self.fn_return_types.get(&mangled).cloned();
                }
                None
            }
            _ => None,
        }
    }

    /// Check if an identifier is a known enum variant constructor.
    #[allow(dead_code)]
    pub(super) fn is_enum_variant_call(&self, name: &str) -> bool {
        self.bare_variant_tags.contains_key(name)
    }

    /// Detect which elements of a tuple-returning expression are strings.
    /// For a call like `pair(42, String_from("hello"))`, returns {1} since arg[1] is String.
    pub(super) fn detect_string_tuple_elements(
        &self,
        init_expr: &ast::Expr,
        op: &Operand,
        _arity: usize,
    ) -> HashSet<usize> {
        let mut result = HashSet::new();
        // If the init expression is a direct tuple, check each element
        if let ast::Expr::Tuple { elements, .. } = init_expr {
            for (i, elem) in elements.iter().enumerate() {
                if self.is_string_ast_expr(elem) {
                    result.insert(i);
                }
            }
            return result;
        }
        // If the init expression is a call to a generic function, check the arguments
        if let ast::Expr::Call { callee, args, .. } = init_expr {
            #[allow(clippy::collapsible_match)]
            if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                // Look up the function's return type
                if let Some(ret_ty) = self.fn_return_types.get(name) {
                    // If the return type is a tuple, map args to tuple elements
                    if let ast::TypeExpr::Tuple(tuple_types, _) = ret_ty {
                        // Check if the function maps args directly to tuple elements
                        // (common for pair-like functions)
                        if tuple_types.len() == args.len() {
                            for (i, arg) in args.iter().enumerate() {
                                if self.is_string_ast_expr(arg) {
                                    result.insert(i);
                                }
                            }
                        }
                    }
                }
            }
        }
        // If the operand itself is a StructInit (lowered tuple), check fields
        if let Operand::StructInit { fields, .. } = op {
            for (i, (_, field_op)) in fields.iter().enumerate() {
                if self.is_string_operand_mir(field_op) {
                    result.insert(i);
                }
            }
        }
        result
    }

    /// Check if an AST expression produces a String value.
    pub(super) fn is_string_ast_expr(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::StringLit { .. } => true,
            ast::Expr::Call { callee, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    matches!(
                        name.as_str(),
                        "String_from"
                            | "String_new"
                            | "concat"
                            | "slice"
                            | "join"
                            | "i32_to_string"
                            | "i64_to_string"
                            | "f64_to_string"
                            | "f32_to_string"
                            | "bool_to_string"
                            | "char_to_string"
                            | "to_lower"
                            | "to_upper"
                            | "clone"
                            | "to_string"
                    )
                } else {
                    false
                }
            }
            ast::Expr::Ident { name, .. } => {
                if let Some(lid) = self.lookup_local(name) {
                    self.string_locals.contains(&lid.0)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a MIR operand represents a String value.
    pub(super) fn is_string_operand_mir(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstString(_) => true,
            Operand::Call(name, _) => {
                matches!(
                    name.as_str(),
                    "String_from"
                        | "String_new"
                        | "concat"
                        | "slice"
                        | "join"
                        | "i32_to_string"
                        | "i64_to_string"
                        | "f64_to_string"
                        | "f32_to_string"
                        | "bool_to_string"
                        | "char_to_string"
                        | "to_lower"
                        | "to_upper"
                        | "clone"
                        | "to_string"
                ) || name.ends_with("__to_string")
            }
            Operand::Place(Place::Local(lid)) => self.string_locals.contains(&lid.0),
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => {
                then_result
                    .as_ref()
                    .is_some_and(|r| self.is_string_operand_mir(r))
                    || else_result
                        .as_ref()
                        .is_some_and(|r| self.is_string_operand_mir(r))
            }
            _ => false,
        }
    }

    /// Check if a MIR operand produces an f64 value.
    pub(super) fn is_f64_operand_mir(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstF64(_) => true,
            Operand::Call(name, _) => {
                if matches!(name.as_str(), "sqrt") {
                    return true;
                }
                // Check fn_return_types for user-defined functions returning f64
                if let Some(ret_ty) = self.fn_return_types.get(name.as_str()) {
                    return matches!(ret_ty, ast::TypeExpr::Named { name: n, .. } if n == "f64");
                }
                false
            }
            Operand::BinOp(_, l, r) => self.is_f64_operand_mir(l) || self.is_f64_operand_mir(r),
            Operand::Place(Place::Local(lid)) => self.f64_locals.contains(&lid.0),
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => {
                then_result
                    .as_ref()
                    .is_some_and(|r| self.is_f64_operand_mir(r))
                    || else_result
                        .as_ref()
                        .is_some_and(|r| self.is_f64_operand_mir(r))
            }
            _ => false,
        }
    }

    pub(super) fn is_i64_operand_mir(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstI64(_) | Operand::ConstU64(_) => true,
            Operand::Call(name, _) => {
                if matches!(name.as_str(), "clock_now" | "monotonic_now") {
                    return true;
                }
                // Check fn_return_types for user-defined functions returning i64
                if let Some(ret_ty) = self.fn_return_types.get(name.as_str()) {
                    return matches!(ret_ty, ast::TypeExpr::Named { name: n, .. } if n == "i64" || n == "u64");
                }
                false
            }
            Operand::BinOp(_, l, r) => self.is_i64_operand_mir(l) || self.is_i64_operand_mir(r),
            Operand::Place(Place::Local(lid)) => self.i64_locals.contains(&lid.0),
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => {
                then_result
                    .as_ref()
                    .is_some_and(|r| self.is_i64_operand_mir(r))
                    || else_result
                        .as_ref()
                        .is_some_and(|r| self.is_i64_operand_mir(r))
            }
            _ => false,
        }
    }
}
