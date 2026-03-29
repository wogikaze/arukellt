//! Pattern matching lowering for MIR.

use std::collections::HashSet;

use ark_parser::ast;

use crate::mir::*;

use super::LowerCtx;
use super::types::{detect_specialized_result, is_string_type};

impl LowerCtx {
    /// Lower a match expression used as a statement (result discarded).
    /// Converts to nested if-else chains.
    pub(super) fn lower_match_stmt(
        &mut self,
        scrutinee: &ast::Expr,
        arms: &[ast::MatchArm],
        out: &mut Vec<MirStmt>,
    ) {
        let scrut_val = self.lower_expr(scrutinee);
        // Store complex scrutinees in a temp local to avoid re-evaluation
        let scrut = match &scrut_val {
            Operand::Place(_)
            | Operand::ConstI32(_)
            | Operand::ConstBool(_)
            | Operand::ConstString(_)
            | Operand::Unit => scrut_val,
            _ => {
                let tmp = self.declare_local("__match_scrut");
                out.push(MirStmt::Assign(Place::Local(tmp), Rvalue::Use(scrut_val)));
                // If scrutinee is a function call, resolve generic enum payload types
                if let ast::Expr::Call { callee, .. } = scrutinee {
                    if let ast::Expr::Ident { name: fn_name, .. } = callee.as_ref() {
                        self.resolve_enum_payload_types_for_local(tmp, fn_name);
                    }
                }
                Operand::Place(Place::Local(tmp))
            }
        };
        let stmt = self.build_match_if_chain(&scrut, arms, 0, true);
        if let Some(s) = stmt {
            out.push(s);
        }
    }

    /// Resolve generic enum payload types for a local holding a function return value.
    /// E.g., if fn returns Result<i32, String>, mark the Err variant's payload as String.
    fn resolve_enum_payload_types_for_local(&mut self, local: LocalId, fn_name: &str) {
        let ret_ty = if let Some(ty) = self.fn_return_types.get(fn_name) {
            ty.clone()
        } else {
            return;
        };
        match &ret_ty {
            ast::TypeExpr::Generic { name, args, .. } if name == "Result" || name == "Option" => {
                self.enum_typed_locals.insert(local.0, name.clone());
                let mut payload_strings = HashSet::new();
                if name == "Result" && args.len() == 2 {
                    // Result<T, E>: Ok payload is args[0], Err payload is args[1]
                    if is_string_type(&args[0]) {
                        payload_strings.insert(("Ok".to_string(), 0u32));
                    }
                    if is_string_type(&args[1]) {
                        payload_strings.insert(("Err".to_string(), 0u32));
                    }
                } else if name == "Option" && args.len() == 1 {
                    // Option<T>: Some payload is args[0]
                    if is_string_type(&args[0]) {
                        payload_strings.insert(("Some".to_string(), 0u32));
                    }
                }
                if !payload_strings.is_empty() {
                    self.enum_local_payload_strings
                        .insert(local.0, payload_strings);
                }
            }
            _ => {}
        }
    }

