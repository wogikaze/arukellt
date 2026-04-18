//! Value-returning match lowering (nested IfExpr) for MIR.

use ark_parser::ast;

use crate::mir::*;

use super::LowerCtx;

impl LowerCtx {
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
                            if let Some(ref ps) = payload_strings
                                && ps.contains(&(variant.clone(), i as u32))
                            {
                                self.string_locals.insert(local_id.0);
                            }
                            if let Some(variants) = self.enum_defs.get(path.as_str())
                                && let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant)
                                && let Some(t) = types.get(i)
                            {
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
                            if let Some(variants) = self.enum_defs.get(enum_name)
                                && let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant_name)
                                && let Some(t) = types.get(field_idx)
                            {
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
                    if let Some(sdef) = self.struct_defs.get(name.as_str())
                        && let Some((_, ftype)) = sdef.iter().find(|(n, _)| n == fname)
                    {
                        if ftype == "f64" {
                            self.f64_locals.insert(local_id.0);
                        }
                        if ftype == "String" {
                            self.string_locals.insert(local_id.0);
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
