//! Expression type checking (literals, calls, binop, field access, match, etc.).

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_parser::ast;

use crate::typed_ast::TypedExprInfo;
use crate::types::Type;

use super::{TypeChecker, TypeEnv};

fn moved_host_prelude_diagnostic(name: &str, span: Span) -> Option<Diagnostic> {
    match name {
        "println" => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host I/O is no longer available from the prelude")
                .with_note("import `use std::host::stdio` and call `stdio::println(...)`"),
        ),
        "print" => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host I/O is no longer available from the prelude")
                .with_note("import `use std::host::stdio` and call `stdio::print(...)`"),
        ),
        "eprintln" => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host I/O is no longer available from the prelude")
                .with_note("import `use std::host::stdio` and call `stdio::eprintln(...)`"),
        ),
        "fs_read_file" => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host filesystem access is no longer available from the prelude")
                .with_note("import `use std::host::fs` and call `fs::read_to_string(...)`"),
        ),
        "fs_write_file" => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host filesystem access is no longer available from the prelude")
                .with_note("import `use std::host::fs` and call `fs::write_string(...)`"),
        ),
        "clock_now" => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host clock access is no longer available from the prelude")
                .with_note("import `use std::host::clock` and call `clock::monotonic_now()`"),
        ),
        "random_i32" => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host randomness is no longer available from the prelude")
                .with_note("import `use std::host::random as host_random` and call `host_random::random_i32()`"),
        ),
        _ => None,
    }
}

fn moved_qualified_diagnostic(module: &str, name: &str, span: Span) -> Option<Diagnostic> {
    match (module, name) {
        ("time", "monotonic_now") => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host clock reads moved out of `std::time`")
                .with_note("import `use std::host::clock` and call `clock::monotonic_now()`"),
        ),
        ("random", "random_i32") => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host randomness moved out of `std::random`")
                .with_note("import `use std::host::random as host_random` and call `host_random::random_i32()`"),
        ),
        ("random", "random_i32_range") => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host randomness moved out of `std::random`")
                .with_note("import `use std::host::random as host_random` and call `host_random::random_i32_range(...)`"),
        ),
        ("random", "random_bool") => Some(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_label(span, "host randomness moved out of `std::random`")
                .with_note("import `use std::host::random as host_random` and call `host_random::random_bool()`"),
        ),
        _ => None,
    }
}