    /// Build a nested if-else chain from match arms starting at `idx`.
    /// `as_stmt` indicates whether arm bodies should be lowered as statements.
    #[allow(clippy::only_used_in_recursion)]
    fn build_match_if_chain(
        &mut self,
        scrut: &Operand,
        arms: &[ast::MatchArm],
        idx: usize,
        as_stmt: bool,
    ) -> Option<MirStmt> {
        if idx >= arms.len() {
            return None;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) => {
                // Default arm — check guard if any, otherwise just emit
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    let mut then_body = Vec::new();
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) =
                        self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                    {
                        vec![next]
                    } else {
                        vec![]
                    };
                    Some(MirStmt::IfStmt {
                        cond: guard_cond,
                        then_body,
                        else_body,
                    })
                } else {
                    let mut body = Vec::new();
                    self.lower_expr_stmt(&arm.body, &mut body);
                    if body.len() == 1 {
                        Some(body.remove(0))
                    } else {
                        Some(MirStmt::IfStmt {
                            cond: Operand::ConstBool(true),
                            then_body: body,
                            else_body: vec![],
                        })
                    }
                }
            }
            ast::Pattern::IntLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::BoolLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::StringLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstString(value.clone())),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::Ident { name, .. } => {
                // Binding pattern — bind the scrutinee to the name
                let local_id = self.declare_local(name);
                if let Some(guard) = &arm.guard {
                    // Bind first, then check guard
                    let mut outer_body = vec![MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(scrut.clone()),
                    )];
                    let guard_cond = self.lower_expr(guard);
                    let mut then_body = Vec::new();
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) =
                        self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                    {
                        vec![next]
                    } else {
                        vec![]
                    };
                    outer_body.push(MirStmt::IfStmt {
                        cond: guard_cond,
                        then_body,
                        else_body,
                    });
                    Some(MirStmt::IfStmt {
                        cond: Operand::ConstBool(true),
                        then_body: outer_body,
                        else_body: vec![],
                    })
                } else {
                    let mut then_body = vec![MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(scrut.clone()),
                    )];
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) =
                        self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                    {
                        vec![next]
                    } else {
                        vec![]
                    };
                    Some(MirStmt::IfStmt {
                        cond: Operand::ConstBool(true),
                        then_body,
                        else_body,
                    })
                }
            }
            ast::Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    // Compare tag: enum ptr -> i32.load at offset 0
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    let mut then_body = Vec::new();
                    // Determine if scrutinee has known payload string types
                    let payload_strings = if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_local_payload_strings.get(&lid.0).cloned()
                    } else {
                        None
                    };
                    // Determine specialized enum name for i64/f64 payloads
                    let effective_enum_name = match scrut {
                        Operand::Place(Place::Local(lid)) => self
                            .enum_local_specialized
                            .get(&lid.0)
                            .cloned()
                            .unwrap_or_else(|| path.clone()),
                        Operand::Call(name, _) => self
                            .fn_return_types
                            .get(name)
                            .and_then(detect_specialized_result)
                            .or_else(|| {
                                self.fn_return_types.get(name).and_then(|ret_ty| {
                                    if let ast::TypeExpr::Named { name, .. } = ret_ty {
                                        self.enum_defs
                                            .contains_key(name.as_str())
                                            .then(|| name.clone())
                                    } else {
                                        None
                                    }
                                })
                            })
                            .unwrap_or_else(|| path.clone()),
                        _ => path.clone(),
                    };
                    if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_typed_locals
                            .entry(lid.0)
                            .or_insert_with(|| effective_enum_name.clone());
                    }
                    // Bind payload fields to local variables
                    for (i, field_pat) in fields.iter().enumerate() {
                        if let ast::Pattern::Ident { name: binding, .. } = field_pat {
                            let local_id = self.declare_local(binding);
                            // Check if this payload field is a string
                            if let Some(ref ps) = payload_strings {
                                if ps.contains(&(variant.clone(), i as u32)) {
                                    self.string_locals.insert(local_id.0);
                                }
                            }
                            // Check if this payload field is f64, i64, or String
                            if let Some(variants) = self.enum_defs.get(effective_enum_name.as_str())
                            {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant)
                                {
                                    if let Some(t) = types.get(i) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "i64" {
                                            self.i64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                        if self.enum_defs.contains_key(t.as_str()) {
                                            self.enum_typed_locals.insert(local_id.0, t.clone());
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: i as u32,
                                enum_name: effective_enum_name.clone(),
                                variant_name: variant.clone(),
                            };
                            then_body.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                    }
                    if let Some(guard) = &arm.guard {
                        // Guard references pattern bindings, which are in then_body.
                        // Wrap: if(tag_match) { bind; if(guard) { body } else { next } }
                        let guard_cond = self.lower_expr(guard);
                        let mut inner_then = Vec::new();
                        self.lower_expr_stmt(&arm.body, &mut inner_then);
                        let else_body = if let Some(next) =
                            self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                        {
                            vec![next]
                        } else {
                            vec![]
                        };
                        then_body.push(MirStmt::IfStmt {
                            cond: guard_cond,
                            then_body: inner_then,
                            else_body: else_body.clone(),
                        });
                        Some(MirStmt::IfStmt {
                            cond,
                            then_body,
                            else_body,
                        })
                    } else {
                        self.lower_expr_stmt(&arm.body, &mut then_body);
                        let else_body = if let Some(next) =
                            self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                        {
                            vec![next]
                        } else {
                            vec![]
                        };
                        Some(MirStmt::IfStmt {
                            cond,
                            then_body,
                            else_body,
                        })
                    }
                } else {
                    self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                }
            }
            ast::Pattern::Or { patterns, .. } => {
                // Or-pattern: try each sub-pattern, share the body
                // Build: if pat1_cond || pat2_cond || ... { body } else { next }
                let mut combined_cond: Option<Operand> = None;
                for pat in patterns {
                    let sub_cond = self.pattern_to_condition(scrut, pat);
                    combined_cond = Some(match combined_cond {
                        Some(prev) => Operand::BinOp(BinOp::Or, Box::new(prev), Box::new(sub_cond)),
                        None => sub_cond,
                    });
                }
                let mut cond = combined_cond.unwrap_or(Operand::ConstBool(false));
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::Struct { name, fields, .. } => {
                // Check if this is an enum struct variant pattern: "EnumName::VariantName"
                if let Some((enum_name, variant_name)) = name.split_once("::") {
                    let key = format!("{}::{}", enum_name, variant_name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let cond = Operand::BinOp(
                            BinOp::Eq,
                            Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                            Box::new(Operand::ConstI32(tag)),
                        );
                        let mut then_body = Vec::new();
                        let def_field_names = self
                            .enum_variant_field_names
                            .get(&key)
                            .cloned()
                            .unwrap_or_default();
                        for (fname, fpat) in fields {
                            let binding_name = match fpat {
                                Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                                None => fname.clone(),
                                _ => fname.clone(),
                            };
                            let local_id = self.declare_local(&binding_name);
                            // Determine field index from definition order
                            let field_idx =
                                def_field_names.iter().position(|n| n == fname).unwrap_or(0);
                            // Track f64/i64/String types
                            if let Some(variants) = self.enum_defs.get(enum_name) {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant_name)
                                {
                                    if let Some(t) = types.get(field_idx) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "i64" {
                                            self.i64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                        if self.enum_defs.contains_key(t.as_str()) {
                                            self.enum_typed_locals.insert(local_id.0, t.clone());
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: field_idx as u32,
                                enum_name: enum_name.to_string(),
                                variant_name: variant_name.to_string(),
                            };
                            then_body.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                        self.lower_expr_stmt(&arm.body, &mut then_body);
                        let else_body = if let Some(next) =
                            self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                        {
                            vec![next]
                        } else {
                            vec![]
                        };
                        return Some(MirStmt::IfStmt {
                            cond,
                            then_body,
                            else_body,
                        });
                    }
                }
                // Regular struct pattern: bind fields from struct
                let mut then_body = Vec::new();
                for (fname, fpat) in fields {
                    let binding_name = match fpat {
                        Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                        None => fname.clone(),
                        _ => fname.clone(),
                    };
                    let local_id = self.declare_local(&binding_name);
                    // Detect f64/String fields from struct_defs
                    if let Some(sdef) = self.struct_defs.get(name.as_str()) {
                        if let Some((_, ftype)) = sdef.iter().find(|(n, _)| n == fname) {
                            if ftype == "f64" {
                                self.f64_locals.insert(local_id.0);
                            }
                            if ftype == "String" {
                                self.string_locals.insert(local_id.0);
                            }
                        }
                    }
                    let field_access = Operand::FieldAccess {
                        object: Box::new(scrut.clone()),
                        struct_name: name.clone(),
                        field: fname.clone(),
                    };
                    then_body.push(MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(field_access),
                    ));
                }
                let cond = if let Some(guard) = &arm.guard {
                    self.lower_expr(guard)
                } else {
                    Operand::ConstBool(true)
                };
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            _ => {
                // Skip unsupported patterns, try next arm
                self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
            }
        }
    }

    /// Convert a single pattern to a condition operand (for or-patterns).
    fn pattern_to_condition(&self, scrut: &Operand, pattern: &ast::Pattern) -> Operand {
        match pattern {
            ast::Pattern::Wildcard(_) | ast::Pattern::Ident { .. } => Operand::ConstBool(true),
            ast::Pattern::IntLit { value, .. } => Operand::BinOp(
                BinOp::Eq,
                Box::new(scrut.clone()),
                Box::new(Operand::ConstI32(*value as i32)),
            ),
            ast::Pattern::BoolLit { value, .. } => Operand::BinOp(
                BinOp::Eq,
                Box::new(scrut.clone()),
                Box::new(Operand::ConstBool(*value)),
            ),
            ast::Pattern::StringLit { value, .. } => Operand::BinOp(
                BinOp::Eq,
                Box::new(scrut.clone()),
                Box::new(Operand::ConstString(value.clone())),
            ),
            ast::Pattern::Enum { path, variant, .. } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    )
                } else {
                    Operand::ConstBool(false)
                }
            }
            _ => Operand::ConstBool(true),
        }
    }

    /// Build a nested IfExpr from match arms for value-returning match.
    pub(super) fn build_match_if_expr(
        &mut self,
        scrut: &Operand,
        arms: &[ast::MatchArm],
        idx: usize,
    ) -> Operand {
        if idx >= arms.len() {
            return Operand::Unit;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) => {
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    let then_result = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    Operand::IfExpr {
                        cond: Box::new(guard_cond),
                        then_body: vec![],
                        then_result: Some(Box::new(then_result)),
                        else_body: vec![],
                        else_result: Some(Box::new(else_result)),
                    }
                } else {
                    self.lower_expr(&arm.body)
                }
            }
            ast::Pattern::Ident { name, .. } => {
                let local_id = self.declare_local(name);
                let assign_stmt =
                    MirStmt::Assign(Place::Local(local_id), Rvalue::Use(scrut.clone()));
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    let body_val = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    // Outer: assign binding, then inner guard check
                    Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body: vec![assign_stmt],
                        then_result: Some(Box::new(Operand::IfExpr {
                            cond: Box::new(guard_cond),
                            then_body: vec![],
                            then_result: Some(Box::new(body_val)),
                            else_body: vec![],
                            else_result: Some(Box::new(else_result)),
                        })),
                        else_body: vec![],
                        else_result: Some(Box::new(Operand::Unit)),
                    }
                } else {
                    let body_val = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body: vec![assign_stmt],
                        then_result: Some(Box::new(body_val)),
                        else_body: vec![],
                        else_result: Some(Box::new(else_result)),
                    }
                }
            }
            ast::Pattern::IntLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::BoolLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    let payload_strings = if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_local_payload_strings.get(&lid.0).cloned()
                    } else {
                        None
                    };
                    let mut setup_stmts = Vec::new();
                    for (i, field_pat) in fields.iter().enumerate() {
                        if let ast::Pattern::Ident { name: binding, .. } = field_pat {
                            let local_id = self.declare_local(binding);
                            if let Some(ref ps) = payload_strings {
                                if ps.contains(&(variant.clone(), i as u32)) {
                                    self.string_locals.insert(local_id.0);
                                }
                            }
                            if let Some(variants) = self.enum_defs.get(path.as_str()) {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant)
                                {
                                    if let Some(t) = types.get(i) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                        if self.enum_defs.contains_key(t.as_str()) {
                                            self.enum_typed_locals.insert(local_id.0, t.clone());
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: i as u32,
                                enum_name: path.clone(),
                                variant_name: variant.clone(),
                            };
                            setup_stmts.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                    }
                    if let Some(guard) = &arm.guard {
                        let guard_cond = self.lower_expr(guard);
                        let then_result = self.lower_expr(&arm.body);
                        let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                        // Outer: tag check → bind fields → inner guard check
                        Operand::IfExpr {
                            cond: Box::new(cond),
                            then_body: setup_stmts,
                            then_result: Some(Box::new(Operand::IfExpr {
                                cond: Box::new(guard_cond),
                                then_body: vec![],
                                then_result: Some(Box::new(then_result)),
                                else_body: vec![],
                                else_result: Some(Box::new(else_result)),
                            })),
                            else_body: vec![],
                            else_result: Some(Box::new(self.build_match_if_expr(
                                scrut,
                                arms,
                                idx + 1,
                            ))),
                        }
                    } else {
                        let then_result = self.lower_expr(&arm.body);
                        let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                        Operand::IfExpr {
                            cond: Box::new(cond),
                            then_body: setup_stmts,
                            then_result: Some(Box::new(then_result)),
                            else_body: vec![],
                            else_result: Some(Box::new(else_result)),
                        }
                    }
                } else {
                    self.build_match_if_expr(scrut, arms, idx + 1)
                }
            }
            ast::Pattern::Or { patterns, .. } => {
                let mut combined_cond: Option<Operand> = None;
                for pat in patterns {
                    let sub_cond = self.pattern_to_condition(scrut, pat);
                    combined_cond = Some(match combined_cond {
                        Some(prev) => Operand::BinOp(BinOp::Or, Box::new(prev), Box::new(sub_cond)),
                        None => sub_cond,
                    });
                }
                let mut cond = combined_cond.unwrap_or(Operand::ConstBool(false));
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::Struct { name, fields, .. } => {
                // Check if this is an enum struct variant pattern: "EnumName::VariantName"
                if let Some((enum_name, variant_name)) = name.split_once("::") {
                    let key = format!("{}::{}", enum_name, variant_name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let cond = Operand::BinOp(
                            BinOp::Eq,
                            Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                            Box::new(Operand::ConstI32(tag)),
                        );
                        let mut setup_stmts = Vec::new();
                        let def_field_names = self
                            .enum_variant_field_names
                            .get(&key)
                            .cloned()
                            .unwrap_or_default();
                        for (fname, fpat) in fields {
                            let binding_name = match fpat {
                                Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                                None => fname.clone(),
                                _ => fname.clone(),
                            };
                            let local_id = self.declare_local(&binding_name);
                            let field_idx =
                                def_field_names.iter().position(|n| n == fname).unwrap_or(0);
                            if let Some(variants) = self.enum_defs.get(enum_name) {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant_name)
                                {
                                    if let Some(t) = types.get(field_idx) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                        if self.enum_defs.contains_key(t.as_str()) {
                                            self.enum_typed_locals.insert(local_id.0, t.clone());
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: field_idx as u32,
                                enum_name: enum_name.to_string(),
                                variant_name: variant_name.to_string(),
                            };
                            setup_stmts.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                        let then_result = self.lower_expr(&arm.body);
                        let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                        return Operand::IfExpr {
                            cond: Box::new(cond),
                            then_body: setup_stmts,
                            then_result: Some(Box::new(then_result)),
                            else_body: vec![],
                            else_result: Some(Box::new(else_result)),
                        };
                    }
                }
                // Regular struct pattern
                let mut setup_stmts = Vec::new();
                for (fname, fpat) in fields {
                    let binding_name = match fpat {
                        Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                        None => fname.clone(),
                        _ => fname.clone(),
                    };
                    let local_id = self.declare_local(&binding_name);
                    if let Some(sdef) = self.struct_defs.get(name.as_str()) {
                        if let Some((_, ftype)) = sdef.iter().find(|(n, _)| n == fname) {
                            if ftype == "f64" {
                                self.f64_locals.insert(local_id.0);
                            }
                            if ftype == "String" {
                                self.string_locals.insert(local_id.0);
                            }
                        }
                    }
                    let field_access = Operand::FieldAccess {
                        object: Box::new(scrut.clone()),
                        struct_name: name.clone(),
                        field: fname.clone(),
                    };
                    setup_stmts.push(MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(field_access),
                    ));
                }
                let cond = if let Some(guard) = &arm.guard {
                    self.lower_expr(guard)
                } else {
                    Operand::ConstBool(true)
                };
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: setup_stmts,
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            _ => {
                // Skip unsupported patterns
                self.build_match_if_expr(scrut, arms, idx + 1)
            }
        }
    }
}
