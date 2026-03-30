//! Statement lowering for MIR.

use std::collections::HashSet;

use ark_parser::ast;

use crate::mir::*;

use super::LowerCtx;
use super::types::{detect_specialized_result, is_string_type};

impl LowerCtx {
    pub(super) fn lower_block(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            self.lower_stmt(stmt, &mut stmts);
        }
        stmts
    }

    /// Lower a block including its tail expression as a statement.
    pub(super) fn lower_block_all(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = self.lower_block(block);
        if let Some(tail) = &block.tail_expr {
            self.lower_expr_stmt(tail, &mut stmts);
        }
        stmts
    }

    pub(super) fn lower_stmt(&mut self, stmt: &ast::Stmt, out: &mut Vec<MirStmt>) {
        match stmt {
            ast::Stmt::Let {
                name,
                init,
                ty,
                pattern,
                ..
            } => {
                // Handle tuple destructuring: let (a, b) = expr
                if let Some(ast::Pattern::Tuple { elements, .. }) = pattern {
                    // Determine if the init is a call to a generic function
                    let callee_is_generic = self.is_generic_call(init);
                    let tuple_name = if callee_is_generic {
                        format!("__tuple{}_any", elements.len())
                    } else {
                        format!("__tuple{}", elements.len())
                    };
                    let local_id = self.declare_local(name);
                    self.struct_typed_locals
                        .insert(local_id.0, tuple_name.clone());
                    let op = self.lower_expr(init);
                    out.push(MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(op.clone()),
                    ));
                    // Detect which tuple elements are strings from the init expression
                    let string_element_indices =
                        self.detect_string_tuple_elements(init, &op, elements.len());
                    // Destructure each element
                    for (i, elem) in elements.iter().enumerate() {
                        if let ast::Pattern::Ident {
                            name: elem_name, ..
                        } = elem
                        {
                            let elem_id = self.declare_local(elem_name);
                            if string_element_indices.contains(&i) {
                                self.string_locals.insert(elem_id.0);
                            }
                            let access = Operand::FieldAccess {
                                object: Box::new(Operand::Place(Place::Local(local_id))),
                                struct_name: tuple_name.clone(),
                                field: i.to_string(),
                            };
                            out.push(MirStmt::Assign(Place::Local(elem_id), Rvalue::Use(access)));
                        }
                    }
                    return;
                }
                if name == "_" {
                    // Wildcard binding: evaluate for side effects only
                    self.lower_expr_stmt(init, out);
                    return;
                }
                // Evaluate init BEFORE declaring the local so that shadowed
                // names (e.g. `let list = prepend(list, 3)`) resolve to the
                // previous binding.
                let op = self.lower_expr(init);
                let local_id = self.declare_local(name);
                if let Some(type_expr) = ty {
                    if is_string_type(type_expr) {
                        self.string_locals.insert(local_id.0);
                    }
                    // Track f64-typed locals
                    if let ast::TypeExpr::Named { name: tname, .. } = type_expr {
                        if tname == "f64" {
                            self.f64_locals.insert(local_id.0);
                        }
                        if tname == "i64" || tname == "u64" {
                            self.i64_locals.insert(local_id.0);
                        }
                        if tname == "bool" {
                            self.bool_locals.insert(local_id.0);
                        }
                        if tname == "char" {
                            self.char_locals.insert(local_id.0);
                        }
                    }
                    // Track struct-typed locals
                    if let ast::TypeExpr::Named { name: tname, .. } = type_expr {
                        if self.struct_defs.contains_key(tname.as_str()) {
                            self.struct_typed_locals.insert(local_id.0, tname.clone());
                        }
                        if self.enum_variants.contains_key(tname.as_str()) {
                            self.enum_typed_locals.insert(local_id.0, tname.clone());
                        }
                    }
                    // Track generic enum types: Option<i32>, Result<i32, String>
                    if let ast::TypeExpr::Generic {
                        name: tname, args, ..
                    } = type_expr
                    {
                        if tname == "Vec" && args.first().is_some_and(is_string_type) {
                            self.vec_string_locals.insert(local_id.0);
                        }
                        if tname == "Vec"
                            && let Some(ast::TypeExpr::Named { name: inner, .. }) = args.first()
                        {
                            if inner == "i64" {
                                self.vec_i64_locals.insert(local_id.0);
                            } else if inner == "f64" {
                                self.vec_f64_locals.insert(local_id.0);
                            } else if inner == "i32" {
                                self.vec_i32_locals.insert(local_id.0);
                            } else if self.struct_defs.contains_key(inner.as_str()) {
                                // Vec<StructName> — track for get_unchecked result type inference
                                self.vec_struct_locals.insert(local_id.0, inner.clone());
                            }
                        }
                        if self.enum_variants.contains_key(tname.as_str()) {
                            self.enum_typed_locals.insert(local_id.0, tname.clone());
                            // Map generic args to variant payload types
                            // For Option<T>: Some has payload 0 = T
                            // For Result<T, E>: Ok has payload 0 = T, Err has payload 0 = E
                            let mut payload_strings = HashSet::new();
                            if tname == "Option" && args.first().is_some_and(is_string_type) {
                                payload_strings.insert(("Some".to_string(), 0u32));
                            } else if tname == "Result" {
                                if args.first().is_some_and(is_string_type) {
                                    payload_strings.insert(("Ok".to_string(), 0u32));
                                }
                                if args.get(1).is_some_and(is_string_type) {
                                    payload_strings.insert(("Err".to_string(), 0u32));
                                }
                            }
                            if !payload_strings.is_empty() {
                                self.enum_local_payload_strings
                                    .insert(local_id.0, payload_strings);
                            }
                            // Track specialized enum types for i64/f64 payloads
                            if let Some(spec) = detect_specialized_result(type_expr) {
                                self.enum_local_specialized.insert(local_id.0, spec);
                            }
                        }
                    }
                }
                // Infer f64 from initializer when there's no explicit type annotation
                if !self.f64_locals.contains(&local_id.0) && self.is_f64_operand_mir(&op) {
                    self.f64_locals.insert(local_id.0);
                }
                // Infer i64 from initializer when there's no explicit type annotation
                if !self.i64_locals.contains(&local_id.0) && self.is_i64_operand_mir(&op) {
                    self.i64_locals.insert(local_id.0);
                }
                // Infer String from initializer when there's no explicit type annotation
                if !self.string_locals.contains(&local_id.0) && self.is_string_operand_mir(&op) {
                    self.string_locals.insert(local_id.0);
                }
                if !self.char_locals.contains(&local_id.0) && matches!(op, Operand::ConstChar(_)) {
                    self.char_locals.insert(local_id.0);
                }
                // Infer struct type from StructInit initializer when there's no type annotation
                #[allow(clippy::map_entry)]
                if !self.struct_typed_locals.contains_key(&local_id.0)
                    && let Some(sname) = self.infer_struct_from_init(init)
                {
                    self.struct_typed_locals.insert(local_id.0, sname);
                }
                // Infer Vec<Struct> elem type from Vec_new_* init (no annotation case)
                if !self.vec_struct_locals.contains_key(&local_id.0)
                    && let ast::Expr::Call { callee, .. } = init
                    && let ast::Expr::Ident {
                        name: callee_name, ..
                    } = callee.as_ref()
                    && let Some(sname) = callee_name.strip_prefix("Vec_new_")
                    && self.struct_defs.contains_key(sname)
                {
                    self.vec_struct_locals.insert(local_id.0, sname.to_string());
                }
                // Infer Vec<Struct> from function call return type when no annotation
                // e.g. `let tokens = lexer::tokenize(src)` where tokenize -> Vec<Token>
                if !self.vec_struct_locals.contains_key(&local_id.0)
                    && let ast::Expr::Call { callee, .. } = init
                {
                    let fn_name = match callee.as_ref() {
                        ast::Expr::Ident { name, .. } => Some(name.as_str()),
                        ast::Expr::QualifiedIdent { name, .. } => Some(name.as_str()),
                        _ => None,
                    };
                    if let Some(fname) = fn_name
                        && let Some(ast::TypeExpr::Generic {
                            name: vec_name,
                            args,
                            ..
                        }) = self.fn_return_types.get(fname)
                        && vec_name == "Vec"
                        && let Some(ast::TypeExpr::Named {
                            name: inner_type, ..
                        }) = args.first()
                        && self.struct_defs.contains_key(inner_type.as_str())
                    {
                        self.vec_struct_locals
                            .insert(local_id.0, inner_type.clone());
                    }
                }
                // Infer struct type from get_unchecked(vec, i) where vec is a Vec<Struct>
                // Case 1: vec is a local variable tracked in vec_struct_locals
                if !self.struct_typed_locals.contains_key(&local_id.0)
                    && let ast::Expr::Call {
                        callee,
                        args: call_args,
                        ..
                    } = init
                    && let ast::Expr::Ident {
                        name: callee_name, ..
                    } = callee.as_ref()
                    && (callee_name == "get_unchecked" || callee_name == "get")
                    && let Some(first_arg) = call_args.first()
                    && let ast::Expr::Ident { name: arg_name, .. } = first_arg
                    && let Some(vec_local_id) = self.lookup_local(arg_name)
                    && let Some(sname) = self.vec_struct_locals.get(&vec_local_id.0).cloned()
                {
                    self.struct_typed_locals.insert(local_id.0, sname);
                }
                // Case 2: vec is a struct field access (e.g. get_unchecked(mir.functions, i))
                if !self.struct_typed_locals.contains_key(&local_id.0)
                    && let ast::Expr::Call {
                        callee,
                        args: call_args,
                        ..
                    } = init
                    && let ast::Expr::Ident {
                        name: callee_name, ..
                    } = callee.as_ref()
                    && (callee_name == "get_unchecked" || callee_name == "get")
                    && let Some(first_arg) = call_args.first()
                    && let ast::Expr::FieldAccess { object, field, .. } = first_arg
                    && let Some(parent_struct) = self.infer_struct_type(object)
                    && let Some(sname) = self
                        .vec_struct_fields
                        .get(&(parent_struct, field.clone()))
                        .cloned()
                {
                    self.struct_typed_locals.insert(local_id.0, sname);
                }
                // Infer enum type from call return type when there's no explicit annotation
                #[allow(clippy::map_entry)]
                if !self.enum_typed_locals.contains_key(&local_id.0)
                    && let Some(ret_te) = self.infer_return_type_expr(init)
                {
                    let is_result =
                        matches!(&ret_te, ast::TypeExpr::Generic { name, .. } if name == "Result");
                    let is_option =
                        matches!(&ret_te, ast::TypeExpr::Generic { name, .. } if name == "Option");
                    if is_result || is_option {
                        let enum_name = if is_result { "Result" } else { "Option" };
                        self.enum_typed_locals
                            .insert(local_id.0, enum_name.to_string());
                        // Compute payload strings for the inferred type
                        let mut payload_strings = HashSet::new();
                        if let ast::TypeExpr::Generic { args, .. } = &ret_te {
                            if enum_name == "Option" {
                                if args.first().is_some_and(is_string_type) {
                                    payload_strings.insert(("Some".to_string(), 0u32));
                                }
                            } else if enum_name == "Result" {
                                if args.first().is_some_and(is_string_type) {
                                    payload_strings.insert(("Ok".to_string(), 0u32));
                                }
                                if args.get(1).is_some_and(is_string_type) {
                                    payload_strings.insert(("Err".to_string(), 0u32));
                                }
                            }
                        }
                        if !payload_strings.is_empty() {
                            self.enum_local_payload_strings
                                .insert(local_id.0, payload_strings);
                        }
                        if let Some(spec) = detect_specialized_result(&ret_te) {
                            self.enum_local_specialized.insert(local_id.0, spec);
                        }
                    }
                    // Infer Vec type from return type expr
                    if let ast::TypeExpr::Generic {
                        name: tname, args, ..
                    } = &ret_te
                        && tname == "Vec"
                    {
                        if args.first().is_some_and(is_string_type) {
                            self.vec_string_locals.insert(local_id.0);
                        } else if let Some(ast::TypeExpr::Named { name: inner, .. }) = args.first()
                        {
                            if inner == "i64" {
                                self.vec_i64_locals.insert(local_id.0);
                            } else if inner == "f64" {
                                self.vec_f64_locals.insert(local_id.0);
                            } else {
                                self.vec_i32_locals.insert(local_id.0);
                            }
                        }
                    }
                }
                // Track closure locals: if the init expression was a closure, record captures
                if let Operand::FnRef(ref fn_name) = op
                    && let Some(caps) = self.closure_fn_captures.get(fn_name).cloned()
                {
                    self.closure_locals
                        .insert(local_id.0, (fn_name.clone(), caps));
                }
                // Promote integer literals to i64 when type annotation is i64
                let op = if self.i64_locals.contains(&local_id.0) {
                    match op {
                        Operand::ConstI32(v) => Operand::ConstI64(v as i64),
                        other => other,
                    }
                } else {
                    op
                };
                out.push(MirStmt::Assign(Place::Local(local_id), Rvalue::Use(op)));
            }
            ast::Stmt::Expr(expr) => {
                self.lower_expr_stmt(expr, out);
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond_op = self.lower_expr(cond);
                out.push(MirStmt::WhileStmt {
                    cond: cond_op,
                    body: self.lower_block_all(body),
                });
            }
            ast::Stmt::Loop { body, .. } => {
                out.push(MirStmt::WhileStmt {
                    cond: Operand::ConstBool(true),
                    body: self.lower_block_all(body),
                });
            }
            ast::Stmt::For {
                target, iter, body, ..
            } => {
                // Desugar for to while
                match iter {
                    ast::ForIter::Range { start, end } => {
                        // for i in start..end { body }
                        // → let mut __i = start; while __i < end { let i = __i; body; __i = __i + 1; }
                        let start_op = self.lower_expr(start);
                        let end_op = self.lower_expr(end);

                        let idx_local = self.declare_local(target);

                        // Assign start value
                        out.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(start_op),
                        ));

                        // Build cond: idx < end
                        let end_local = self.new_temp();
                        out.push(MirStmt::Assign(
                            Place::Local(end_local),
                            Rvalue::Use(end_op),
                        ));

                        let cond_local = self.new_temp();

                        // Build body: original body + increment
                        let mut while_body = self.lower_block_all(body);

                        // idx = idx + 1
                        let inc_tmp = self.new_temp();
                        while_body.push(MirStmt::Assign(
                            Place::Local(inc_tmp),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Add,
                                Box::new(Operand::Place(Place::Local(idx_local))),
                                Box::new(Operand::ConstI32(1)),
                            )),
                        ));
                        while_body.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(Operand::Place(Place::Local(inc_tmp))),
                        ));

                        // cond = idx < end
                        let mut full_body = vec![MirStmt::Assign(
                            Place::Local(cond_local),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Lt,
                                Box::new(Operand::Place(Place::Local(idx_local))),
                                Box::new(Operand::Place(Place::Local(end_local))),
                            )),
                        )];

                        out.push(MirStmt::WhileStmt {
                            cond: Operand::ConstBool(true),
                            body: {
                                full_body.push(MirStmt::IfStmt {
                                    cond: Operand::Place(Place::Local(cond_local)),
                                    then_body: while_body,
                                    else_body: vec![MirStmt::Break],
                                });
                                full_body
                            },
                        });
                    }
                    ast::ForIter::Values(vec_expr) => {
                        // for x in values(v) { body }
                        // → let mut __i = 0; while __i < len(v) { let x = get(v, __i); body; __i = __i + 1; }
                        let vec_op = self.lower_expr(vec_expr);

                        let idx_local = self.new_temp();
                        let vec_local = self.new_temp();
                        let target_local = self.declare_local(target);

                        // __i = 0
                        out.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(Operand::ConstI32(0)),
                        ));
                        // __vec = vec_expr
                        out.push(MirStmt::Assign(
                            Place::Local(vec_local),
                            Rvalue::Use(vec_op),
                        ));

                        // Build loop body
                        let len_tmp = self.new_temp();
                        let cond_tmp = self.new_temp();

                        // cond: __i < len(__vec)
                        let mut loop_body = vec![
                            MirStmt::Assign(
                                Place::Local(len_tmp),
                                Rvalue::Use(Operand::Call(
                                    "len".to_string(),
                                    vec![Operand::Place(Place::Local(vec_local))],
                                )),
                            ),
                            MirStmt::Assign(
                                Place::Local(cond_tmp),
                                Rvalue::Use(Operand::BinOp(
                                    BinOp::Lt,
                                    Box::new(Operand::Place(Place::Local(idx_local))),
                                    Box::new(Operand::Place(Place::Local(len_tmp))),
                                )),
                            ),
                        ];

                        // x = get_unchecked(__vec, __i) — safe because __i < len(__vec) is checked
                        let mut inner_body = vec![MirStmt::Assign(
                            Place::Local(target_local),
                            Rvalue::Use(Operand::Call(
                                "get_unchecked".to_string(),
                                vec![
                                    Operand::Place(Place::Local(vec_local)),
                                    Operand::Place(Place::Local(idx_local)),
                                ],
                            )),
                        )];

                        // original body
                        inner_body.extend(self.lower_block_all(body));

                        // __i = __i + 1
                        let inc_tmp = self.new_temp();
                        inner_body.push(MirStmt::Assign(
                            Place::Local(inc_tmp),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Add,
                                Box::new(Operand::Place(Place::Local(idx_local))),
                                Box::new(Operand::ConstI32(1)),
                            )),
                        ));
                        inner_body.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(Operand::Place(Place::Local(inc_tmp))),
                        ));

                        loop_body.push(MirStmt::IfStmt {
                            cond: Operand::Place(Place::Local(cond_tmp)),
                            then_body: inner_body,
                            else_body: vec![MirStmt::Break],
                        });

                        out.push(MirStmt::WhileStmt {
                            cond: Operand::ConstBool(true),
                            body: loop_body,
                        });
                    }
                    ast::ForIter::Iter(iter_expr) => {
                        // for x in iter_expr { body }
                        // → let __iter = iter_expr
                        //   loop {
                        //     let __next = StructName__next(__iter)
                        //     // __next is Option<T>: [tag(4)][payload(4)]
                        //     // tag==0 → Some(x): let x = payload; body
                        //     // tag==1 → None: break
                        //   }
                        let struct_name = self.infer_struct_type(iter_expr);
                        let iter_op = self.lower_expr(iter_expr);

                        let iter_local = self.new_temp();
                        let next_local = self.new_temp();
                        let tag_local = self.new_temp();
                        let target_local = self.declare_local(target);

                        // Track struct type for the iterator local
                        if let Some(ref sname) = struct_name {
                            self.struct_typed_locals.insert(iter_local.0, sname.clone());
                        }
                        // next_local holds Option<T> enum ref from __next() call
                        self.enum_typed_locals
                            .insert(next_local.0, "Option".to_string());

                        // __iter = iter_expr
                        out.push(MirStmt::Assign(
                            Place::Local(iter_local),
                            Rvalue::Use(iter_op),
                        ));

                        let method_name = if let Some(ref sname) = struct_name {
                            format!("{}__next", sname)
                        } else {
                            "__next".to_string()
                        };

                        // Build loop body:
                        // __next = StructName__next(__iter)
                        let mut loop_body = vec![MirStmt::Assign(
                            Place::Local(next_local),
                            Rvalue::Use(Operand::Call(
                                method_name,
                                vec![Operand::Place(Place::Local(iter_local))],
                            )),
                        )];

                        // tag = __next.tag (EnumTag)
                        loop_body.push(MirStmt::Assign(
                            Place::Local(tag_local),
                            Rvalue::Use(Operand::EnumTag(Box::new(Operand::Place(Place::Local(
                                next_local,
                            ))))),
                        ));

                        // Build inner body: extract payload and run user body
                        let mut some_body = vec![MirStmt::Assign(
                            Place::Local(target_local),
                            Rvalue::Use(Operand::EnumPayload {
                                object: Box::new(Operand::Place(Place::Local(next_local))),
                                index: 0,
                                enum_name: "Option".to_string(),
                                variant_name: "Some".to_string(),
                            }),
                        )];
                        some_body.extend(self.lower_block_all(body));

                        // if tag == 0 (Some) → some_body; else → break
                        let cond_local = self.new_temp();
                        loop_body.push(MirStmt::Assign(
                            Place::Local(cond_local),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::Place(Place::Local(tag_local))),
                                Box::new(Operand::ConstI32(0)),
                            )),
                        ));
                        loop_body.push(MirStmt::IfStmt {
                            cond: Operand::Place(Place::Local(cond_local)),
                            then_body: some_body,
                            else_body: vec![MirStmt::Break],
                        });

                        out.push(MirStmt::WhileStmt {
                            cond: Operand::ConstBool(true),
                            body: loop_body,
                        });
                    }
                }
            }
        }
    }

    pub(super) fn lower_expr_stmt(&mut self, expr: &ast::Expr, out: &mut Vec<MirStmt>) {
        match expr {
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    out.push(MirStmt::CallBuiltin {
                        dest: None,
                        name: name.clone(),
                        args: mir_args,
                    });
                } else if let ast::Expr::QualifiedIdent { name, .. } = callee.as_ref() {
                    // Module-qualified void calls: io::writeln_stdout, etc.
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    out.push(MirStmt::CallBuiltin {
                        dest: None,
                        name: name.clone(),
                        args: mir_args,
                    });
                } else if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    // Method call as statement: x.method(args) → discard result
                    if let Some((mangled, _)) = self.method_resolutions.get(&span.start).cloned() {
                        let self_arg = self.lower_expr(object);
                        let mut all_args = vec![self_arg];
                        all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                        out.push(MirStmt::CallBuiltin {
                            dest: None,
                            name: mangled,
                            args: all_args,
                        });
                    } else if let Some(struct_name) = self.infer_struct_type(object) {
                        let mangled = format!("{}__{}", struct_name, field);
                        if self.user_fn_names.contains(&mangled) {
                            let self_arg = self.lower_expr(object);
                            let mut all_args = vec![self_arg];
                            all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                            out.push(MirStmt::CallBuiltin {
                                dest: None,
                                name: mangled,
                                args: all_args,
                            });
                        }
                    }
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                if let ast::Expr::Ident { name, .. } = target.as_ref() {
                    if let Some(local_id) = self.lookup_local(name) {
                        let op = self.lower_expr(value);
                        out.push(MirStmt::Assign(Place::Local(local_id), Rvalue::Use(op)));
                    }
                } else if let ast::Expr::FieldAccess { object, field, .. } = target.as_ref() {
                    // self.field = value → FieldStore
                    if let ast::Expr::Ident { name, .. } = object.as_ref()
                        && let Some(local_id) = self.lookup_local(name)
                    {
                        let struct_name = self.struct_typed_locals.get(&local_id.0).cloned();
                        let val_op = self.lower_expr(value);
                        out.push(MirStmt::Assign(
                            Place::Field(Box::new(Place::Local(local_id)), field.clone()),
                            Rvalue::Use(val_op),
                        ));
                        // Track struct type for the field access
                        if let Some(sname) = struct_name {
                            // No-op: struct type already tracked
                            let _ = sname;
                        }
                    }
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block_all(then_block);
                let else_stmts = else_block
                    .as_ref()
                    .map(|b| self.lower_block_all(b))
                    .unwrap_or_default();
                out.push(MirStmt::IfStmt {
                    cond: c,
                    then_body: then_stmts,
                    else_body: else_stmts,
                });
            }
            ast::Expr::Break { value, .. } => {
                if let Some(val) = value
                    && let Some(result_id) = self.loop_result_local
                {
                    let op = self.lower_expr(val);
                    out.push(MirStmt::Assign(Place::Local(result_id), Rvalue::Use(op)));
                }
                out.push(MirStmt::Break);
            }
            ast::Expr::Continue { .. } => {
                out.push(MirStmt::Continue);
            }
            ast::Expr::Return { value, .. } => {
                let op = value.as_ref().map(|v| self.lower_expr(v));
                out.push(MirStmt::Return(op));
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                self.lower_match_stmt(scrutinee, arms, out);
            }
            ast::Expr::Block(block) => {
                out.extend(self.lower_block(block));
                if let Some(tail) = &block.tail_expr {
                    self.lower_expr_stmt(tail, out);
                }
            }
            _ => {}
        }
    }
}
