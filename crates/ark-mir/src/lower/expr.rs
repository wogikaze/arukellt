//! Expression lowering for MIR.

use std::collections::{HashMap, HashSet};

use ark_parser::ast;

use crate::mir::*;

use super::LowerCtx;
use super::types::is_string_type;
use super::{
    default_function_instance, fallback_block, fallback_function, finalize_function_metadata,
    infer_fn_id,
};

fn lower_binop(op: &ast::BinOp) -> BinOp {
    match op {
        ast::BinOp::Add => BinOp::Add,
        ast::BinOp::Sub => BinOp::Sub,
        ast::BinOp::Mul => BinOp::Mul,
        ast::BinOp::Div => BinOp::Div,
        ast::BinOp::Mod => BinOp::Mod,
        ast::BinOp::Eq => BinOp::Eq,
        ast::BinOp::Ne => BinOp::Ne,
        ast::BinOp::Lt => BinOp::Lt,
        ast::BinOp::Gt => BinOp::Gt,
        ast::BinOp::Le => BinOp::Le,
        ast::BinOp::Ge => BinOp::Ge,
        ast::BinOp::And => BinOp::And,
        ast::BinOp::Or => BinOp::Or,
        ast::BinOp::BitAnd => BinOp::BitAnd,
        ast::BinOp::BitOr => BinOp::BitOr,
        ast::BinOp::BitXor => BinOp::BitXor,
        ast::BinOp::Shl => BinOp::Shl,
        ast::BinOp::Shr => BinOp::Shr,
    }
}

fn lower_unaryop(op: &ast::UnaryOp) -> UnaryOp {
    match op {
        ast::UnaryOp::Neg => UnaryOp::Neg,
        ast::UnaryOp::Not => UnaryOp::Not,
        ast::UnaryOp::BitNot => UnaryOp::BitNot,
    }
}