impl TypeChecker {
    /// Synthesize the type of an expression.
    pub(crate) fn synthesize_expr(
        &mut self,
        expr: &ast::Expr,
        env: &mut TypeEnv,
        sink: &mut DiagnosticSink,
    ) -> Type {
        match expr {
            ast::Expr::IntLit {
                suffix: Some(s), ..
            } => self.suffix_to_type(s),
            ast::Expr::IntLit { .. } => Type::I32,
            ast::Expr::FloatLit {
                suffix: Some(s), ..
            } => self.suffix_to_type(s),
            ast::Expr::FloatLit { .. } => Type::F64,
            ast::Expr::StringLit { .. } => Type::String,
            ast::Expr::CharLit { .. } => Type::Char,
            ast::Expr::BoolLit { .. } => Type::Bool,
            ast::Expr::Ident { name, span } => {
                if let Some(ty) = env.lookup(name) {
                    ty.clone()
                } else if self.fn_sigs.contains_key(name) {
                    // Function reference
                    let sig = self.fn_sigs[name].clone();
                    Type::Function {
                        params: sig.params,
                        ret: Box::new(sig.ret),
                    }
                } else if name == "None" || name == "true" || name == "false" {
                    // Known prelude values — don't emit an error
                    Type::I32
                } else if name.starts_with("Vec_new_") {
                    // Dynamic Vec constructor for any type (e.g., Vec_new_Point)
                    Type::Function {
                        params: vec![],
                        ret: Box::new(Type::I32),
                    }
                } else if let Some(diag) = moved_host_prelude_diagnostic(name, *span) {
                    sink.emit(diag);
                    Type::Error
                } else {
                    // "Did you mean?" suggestion
                    let candidates: Vec<&str> = self
                        .fn_sigs
                        .keys()
                        .map(|s| s.as_str())
                        .chain(env.bindings.keys().map(|s| s.as_str()))
                        .chain(self.struct_defs.keys().map(|s| s.as_str()))
                        .collect();
                    let mut best_dist = usize::MAX;
                    let mut best_name = "";
                    for c in &candidates {
                        if c.starts_with("__") {
                            continue;
                        }
                        let d = edit_distance(name, c);
                        if d < best_dist && d <= 2 {
                            best_dist = d;
                            best_name = c;
                        }
                    }
                    let mut diag = Diagnostic::new(DiagnosticCode::E0100)
                        .with_label(*span, format!("unresolved name `{}`", name));
                    if !best_name.is_empty() {
                        diag = diag.with_suggestion(format!("did you mean `{}`?", best_name));
                    }
                    sink.emit(diag);
                    Type::Error
                }
            }
            ast::Expr::Binary {
                left,
                op,
                right,
                span,
                ..
            } => {
                let left_ty = self.synthesize_expr(left, env, sink);
                let right_ty = self.synthesize_expr(right, env, sink);
                self.check_binary_op(op, &left_ty, &right_ty, *span, sink)
            }
            ast::Expr::Unary { op, operand, .. } => {
                let operand_ty = self.synthesize_expr(operand, env, sink);
                self.check_unary_op(op, &operand_ty, sink)
            }
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                // Check for method call: x.method(args)
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    let obj_ty = self.synthesize_expr(object, env, sink);
                    if let Type::Struct(type_id) = &obj_ty {
                        // Find struct name from type_id
                        let struct_name = self
                            .struct_defs
                            .values()
                            .find(|s| s.type_id == *type_id)
                            .map(|s| s.name.clone());
                        if let Some(sname) = struct_name {
                            let mangled = format!("{}__{}", sname, field);
                            if let Some(sig) = self.fn_sigs.get(&mangled).cloned() {
                                // Method found — type check args (skip self param)
                                let expected_params = if sig.params.len() > 1 {
                                    &sig.params[1..]
                                } else {
                                    &[]
                                };
                                if args.len() != expected_params.len() {
                                    sink.emit(Diagnostic::new(DiagnosticCode::E0202).with_message(
                                        format!(
                                            "method `{}` expected {} argument(s), found {}",
                                            field,
                                            expected_params.len(),
                                            args.len()
                                        ),
                                    ));
                                }
                                for a in args {
                                    self.synthesize_expr(a, env, sink);
                                }
                                // Record method resolution for MIR lowering
                                self.method_resolutions
                                    .insert(span.start, (mangled.clone(), sname.clone()));
                                let expr_id = self.node_ids.fresh_expr();
                                self.typed_ast_map.register_span(span.start, expr_id);
                                self.typed_ast_map.insert_expr(
                                    expr_id,
                                    TypedExprInfo {
                                        ty: sig.ret.clone(),
                                        method_resolution: Some((mangled, sname)),
                                    },
                                );
                                return sig.ret;
                            }
                        }
                    }
                    // Not a method call — synthesize args but return I32 (struct field or error)
                    for a in args {
                        self.synthesize_expr(a, env, sink);
                    }
                    return Type::I32;
                }

                let callee_ty = self.synthesize_expr(callee, env, sink);
                match callee_ty {
                    Type::Function { params, ret } => {
                        if args.len() != params.len() {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0202).with_message(
                                format!(
                                    "expected {} argument(s), found {}",
                                    params.len(),
                                    args.len()
                                ),
                            ));
                        }
                        // Generic Vec function inference: if the declared param is Vec<i32>
                        // but actual arg is Vec<String>, adjust the return type accordingly.
                        let callee_name = match callee.as_ref() {
                            ast::Expr::Ident { name, .. } => Some(name.as_str()),
                            _ => None,
                        };
                        if let Some(fname) = callee_name {
                            let arg_types: Vec<Type> = args
                                .iter()
                                .map(|a| self.synthesize_expr(a, env, sink))
                                .collect();
                            match fname {
                                "len" | "sort_i32" => {
                                    // Always returns i32/Unit regardless of element type
                                    *ret
                                }
                                "get_unchecked" => {
                                    if let Some(Type::Vec(elem_ty)) = arg_types.first() {
                                        *elem_ty.clone()
                                    } else {
                                        *ret
                                    }
                                }
                                "get" => {
                                    if let Some(Type::Vec(elem_ty)) = arg_types.first() {
                                        Type::Option(elem_ty.clone())
                                    } else {
                                        *ret
                                    }
                                }
                                "push" | "set" => Type::Unit,
                                "pop" => {
                                    if let Some(Type::Vec(elem_ty)) = arg_types.first() {
                                        Type::Option(elem_ty.clone())
                                    } else {
                                        *ret
                                    }
                                }
                                "to_string" => {
                                    // Polymorphic: accepts any displayable type, returns String
                                    // For struct types, check Display trait impl
                                    if let Some(arg_ty) = arg_types.first() {
                                        match arg_ty {
                                            Type::I32
                                            | Type::I64
                                            | Type::F64
                                            | Type::Bool
                                            | Type::Char
                                            | Type::String => Type::String,
                                            Type::Struct(tid) => {
                                                // Look up struct name from type_id
                                                let sname = self
                                                    .struct_defs
                                                    .iter()
                                                    .find(|(_, info)| info.type_id == *tid)
                                                    .map(|(name, _)| name.clone());
                                                if let Some(ref name) = sname {
                                                    let has_display = self
                                                        .trait_impls
                                                        .get(name)
                                                        .is_some_and(|traits| {
                                                            traits.contains(&"Display".to_string())
                                                        });
                                                    if !has_display {
                                                        sink.emit(
                                                            Diagnostic::new(DiagnosticCode::E0200)
                                                                .with_message(format!(
                                                                    "type `{}` does not implement `Display` trait; add `impl Display for {} {{ fn to_string(self) -> String {{ ... }} }}`",
                                                                    name, name
                                                                )),
                                                        );
                                                    }
                                                }
                                                Type::String
                                            }
                                            _ => Type::String,
                                        }
                                    } else {
                                        Type::String
                                    }
                                }
                                _ => *ret,
                            }
                        } else {
                            *ret
                        }
                    }
                    Type::Error => Type::Error,
                    _ => {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0200)
                                .with_message(format!("expected function, found `{}`", callee_ty)),
                        );
                        Type::Error
                    }
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let cond_ty = self.synthesize_expr(cond, env, sink);
                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                            "if condition must be `bool`, found `{}`",
                            cond_ty
                        )),
                    );
                }
                let then_ty = self.check_block(then_block, env, &Type::Unit, sink);
                if let Some(else_blk) = else_block {
                    let else_ty = self.check_block(else_blk, env, &then_ty, sink);
                    if !self.types_compatible(&then_ty, &else_ty) {
                        sink.emit(Diagnostic::new(DiagnosticCode::E0205).with_message(format!(
                            "if/else branches have incompatible types: `{}` vs `{}`",
                            then_ty, else_ty
                        )));
                    }
                    then_ty
                } else {
                    Type::Unit
                }
            }
            ast::Expr::Block(block) => self.check_block(block, env, &Type::Unit, sink),
            ast::Expr::Tuple { elements, .. } => Type::Tuple(
                elements
                    .iter()
                    .map(|e| self.synthesize_expr(e, env, sink))
                    .collect(),
            ),
            ast::Expr::Array { elements, .. } => {
                if elements.is_empty() {
                    Type::Error // need type annotation
                } else {
                    let first = self.synthesize_expr(&elements[0], env, sink);
                    Type::Array(Box::new(first), elements.len() as u64)
                }
            }
            ast::Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.synthesize_expr(val, env, sink);
                }
                Type::Never
            }
            ast::Expr::Break { value, .. } => {
                if let Some(val) = value {
                    self.synthesize_expr(val, env, sink);
                }
                Type::Never
            }
            ast::Expr::Continue { .. } => Type::Never,
            ast::Expr::Loop { body, .. } => {
                self.check_block(body, env, &Type::Unit, sink);
                // Loop as expression: type comes from break value
                Type::I32
            }
            ast::Expr::Assign { target, value, .. } => {
                let val_ty = self.synthesize_expr(value, env, sink);
                if let ast::Expr::Ident { name, span } = target.as_ref() {
                    // Check mutability
                    if env.lookup(name).is_some() && !env.is_mutable(name) {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0207)
                                .with_message(format!(
                                    "cannot assign to immutable variable `{}`",
                                    name
                                ))
                                .with_label(*span, "cannot assign to immutable variable"),
                        );
                    }
                    if let Some(target_ty) = env.lookup(name) {
                        let target_ty = target_ty.clone();
                        if !self.types_compatible(&val_ty, &target_ty) {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_label(
                                *span,
                                format!("expected `{}`, found `{}`", target_ty, val_ty),
                            ));
                        }
                    }
                }
                Type::Unit
            }
            ast::Expr::Match {
                scrutinee,
                arms,
                span,
            } => {
                let scrutinee_ty = self.synthesize_expr(scrutinee, env, sink);
                let mut result_ty: Option<Type> = None;
                let mut has_wildcard = false;
                let mut covered_variants: Vec<String> = Vec::new();
                for arm in arms {
                    // Create a child env for each arm to bind pattern variables
                    let mut arm_env = env.clone();

                    // Track pattern coverage and bind pattern variables
                    match &arm.pattern {
                        ast::Pattern::Wildcard(_) => {
                            has_wildcard = true;
                        }
                        ast::Pattern::Ident { name, .. } => {
                            has_wildcard = true;
                            arm_env.bind(name.clone(), scrutinee_ty.clone());
                        }
                        ast::Pattern::Enum {
                            path,
                            variant,
                            fields,
                            ..
                        } => {
                            covered_variants.push(if path.is_empty() {
                                variant.clone()
                            } else {
                                format!("{}::{}", path, variant)
                            });
                            // Bind fields from enum variant payload
                            // Handle Option<T> and Result<T,E> specially since they use
                            // Type::Option / Type::Result, not Type::Enum
                            let mut bound_from_builtin = false;
                            if variant == "Some" {
                                if let Type::Option(ref inner) = scrutinee_ty {
                                    if let Some(ast::Pattern::Ident { name, .. }) = fields.first() {
                                        arm_env.bind(name.clone(), *inner.clone());
                                        bound_from_builtin = true;
                                    }
                                }
                            } else if variant == "Ok" {
                                if let Type::Result(ref ok_ty, _) = scrutinee_ty {
                                    if let Some(ast::Pattern::Ident { name, .. }) = fields.first() {
                                        arm_env.bind(name.clone(), *ok_ty.clone());
                                        bound_from_builtin = true;
                                    }
                                }
                            } else if variant == "Err" {
                                if let Type::Result(_, ref err_ty) = scrutinee_ty {
                                    if let Some(ast::Pattern::Ident { name, .. }) = fields.first() {
                                        arm_env.bind(name.clone(), *err_ty.clone());
                                        bound_from_builtin = true;
                                    }
                                }
                            }
                            if !bound_from_builtin {
                                let enum_name = if path.is_empty() {
                                    // Try to infer from scrutinee type
                                    if let Type::Enum(ref tid) = scrutinee_ty {
                                        self.enum_defs
                                            .iter()
                                            .find(|(_, info)| &info.type_id == tid)
                                            .map(|(n, _)| n.clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    Some(path.clone())
                                };
                                if let Some(ref ename) = enum_name {
                                    if let Some(info) = self.enum_defs.get(ename) {
                                        if let Some(vinfo) =
                                            info.variants.iter().find(|v| v.name == *variant)
                                        {
                                            for (i, field_pat) in fields.iter().enumerate() {
                                                if let ast::Pattern::Ident { name, .. } = field_pat
                                                {
                                                    let field_ty = vinfo
                                                        .fields
                                                        .get(i)
                                                        .cloned()
                                                        .unwrap_or(Type::Error);
                                                    arm_env.bind(name.clone(), field_ty);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ast::Pattern::Struct {
                            name: sname,
                            fields: sfields,
                            ..
                        } => {
                            // Check if this is an enum struct variant pattern: "EnumName::VariantName"
                            if let Some((enum_name, variant_name)) = sname.split_once("::") {
                                covered_variants.push(sname.clone());
                                if let Some(einfo) = self.enum_defs.get(enum_name) {
                                    if let Some(vinfo) =
                                        einfo.variants.iter().find(|v| v.name == variant_name)
                                    {
                                        for (fname, fpat) in sfields {
                                            let binding_name = match fpat {
                                                Some(ast::Pattern::Ident { name: n, .. }) => {
                                                    n.clone()
                                                }
                                                None => fname.clone(),
                                                _ => fname.clone(),
                                            };
                                            let field_ty = vinfo
                                                .field_names
                                                .iter()
                                                .position(|n| n == fname)
                                                .and_then(|idx| vinfo.fields.get(idx))
                                                .cloned()
                                                .unwrap_or(Type::Error);
                                            arm_env.bind(binding_name, field_ty);
                                        }
                                    }
                                }
                            } else {
                                has_wildcard = true;
                                if let Some(sinfo) = self.struct_defs.get(sname.as_str()).cloned() {
                                    for (fname, fpat) in sfields {
                                        let binding_name = match fpat {
                                            Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                                            None => fname.clone(),
                                            _ => fname.clone(),
                                        };
                                        let field_ty = sinfo
                                            .fields
                                            .iter()
                                            .find(|(n, _)| n == fname)
                                            .map(|(_, t)| t.clone())
                                            .unwrap_or(Type::Error);
                                        arm_env.bind(binding_name, field_ty);
                                    }
                                }
                            }
                        }
                        ast::Pattern::Or { patterns, .. } => {
                            // Bind variables from the first sub-pattern
                            for pat in patterns.iter().take(1) {
                                match pat {
                                    ast::Pattern::Ident { name, .. } => {
                                        has_wildcard = true;
                                        arm_env.bind(name.clone(), scrutinee_ty.clone());
                                    }
                                    ast::Pattern::Wildcard(_) => {
                                        has_wildcard = true;
                                    }
                                    ast::Pattern::Enum { path, variant, .. } => {
                                        covered_variants.push(if path.is_empty() {
                                            variant.clone()
                                        } else {
                                            format!("{}::{}", path, variant)
                                        });
                                    }
                                    _ => {}
                                }
                            }
                            // Track coverage for remaining or-patterns
                            for pat in patterns.iter().skip(1) {
                                if let ast::Pattern::Enum { path, variant, .. } = pat {
                                    covered_variants.push(if path.is_empty() {
                                        variant.clone()
                                    } else {
                                        format!("{}::{}", path, variant)
                                    });
                                }
                            }
                        }
                        ast::Pattern::IntLit { .. }
                        | ast::Pattern::StringLit { .. }
                        | ast::Pattern::BoolLit { .. }
                        | ast::Pattern::CharLit { .. }
                        | ast::Pattern::FloatLit { .. }
                        | ast::Pattern::Tuple { .. } => {}
                    }
                    // Check guard expression if present
                    if let Some(ref guard) = arm.guard {
                        self.synthesize_expr(guard, &mut arm_env, sink);
                    }
                    // Synthesize arm body type with the arm's env
                    let arm_ty = self.synthesize_expr(&arm.body, &mut arm_env, sink);
                    if let Some(ref first) = result_ty {
                        if !self.types_compatible(&arm_ty, first)
                            && arm_ty != Type::Error
                            && *first != Type::Error
                        {
                            sink.emit(
                                Diagnostic::new(DiagnosticCode::E0205)
                                    .with_message("mismatched match arm types")
                                    .with_label(
                                        *span,
                                        format!("expected `{}`, found `{}`", first, arm_ty),
                                    ),
                            );
                        }
                    } else {
                        result_ty = Some(arm_ty);
                    }
                }
                // Check exhaustiveness for enum types
                if !has_wildcard {
                    if let Type::Enum(ref type_id) = scrutinee_ty {
                        // Find enum info by TypeId
                        let enum_entry = self
                            .enum_defs
                            .iter()
                            .find(|(_, info)| &info.type_id == type_id);
                        if let Some((enum_name, info)) = enum_entry {
                            let enum_name = enum_name.clone();
                            let all_variants: Vec<String> = info
                                .variants
                                .iter()
                                .map(|v| format!("{}::{}", enum_name, v.name))
                                .collect();
                            let missing: Vec<&String> = all_variants
                                .iter()
                                .filter(|v| !covered_variants.contains(v))
                                .collect();
                            if !missing.is_empty() {
                                sink.emit(
                                    Diagnostic::new(DiagnosticCode::E0204)
                                        .with_message("non-exhaustive match")
                                        .with_label(
                                            *span,
                                            format!(
                                                "missing patterns: {}",
                                                missing
                                                    .iter()
                                                    .map(|s| s.as_str())
                                                    .collect::<Vec<_>>()
                                                    .join(", ")
                                            ),
                                        ),
                                );
                            }
                        }
                    }
                }
                result_ty.unwrap_or(Type::Unit)
            }
            ast::Expr::Try { expr, span } => {
                let inner_ty = self.synthesize_expr(expr, env, sink);
                // Check that the current function returns Result
                let fn_returns_result =
                    matches!(&self.current_fn_return_type, Some(Type::Result(_, _)));
                if !fn_returns_result {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0210)
                            .with_message("? operator requires function to return Result")
                            .with_label(*span, "used here".to_string())
                            .with_note("add `-> Result[T, E]` to the function signature"),
                    );
                }
                // Check error type compatibility with From trait support
                if let (Type::Result(_, src_err), Some(Type::Result(_, dst_err))) =
                    (&inner_ty, &self.current_fn_return_type)
                {
                    // Use strict equality (not lenient types_compatible) for ? error checking
                    if src_err != dst_err {
                        // Resolve the actual type name for the destination error
                        let dst_name = self.type_name(dst_err);
                        let src_name = self.type_name(src_err);
                        let from_fn = format!("{}__from", dst_name);
                        let has_from = self
                            .method_table
                            .contains_key(&(dst_name.clone(), "from".to_string()));
                        if has_from {
                            // Record that this ? needs From conversion
                            self.method_resolutions
                                .insert(span.start, (from_fn.clone(), dst_name.clone()));
                            let expr_id = self.node_ids.fresh_expr();
                            self.typed_ast_map.register_span(span.start, expr_id);
                            self.typed_ast_map.insert_expr(
                                expr_id,
                                TypedExprInfo {
                                    ty: inner_ty.clone(),
                                    method_resolution: Some((from_fn, dst_name)),
                                },
                            );
                        } else {
                            sink.emit(
                                Diagnostic::new(DiagnosticCode::E0210)
                                    .with_message(format!(
                                        "? operator: error type `{}` is not compatible with function return error type `{}`",
                                        src_name, dst_name
                                    ))
                                    .with_label(*span, "error type mismatch".to_string())
                                    .with_note(format!(
                                        "implement `From<{}>` for `{}` to enable automatic conversion",
                                        src_name, dst_name
                                    )),
                            );
                        }
                    }
                }
                // The type of ? on Result<T, E> is T
                match inner_ty {
                    Type::Result(ok, _) => *ok,
                    _ => Type::Error,
                }
            }
            ast::Expr::Index { object, index, .. } => {
                let obj_ty = self.synthesize_expr(object, env, sink);
                let _idx_ty = self.synthesize_expr(index, env, sink);
                match obj_ty {
                    Type::Array(elem, _) => *elem,
                    _ => Type::I32,
                }
            }
            ast::Expr::ArrayRepeat { value, count, .. } => {
                let elem_ty = self.synthesize_expr(value, env, sink);
                if let ast::Expr::IntLit { value: n, .. } = count.as_ref() {
                    Type::Array(Box::new(elem_ty), *n as u64)
                } else {
                    Type::Error
                }
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.synthesize_expr(object, env, sink);
                if let Type::Struct(type_id) = &obj_ty {
                    if let Some(info) = self.struct_defs.values().find(|s| s.type_id == *type_id) {
                        if let Some((_, field_ty)) =
                            info.fields.iter().find(|(name, _)| name == field)
                        {
                            return field_ty.clone();
                        }
                    }
                }
                // Struct field access at Wasm level is always i32 (pointer)
                Type::I32
            }
            ast::Expr::StructInit { name, fields, .. } => {
                // Check if this is an enum struct variant: "EnumName::VariantName"
                if let Some((enum_name, _variant_name)) = name.split_once("::") {
                    let enum_tid = self.enum_defs.get(enum_name).map(|e| e.type_id);
                    if let Some(tid) = enum_tid {
                        for (_fname, fexpr) in fields {
                            self.synthesize_expr(fexpr, env, sink);
                        }
                        return Type::Enum(tid);
                    }
                }
                let type_id = self.struct_defs.get(name).map(|info| info.type_id);
                for (_fname, fexpr) in fields {
                    self.synthesize_expr(fexpr, env, sink);
                }
                if let Some(tid) = type_id {
                    Type::Struct(tid)
                } else {
                    Type::I32
                }
            }
            ast::Expr::Closure { params, body, .. } => {
                let mut param_types = Vec::new();
                let mut child_env = env.child();
                for p in params {
                    let ty = if let Some(ty_expr) = &p.ty {
                        self.resolve_type_expr(ty_expr)
                    } else {
                        Type::I32
                    };
                    child_env.bind(p.name.clone(), ty.clone());
                    param_types.push(ty);
                }
                let ret_ty = self.synthesize_expr(body, &mut child_env, sink);
                Type::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                }
            }
            ast::Expr::QualifiedIdent { module, name, span } => {
                // Qualified enum variant or module function reference
                let qualified = format!("{}::{}", module, name);
                if let Some(sig) = self.fn_sigs.get(&qualified).cloned() {
                    Type::Function {
                        params: sig.params,
                        ret: Box::new(sig.ret),
                    }
                } else if let Some(sig) = self.fn_sigs.get(name).cloned() {
                    Type::Function {
                        params: sig.params,
                        ret: Box::new(sig.ret),
                    }
                } else if let Some(info) = self.enum_defs.get(module.as_str()) {
                    let variant = info.variants.iter().find(|v| v.name == *name);
                    match variant {
                        Some(v) if v.fields.is_empty() => {
                            // Unit variant: Direction::South → Type::Enum(Direction)
                            Type::Enum(info.type_id)
                        }
                        Some(v) => {
                            // Tuple variant constructor: Color::Rgb → fn(fields...) -> Enum
                            Type::Function {
                                params: v.fields.clone(),
                                ret: Box::new(Type::Enum(info.type_id)),
                            }
                        }
                        None => Type::Enum(info.type_id),
                    }
                } else if let Some(diag) = moved_qualified_diagnostic(module, name, *span) {
                    sink.emit(diag);
                    Type::Error
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0100)
                            .with_label(*span, format!("unresolved name `{}::{}`", module, name)),
                    );
                    Type::Error
                }
            }
        }
    }

    fn check_binary_op(
        &mut self,
        op: &ast::BinOp,
        left: &Type,
        right: &Type,
        span: Span,
        sink: &mut DiagnosticSink,
    ) -> Type {
        use ast::BinOp::*;
        if *left == Type::Error || *right == Type::Error {
            return Type::Error;
        }
        // Check for operator overloading on struct types
        if let (Type::Struct(left_id), Type::Struct(right_id)) = (left, right) {
            if left_id == right_id {
                let struct_name = self
                    .struct_defs
                    .values()
                    .find(|s| s.type_id == *left_id)
                    .map(|s| s.name.clone());
                if let Some(sname) = struct_name {
                    let op_method = match op {
                        Add => "add",
                        Sub => "sub",
                        Mul => "mul",
                        Div => "div",
                        Mod => "rem",
                        Eq | Ne => "eq",
                        Lt | Le | Gt | Ge => "cmp",
                        _ => "",
                    };
                    if !op_method.is_empty() {
                        let mangled = format!("{}__{}", sname, op_method);
                        if let Some(sig) = self.fn_sigs.get(&mangled).cloned() {
                            // Record method resolution for MIR lowering
                            self.method_resolutions
                                .insert(span.start, (mangled.clone(), sname.clone()));
                            let ret_ty = match op {
                                Eq | Ne | Lt | Le | Gt | Ge => Type::Bool,
                                _ => sig.ret,
                            };
                            let expr_id = self.node_ids.fresh_expr();
                            self.typed_ast_map.register_span(span.start, expr_id);
                            self.typed_ast_map.insert_expr(
                                expr_id,
                                TypedExprInfo {
                                    ty: ret_ty.clone(),
                                    method_resolution: Some((mangled, sname)),
                                },
                            );
                            // For comparison ops, return Bool
                            return ret_ty;
                        }
                    }
                }
            }
        }
        // Promote i32 to wider numeric types for mixed operations
        let (left, right) = match (left, right) {
            (Type::I64, Type::I32) | (Type::I32, Type::I64) => (&Type::I64, &Type::I64),
            (Type::F64, Type::I32) | (Type::I32, Type::F64) => (&Type::F64, &Type::F64),
            (Type::F64, Type::I64) | (Type::I64, Type::F64) => (&Type::F64, &Type::F64),
            _ => (left, right),
        };
        match op {
            Add | Sub | Mul | Div | Mod => {
                if left.is_numeric() && left == right {
                    left.clone()
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "cannot apply arithmetic operator to `{}` and `{}`",
                        left, right
                    )));
                    Type::Error
                }
            }
            Eq | Ne => {
                if left == right {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("cannot compare `{}` with `{}`", left, right)),
                    );
                    Type::Error
                }
            }
            Lt | Le | Gt | Ge => {
                if left.is_numeric() && left == right {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("cannot order `{}` and `{}`", left, right)),
                    );
                    Type::Error
                }
            }
            And | Or => {
                if *left == Type::Bool && *right == Type::Bool {
                    Type::Bool
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "logical operators require `bool`, found `{}` and `{}`",
                        left, right
                    )));
                    Type::Error
                }
            }
            BitAnd | BitOr | BitXor | Shl | Shr => {
                if left.is_integer() && left == right {
                    left.clone()
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "bitwise operators require integer types, found `{}` and `{}`",
                        left, right
                    )));
                    Type::Error
                }
            }
        }
    }

    fn check_unary_op(&self, op: &ast::UnaryOp, operand: &Type, sink: &mut DiagnosticSink) -> Type {
        if *operand == Type::Error {
            return Type::Error;
        }
        match op {
            ast::UnaryOp::Neg => {
                if operand.is_numeric() {
                    operand.clone()
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("cannot negate `{}`", operand)),
                    );
                    Type::Error
                }
            }
            ast::UnaryOp::Not => {
                if *operand == Type::Bool {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                            "logical NOT requires `bool`, found `{}`",
                            operand
                        )),
                    );
                    Type::Error
                }
            }
            ast::UnaryOp::BitNot => {
                if operand.is_integer() {
                    operand.clone()
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "bitwise NOT requires integer, found `{}`",
                        operand
                    )));
                    Type::Error
                }
            }
        }
    }
}

/// Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}