impl LowerCtx {
    pub(super) fn lower_expr(&mut self, expr: &ast::Expr) -> Operand {
        match expr {
            ast::Expr::StringLit { value, .. } => Operand::ConstString(value.clone()),
            ast::Expr::IntLit { value, suffix, .. } => match suffix.as_deref() {
                Some("u8") => Operand::ConstU8(*value as u8),
                Some("u16") => Operand::ConstU16(*value as u16),
                Some("u32") => Operand::ConstU32(*value as u32),
                Some("u64") => Operand::ConstU64(*value as u64),
                Some("i8") => Operand::ConstI8(*value as i8),
                Some("i16") => Operand::ConstI16(*value as i16),
                Some("i64") => Operand::ConstI64(*value),
                Some("i32") => Operand::ConstI32(*value as i32),
                _ => {
                    if *value > i32::MAX as i64 || *value < i32::MIN as i64 {
                        Operand::ConstI64(*value)
                    } else {
                        Operand::ConstI32(*value as i32)
                    }
                }
            },
            ast::Expr::FloatLit { value, suffix, .. } => match suffix.as_deref() {
                Some("f32") => Operand::ConstF32(*value as f32),
                _ => Operand::ConstF64(*value),
            },
            ast::Expr::BoolLit { value, .. } => Operand::ConstBool(*value),
            ast::Expr::CharLit { value, .. } => Operand::ConstChar(*value),
            ast::Expr::Ident { name, .. } => {
                // Check if this is a bare enum variant (e.g., None)
                if let Some((enum_name, tag, field_count)) = self.bare_variant_tags.get(name)
                    && *field_count == 0
                {
                    return Operand::EnumInit {
                        enum_name: enum_name.clone(),
                        variant: name.clone(),
                        tag: *tag,
                        payload: vec![],
                    };
                }
                if let Some(local_id) = self.lookup_local(name) {
                    Operand::Place(Place::Local(local_id))
                } else if self.user_fn_names.contains(name) {
                    Operand::FnRef(name.clone())
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Binary {
                op,
                left,
                right,
                span,
                ..
            } => {
                // Check for operator overloading (struct + struct → method call)
                if let Some((mangled, _struct_name)) =
                    self.method_resolutions.get(&span.start).cloned()
                {
                    let l = self.lower_expr(left);
                    let r = self.lower_expr(right);
                    let result = Operand::Call(mangled, vec![l, r]);
                    // For Ne, wrap eq result with negation
                    return match op {
                        ast::BinOp::Ne => {
                            Operand::UnaryOp(crate::mir::UnaryOp::Not, Box::new(result))
                        }
                        ast::BinOp::Gt => {
                            // a > b → b.cmp(a) (swap args)
                            // Actually, just return the call result; cmp returns bool
                            result
                        }
                        _ => result,
                    };
                }
                match op {
                    // Short-circuit: a && b  =>  if a { b } else { false }
                    ast::BinOp::And => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::IfExpr {
                            cond: Box::new(l),
                            then_body: vec![],
                            then_result: Some(Box::new(r)),
                            else_body: vec![],
                            else_result: Some(Box::new(Operand::ConstBool(false))),
                        }
                    }
                    // Short-circuit: a || b  =>  if a { true } else { b }
                    ast::BinOp::Or => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::IfExpr {
                            cond: Box::new(l),
                            then_body: vec![],
                            then_result: Some(Box::new(Operand::ConstBool(true))),
                            else_body: vec![],
                            else_result: Some(Box::new(r)),
                        }
                    }
                    _ => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::BinOp(lower_binop(op), Box::new(l), Box::new(r))
                    }
                }
            }
            ast::Expr::Unary { op, operand, .. } => {
                let inner = self.lower_expr(operand);
                Operand::UnaryOp(lower_unaryop(op), Box::new(inner))
            }
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    // Check if this is a bare enum variant constructor (e.g., Some(42), Ok(100))
                    if let Some((enum_name, tag, _field_count)) =
                        self.bare_variant_tags.get(name).cloned()
                    {
                        let payload: Vec<Operand> =
                            args.iter().map(|a| self.lower_expr(a)).collect();
                        return Operand::EnumInit {
                            enum_name,
                            variant: name.clone(),
                            tag,
                            payload,
                        };
                    }
                    // Builtin Option/Result operations
                    match name.as_str() {
                        "unwrap" => {
                            let arg = self.lower_expr(&args[0]);
                            // unwrap: extract payload[0] (tag 0 = Some/Ok)
                            return Operand::EnumPayload {
                                object: Box::new(arg),
                                index: 0,
                                enum_name: "Option".to_string(),
                                variant_name: "Some".to_string(),
                            };
                        }
                        "unwrap_or" => {
                            let arg = self.lower_expr(&args[0]);
                            let default = self.lower_expr(&args[1]);
                            // if is_some(arg) then payload[0] else default
                            let cond = Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg.clone()))),
                                Box::new(Operand::ConstI32(0)), // Some/Ok tag
                            );
                            return Operand::IfExpr {
                                cond: Box::new(cond),
                                then_body: vec![],
                                then_result: Some(Box::new(Operand::EnumPayload {
                                    object: Box::new(arg),
                                    index: 0,
                                    enum_name: "Option".to_string(),
                                    variant_name: "Some".to_string(),
                                })),
                                else_body: vec![],
                                else_result: Some(Box::new(default)),
                            };
                        }
                        "is_some" | "is_ok" => {
                            let arg = self.lower_expr(&args[0]);
                            // Some/Ok tag = 0
                            return Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg))),
                                Box::new(Operand::ConstI32(0)),
                            );
                        }
                        "is_none" | "is_err" => {
                            let arg = self.lower_expr(&args[0]);
                            // None/Err tag != 0 (i.e., tag == 1)
                            return Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg))),
                                Box::new(Operand::ConstI32(1)),
                            );
                        }
                        "to_string" if args.len() == 1 => {
                            // Display trait dispatch: if arg is a struct with Display impl,
                            // rewrite to StructName__to_string(arg)
                            if let Some(struct_name) = self.infer_struct_type(&args[0]) {
                                let mangled = format!("{}__{}", struct_name, "to_string");
                                if self.user_fn_names.contains(&mangled) {
                                    let lowered_arg = self.lower_expr(&args[0]);
                                    return Operand::Call(mangled, vec![lowered_arg]);
                                }
                            }
                        }
                        _ => {}
                    }
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    // Check if callee is a local (function pointer parameter) → indirect call
                    if let Some(local_id) = self.lookup_local(name) {
                        // Check if this is a closure with captures → direct call with injected args
                        if let Some((synth_fn, cap_names)) =
                            self.closure_locals.get(&local_id.0).cloned()
                        {
                            let mut all_args = mir_args;
                            for cap_name in &cap_names {
                                if let Some(cap_lid) = self.lookup_local(cap_name) {
                                    all_args.push(Operand::Place(Place::Local(cap_lid)));
                                } else {
                                    all_args.push(Operand::ConstI32(0));
                                }
                            }
                            Operand::Call(synth_fn, all_args)
                        } else {
                            let callee_op = self.lower_expr(callee);
                            Operand::CallIndirect {
                                callee: Box::new(callee_op),
                                args: mir_args,
                            }
                        }
                    } else {
                        Operand::Call(name.clone(), mir_args)
                    }
                } else if let ast::Expr::QualifiedIdent { module, name, .. } = callee.as_ref() {
                    // Qualified enum variant constructor: Shape::Circle(5.0)
                    let key = format!("{}::{}", module, name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let payload: Vec<Operand> =
                            args.iter().map(|a| self.lower_expr(a)).collect();
                        return Operand::EnumInit {
                            enum_name: module.clone(),
                            variant: name.clone(),
                            tag,
                            payload,
                        };
                    }
                    // Qualified module function call: module::func(args)
                    // Loaded modules are flattened into the merged module, so codegen resolves by item name.
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    Operand::Call(name.clone(), mir_args)
                } else if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    // Method call: x.method(args) → TypeName__method(x, args)
                    if let Some((mangled, _struct_name)) =
                        self.method_resolutions.get(&span.start).cloned()
                    {
                        let self_arg = self.lower_expr(object);
                        let mut all_args = vec![self_arg];
                        all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                        Operand::Call(mangled, all_args)
                    } else {
                        // Fallback: try to infer struct type and look up method
                        if let Some(struct_name) = self.infer_struct_type(object) {
                            let mangled = format!("{}__{}", struct_name, field);
                            if self.user_fn_names.contains(&mangled) {
                                let self_arg = self.lower_expr(object);
                                let mut all_args = vec![self_arg];
                                all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                                return Operand::Call(mangled, all_args);
                            }
                        }
                        Operand::Unit
                    }
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block(then_block);
                let then_tail = then_block.tail_expr.as_ref().map(|e| self.lower_expr(e));
                let else_stmts = else_block
                    .as_ref()
                    .map(|b| self.lower_block(b))
                    .unwrap_or_default();
                let else_tail = else_block
                    .as_ref()
                    .and_then(|b| b.tail_expr.as_ref().map(|e| self.lower_expr(e)));
                Operand::IfExpr {
                    cond: Box::new(c),
                    then_body: then_stmts,
                    then_result: then_tail.map(Box::new),
                    else_body: else_stmts,
                    else_result: else_tail.map(Box::new),
                }
            }
            ast::Expr::Block(block) => {
                if block.stmts.is_empty() {
                    if let Some(tail) = &block.tail_expr {
                        self.lower_expr(tail)
                    } else {
                        Operand::Unit
                    }
                } else {
                    // Lower statements as side effects, then produce the tail value.
                    let mut then_body = Vec::new();
                    for stmt in &block.stmts {
                        self.lower_stmt(stmt, &mut then_body);
                    }
                    let then_result = if let Some(tail) = &block.tail_expr {
                        self.lower_expr(tail)
                    } else {
                        Operand::Unit
                    };
                    Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body,
                        then_result: Some(Box::new(then_result)),
                        else_body: vec![],
                        else_result: Some(Box::new(Operand::Unit)),
                    }
                }
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                let scrut = self.lower_expr(scrutinee);
                self.build_match_if_expr(&scrut, arms, 0)
            }
            ast::Expr::Loop { body, .. } => {
                let result_id = self.declare_local("__loop_result");
                let prev = self.loop_result_local;
                self.loop_result_local = Some(result_id);
                let mut body_stmts = Vec::new();
                for stmt in &body.stmts {
                    self.lower_stmt(stmt, &mut body_stmts);
                }
                if let Some(tail) = &body.tail_expr {
                    self.lower_expr_stmt(tail, &mut body_stmts);
                }
                self.loop_result_local = prev;
                // Emit as a while(true) loop
                let outer = vec![
                    MirStmt::Assign(Place::Local(result_id), Rvalue::Use(Operand::ConstI32(0))),
                    MirStmt::WhileStmt {
                        cond: Operand::ConstBool(true),
                        body: body_stmts,
                    },
                ];
                // Return the stmts as side effects and the result local as value
                // We need a way to emit statements before returning an operand.
                // Use the Block operand approach: lower stmts as a sequence, then return the local
                Operand::LoopExpr {
                    init: Box::new(Operand::ConstI32(0)),
                    body: outer,
                    result: Box::new(Operand::Place(Place::Local(result_id))),
                }
            }
            ast::Expr::QualifiedIdent { module, name, .. } => {
                // Enum variant reference: Direction::South -> EnumInit with no payload
                let key = format!("{}::{}", module, name);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    // Check if this variant has fields
                    let has_fields = self
                        .enum_variants
                        .get(module.as_str())
                        .and_then(|vs| vs.iter().find(|(vn, _)| vn == name))
                        .is_some_and(|(_, fc)| *fc > 0);
                    if has_fields {
                        // Variant with payload but called without args — shouldn't happen
                        Operand::ConstI32(tag)
                    } else {
                        // Unit variant — allocate in memory like other enum variants for consistency
                        Operand::EnumInit {
                            enum_name: module.clone(),
                            variant: name.clone(),
                            tag,
                            payload: vec![],
                        }
                    }
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Tuple { elements, .. } => {
                // Lower tuple as a struct with numbered fields.
                // In generic functions, use __tupleN_any (anyref fields) to hold polymorphic values.
                let tuple_name = if !self.type_params.is_empty() {
                    format!("__tuple{}_any", elements.len())
                } else {
                    format!("__tuple{}", elements.len())
                };
                let lowered_fields: Vec<(String, Operand)> = elements
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (i.to_string(), self.lower_expr(e)))
                    .collect();
                Operand::StructInit {
                    name: tuple_name,
                    fields: lowered_fields,
                }
            }
            ast::Expr::StructInit {
                name, fields, base, ..
            } => {
                // Check if this is an enum struct variant: "EnumName::VariantName"
                if let Some((enum_name, variant_name)) = name.split_once("::") {
                    let key = format!("{}::{}", enum_name, variant_name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let lowered: HashMap<String, Operand> = fields
                            .iter()
                            .map(|(fname, fexpr)| (fname.clone(), self.lower_expr(fexpr)))
                            .collect();
                        // Order payload fields by definition order
                        let def_field_names = self
                            .enum_variant_field_names
                            .get(&key)
                            .cloned()
                            .unwrap_or_default();
                        let payload: Vec<Operand> = def_field_names
                            .iter()
                            .map(|fname| {
                                lowered.get(fname).cloned().unwrap_or(Operand::ConstI32(0))
                            })
                            .collect();
                        return Operand::EnumInit {
                            enum_name: enum_name.to_string(),
                            variant: variant_name.to_string(),
                            tag,
                            payload,
                        };
                    }
                }
                let mut lowered_fields: Vec<(String, Operand)> = fields
                    .iter()
                    .map(|(fname, fexpr)| (fname.clone(), self.lower_expr(fexpr)))
                    .collect();
                // Handle struct field update: fill missing fields from base
                if let Some(base_expr) = base {
                    let base_op = self.lower_expr(base_expr);
                    if let Some(sdef) = self.struct_defs.get(name.as_str()).cloned() {
                        let explicit: std::collections::HashSet<String> =
                            lowered_fields.iter().map(|(n, _)| n.clone()).collect();
                        for (fname, _) in &sdef {
                            if !explicit.contains(fname) {
                                lowered_fields.push((
                                    fname.clone(),
                                    Operand::FieldAccess {
                                        object: Box::new(base_op.clone()),
                                        struct_name: name.clone(),
                                        field: fname.clone(),
                                    },
                                ));
                            }
                        }
                    }
                }
                Operand::StructInit {
                    name: name.clone(),
                    fields: lowered_fields,
                }
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                // Try to determine the struct type from the object
                let struct_name = self.infer_struct_type(object);
                let obj = self.lower_expr(object);
                Operand::FieldAccess {
                    object: Box::new(obj),
                    struct_name: struct_name.unwrap_or_default(),
                    field: field.clone(),
                }
            }
            ast::Expr::Try { expr, span } => {
                let inner = self.lower_expr(expr);
                // Check if the typechecker recorded a From conversion for this ?
                let from_fn = self
                    .method_resolutions
                    .get(&span.start)
                    .map(|(f, _)| f.clone());
                Operand::TryExpr {
                    expr: Box::new(inner),
                    from_fn,
                }
            }
            ast::Expr::Closure {
                params,
                body,
                return_type,
                ..
            } => {
                // Lambda-lift: create a synthetic function
                let synth_name = format!("__closure_{}", self.closure_counter);
                self.closure_counter += 1;

                // Identify free variables (captured from enclosing scope)
                let param_names: HashSet<&str> = params.iter().map(|p| p.name.as_str()).collect();
                let mut captures: Vec<String> = Vec::new();
                self.collect_free_vars(body, &param_names, &mut captures);
                captures.dedup();

                // Build params for the synthetic function: closure params first, then captures
                let mut mir_params: Vec<MirLocal> = Vec::new();
                let mut param_idx = 0u32;
                for p in params {
                    let ty = match &p.ty {
                        Some(te) if is_string_type(te) => ark_typecheck::types::Type::String,
                        Some(ast::TypeExpr::Named { name, .. }) if name == "i64" => {
                            ark_typecheck::types::Type::I64
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "f64" => {
                            ark_typecheck::types::Type::F64
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "bool" => {
                            ark_typecheck::types::Type::Bool
                        }
                        _ => ark_typecheck::types::Type::I32,
                    };
                    mir_params.push(MirLocal {
                        id: LocalId(param_idx),
                        name: Some(p.name.clone()),
                        ty,
                    });
                    param_idx += 1;
                }
                for cap in &captures {
                    let ty = if let Some(lid) = self.lookup_local(cap) {
                        if self.string_locals.contains(&lid.0) {
                            ark_typecheck::types::Type::String
                        } else if self.f64_locals.contains(&lid.0) {
                            ark_typecheck::types::Type::F64
                        } else if self.i64_locals.contains(&lid.0) {
                            ark_typecheck::types::Type::I64
                        } else {
                            ark_typecheck::types::Type::I32
                        }
                    } else {
                        ark_typecheck::types::Type::I32
                    };
                    mir_params.push(MirLocal {
                        id: LocalId(param_idx),
                        name: Some(cap.clone()),
                        ty,
                    });
                    param_idx += 1;
                }

                // Lower closure body in a fresh sub-context
                let mut sub_ctx = LowerCtx::new(
                    self.enum_tags.clone(),
                    self.struct_defs.clone(),
                    self.enum_variants.clone(),
                    self.variant_to_enum.clone(),
                    self.bare_variant_tags.clone(),
                    self.enum_defs.clone(),
                    self.enum_variant_field_names.clone(),
                    self.fn_return_types.clone(),
                    self.user_fn_names.clone(),
                    self.method_resolutions.clone(),
                    vec![], // closures don't have type params
                    self.generic_fn_names.clone(),
                    self.vec_struct_fields.clone(),
                );
                for p in &mir_params {
                    let lid = sub_ctx.declare_local(p.name.as_deref().unwrap_or("_"));
                    match &p.ty {
                        ark_typecheck::types::Type::String => {
                            sub_ctx.string_locals.insert(lid.0);
                        }
                        ark_typecheck::types::Type::F64 => {
                            sub_ctx.f64_locals.insert(lid.0);
                        }
                        ark_typecheck::types::Type::I64 => {
                            sub_ctx.i64_locals.insert(lid.0);
                        }
                        ark_typecheck::types::Type::Bool => {
                            sub_ctx.bool_locals.insert(lid.0);
                        }
                        _ => {
                            // Propagate struct type info for captured variables
                            if let Some(pname) = &p.name
                                && let Some(parent_lid) = self.lookup_local(pname)
                                && let Some(sname) = self.struct_typed_locals.get(&parent_lid.0)
                            {
                                sub_ctx.struct_typed_locals.insert(lid.0, sname.clone());
                            }
                        }
                    }
                }

                // Lower body
                let (body_stmts, tail_op) = match body.as_ref() {
                    ast::Expr::Block(block) => {
                        let stmts = sub_ctx.lower_block(block);
                        let tail = block.tail_expr.as_ref().map(|e| sub_ctx.lower_expr(e));
                        (stmts, tail)
                    }
                    other => {
                        let op = sub_ctx.lower_expr(other);
                        (vec![], Some(op))
                    }
                };

                let return_ty = if let Some(rt) = return_type {
                    if is_string_type(rt) {
                        ark_typecheck::types::Type::String
                    } else {
                        match rt {
                            ast::TypeExpr::Named { name, .. } if name == "i64" => {
                                ark_typecheck::types::Type::I64
                            }
                            ast::TypeExpr::Named { name, .. } if name == "f64" => {
                                ark_typecheck::types::Type::F64
                            }
                            ast::TypeExpr::Named { name, .. } if name == "bool" => {
                                ark_typecheck::types::Type::Bool
                            }
                            _ => ark_typecheck::types::Type::I32,
                        }
                    }
                } else if let Some(ref op) = tail_op {
                    if sub_ctx.is_string_operand_mir(op) {
                        ark_typecheck::types::Type::String
                    } else if sub_ctx.is_f64_operand_mir(op) {
                        ark_typecheck::types::Type::F64
                    } else if sub_ctx.is_i64_operand_mir(op) {
                        ark_typecheck::types::Type::I64
                    } else {
                        ark_typecheck::types::Type::I32
                    }
                } else {
                    ark_typecheck::types::Type::I32
                };
                let num_locals = sub_ctx.next_local;
                let entry = BlockId(0);
                let locals: Vec<MirLocal> = (0..num_locals)
                    .map(|i| {
                        let ty = if sub_ctx.string_locals.contains(&i) {
                            ark_typecheck::types::Type::String
                        } else if sub_ctx.f64_locals.contains(&i) {
                            ark_typecheck::types::Type::F64
                        } else if sub_ctx.i64_locals.contains(&i) {
                            ark_typecheck::types::Type::I64
                        } else {
                            ark_typecheck::types::Type::I32
                        };
                        MirLocal {
                            id: LocalId(i),
                            name: Some(format!("_l{}", i)),
                            ty,
                        }
                    })
                    .collect();
                let mir_fn = fallback_function(
                    FnId(0), // will be reassigned in lower_module
                    synth_name.clone(),
                    mir_params,
                    return_ty,
                    locals,
                    vec![fallback_block(
                        entry,
                        body_stmts,
                        if let Some(op) = tail_op {
                            Terminator::Return(Some(op))
                        } else {
                            Terminator::Return(None)
                        },
                    )],
                    entry,
                    sub_ctx.struct_typed_locals.clone(),
                    sub_ctx.enum_typed_locals.clone(),
                    vec![], // closures are not generic
                    false,  // closures are not component exports
                );

                let mir_fn = MirFunction {
                    id: infer_fn_id(&synth_name, self.closure_counter),
                    ..mir_fn
                };

                let mir_fn = MirFunction {
                    instance: default_function_instance(&synth_name),
                    ..mir_fn
                };

                let mut mir_fn = mir_fn;
                finalize_function_metadata(&mut mir_fn);
                self.pending_closures.push(mir_fn);
                self.user_fn_names.insert(synth_name.clone());

                // Store captures for call-site injection
                if !captures.is_empty() {
                    self.closure_fn_captures
                        .insert(synth_name.clone(), captures);
                }

                Operand::FnRef(synth_name)
            }
            ast::Expr::Array { elements, .. } => {
                let lowered: Vec<Operand> = elements.iter().map(|e| self.lower_expr(e)).collect();
                Operand::ArrayInit { elements: lowered }
            }
            ast::Expr::ArrayRepeat { value, count, .. } => {
                let val = self.lower_expr(value);
                if let ast::Expr::IntLit { value: n, .. } = count.as_ref() {
                    let n = *n as usize;
                    let elements: Vec<Operand> = (0..n).map(|_| val.clone()).collect();
                    Operand::ArrayInit { elements }
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Index { object, index, .. } => {
                let obj = self.lower_expr(object);
                let idx = self.lower_expr(index);
                Operand::IndexAccess {
                    object: Box::new(obj),
                    index: Box::new(idx),
                }
            }
            other => {
                eprintln!(
                    "ICE: unhandled expression in lower_expr: {:?}",
                    std::mem::discriminant(other)
                );
                Operand::Unit
            }
        }
    }
}
