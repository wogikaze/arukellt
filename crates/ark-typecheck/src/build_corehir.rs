use std::collections::HashMap;

use ark_diagnostics::Span;
use ark_hir::{
    Body, BodyId, BuiltinBinaryOp, BuiltinUnaryOp, CallArg, CallTarget, CaptureInfo, ConstValue,
    EffectFlags, Expr, ExprId as HirExprId, ExprKind, FieldDef, FunctionItem, ImplItem, ImportRef,
    Item, ItemId, ItemKind, LetStmt, Local, LocalId, MatchArm, Module, ModuleId, Param, Pattern,
    PatternId, PatternKind, Program, ProgramId, SourceMap, Stmt, StructItem, TraitItem,
    TraitMethodSig, Ty, ValueMode, VariantDef,
};
use ark_parser::ast;

use crate::checker::{TypeChecker, TypeEnv};
use crate::selection::{make_selection, selection_kind_for_method_name};
use crate::types::Type;

#[derive(Debug, Clone)]
pub struct CoreHirBundle {
    pub program: Program,
    pub method_resolutions: HashMap<u32, (String, String)>,
}

pub(crate) fn build_core_hir_program(
    checker: &mut TypeChecker,
    entry_name: &str,
    imports: &[ast::Import],
    items: &[ast::Item],
) -> CoreHirBundle {
    let method_resolutions = checker.method_resolutions.clone();
    let mut builder = CoreHirBuilder::new(checker);
    let program = builder.build_program(entry_name, imports, items);
    CoreHirBundle {
        program,
        method_resolutions,
    }
}

pub(crate) struct CoreHirBuilder<'a> {
    checker: &'a mut TypeChecker,
    program_id: ProgramId,
    next_module_id: u32,
    next_item_id: u32,
    next_body_id: u32,
    next_expr_id: u32,
    next_pattern_id: u32,
    next_local_id: u32,
    source_map: SourceMap,
    bodies: Vec<Body>,
    current_fn_effects: EffectFlags,
}

impl<'a> CoreHirBuilder<'a> {
    pub(crate) fn new(checker: &'a mut TypeChecker) -> Self {
        Self {
            checker,
            program_id: ProgramId(0),
            next_module_id: 0,
            next_item_id: 0,
            next_body_id: 0,
            next_expr_id: 0,
            next_pattern_id: 0,
            next_local_id: 0,
            source_map: SourceMap::default(),
            bodies: Vec::new(),
            current_fn_effects: EffectFlags::default(),
        }
    }

    pub(crate) fn build_program(
        &mut self,
        entry_name: &str,
        imports: &[ast::Import],
        items: &[ast::Item],
    ) -> Program {
        self.source_map
            .insert_program(self.program_id, Span::dummy());
        let module_id = self.fresh_module_id();
        self.source_map.insert_module(module_id, Span::dummy());
        let module = Module {
            id: module_id,
            name: entry_name.to_string(),
            imports: imports
                .iter()
                .map(|import| ImportRef {
                    module_name: import.module_name.clone(),
                    alias: import.alias.clone(),
                })
                .collect(),
            items: self.build_items(items),
        };
        Program {
            id: self.program_id,
            modules: vec![module],
            bodies: std::mem::take(&mut self.bodies),
            source_map: std::mem::take(&mut self.source_map),
        }
    }

    fn build_items(&mut self, items: &[ast::Item]) -> Vec<Item> {
        let mut built = Vec::new();
        for item in items {
            match item {
                ast::Item::FnDef(f) => built.push(self.build_function_item(f, None, None)),
                ast::Item::StructDef(s) => built.push(self.build_struct_item(s)),
                ast::Item::EnumDef(e) => built.push(self.build_enum_item(e)),
                ast::Item::TraitDef(t) => built.push(self.build_trait_item(t)),
                ast::Item::ImplBlock(ib) => {
                    let impl_id = Some(ark_hir::ImplId(self.next_item_id));
                    let mut method_ids = Vec::new();
                    for method in &ib.methods {
                        let item = self.build_function_item(method, impl_id, None);
                        method_ids.push(item.id);
                        built.push(item);
                    }
                    let item_id = self.fresh_item_id();
                    self.source_map.insert_item(item_id, ib.span);
                    built.push(Item {
                        id: item_id,
                        name: format!("impl:{}", ib.target_type),
                        kind: ItemKind::Impl(ImplItem {
                            impl_id,
                            trait_name: ib.trait_name.clone(),
                            target_type: ib.target_type.clone(),
                            methods: method_ids,
                        }),
                    });
                }
            }
        }
        built
    }

    fn build_function_item(
        &mut self,
        f: &ast::FnDef,
        impl_id: Option<ark_hir::ImplId>,
        method_item_id: Option<ItemId>,
    ) -> Item {
        let item_id = self.fresh_item_id();
        self.source_map.insert_item(item_id, f.span);
        let mut env = TypeEnv::new();
        let mut params = Vec::new();
        let mut param_locals = Vec::new();
        for param in &f.params {
            let ty = self.to_hir_ty(&self.checker.resolve_type_expr(&param.ty));
            let value_mode = if ty.is_reference() {
                ValueMode::SharedRef
            } else {
                ValueMode::ValueCopy
            };
            let local = Local {
                id: self.fresh_local_id(),
                name: param.name.clone(),
                ty: ty.clone(),
                value_mode,
            };
            env.bind(
                param.name.clone(),
                self.checker.resolve_type_expr(&param.ty),
            );
            param_locals.push(local.clone());
            params.push(Param {
                local_id: local.id,
                name: param.name.clone(),
                ty,
                value_mode,
            });
        }
        for tp in &f.type_params {
            env.bind(tp.clone(), self.checker.fresh_type_var());
        }
        let return_ty = f
            .return_type
            .as_ref()
            .map(|ty| self.to_hir_ty(&self.checker.resolve_type_expr(ty)))
            .unwrap_or(Ty::Unit);
        let prev_return = self.checker.current_fn_return_type.clone();
        self.checker.current_fn_return_type = Some(
            f.return_type
                .as_ref()
                .map(|ty| self.checker.resolve_type_expr(ty))
                .unwrap_or(Type::Unit),
        );
        let prev_effects = self.current_fn_effects;
        self.current_fn_effects = EffectFlags::default();
        let (body_id, body) =
            self.build_body(&f.body, &mut env, Some((impl_id, method_item_id, item_id)));
        self.bodies.push(body);
        let effects = self.current_fn_effects;
        self.current_fn_effects = prev_effects;
        self.checker.current_fn_return_type = prev_return;
        Item {
            id: item_id,
            name: f.name.clone(),
            kind: ItemKind::Function(FunctionItem {
                body: body_id,
                params,
                return_ty,
                type_params: f.type_params.clone(),
                type_param_bounds: f.type_param_bounds.clone(),
                effects,
            }),
        }
    }

    fn build_struct_item(&mut self, s: &ast::StructDef) -> Item {
        let item_id = self.fresh_item_id();
        self.source_map.insert_item(item_id, s.span);
        let type_id = self
            .checker
            .struct_defs
            .get(&s.name)
            .map(|info| ark_hir::TypeId(info.type_id.0));
        Item {
            id: item_id,
            name: s.name.clone(),
            kind: ItemKind::Struct(StructItem {
                type_id,
                fields: s
                    .fields
                    .iter()
                    .map(|field| FieldDef {
                        name: field.name.clone(),
                        ty: self.to_hir_ty(&self.checker.resolve_type_expr(&field.ty)),
                    })
                    .collect(),
                type_params: s.type_params.clone(),
            }),
        }
    }

    fn build_enum_item(&mut self, e: &ast::EnumDef) -> Item {
        let item_id = self.fresh_item_id();
        self.source_map.insert_item(item_id, e.span);
        let type_id = self
            .checker
            .enum_defs
            .get(&e.name)
            .map(|info| ark_hir::TypeId(info.type_id.0));
        Item {
            id: item_id,
            name: e.name.clone(),
            kind: ItemKind::Enum(ark_hir::EnumItem {
                type_id,
                variants: e
                    .variants
                    .iter()
                    .map(|variant| match variant {
                        ast::Variant::Unit { name, .. } => VariantDef {
                            name: name.clone(),
                            fields: vec![],
                        },
                        ast::Variant::Tuple { name, fields, .. } => VariantDef {
                            name: name.clone(),
                            fields: fields
                                .iter()
                                .enumerate()
                                .map(|(index, ty)| FieldDef {
                                    name: index.to_string(),
                                    ty: self.to_hir_ty(&self.checker.resolve_type_expr(ty)),
                                })
                                .collect(),
                        },
                        ast::Variant::Struct { name, fields, .. } => VariantDef {
                            name: name.clone(),
                            fields: fields
                                .iter()
                                .map(|field| FieldDef {
                                    name: field.name.clone(),
                                    ty: self.to_hir_ty(&self.checker.resolve_type_expr(&field.ty)),
                                })
                                .collect(),
                        },
                    })
                    .collect(),
                type_params: e.type_params.clone(),
            }),
        }
    }

    fn build_trait_item(&mut self, t: &ast::TraitDef) -> Item {
        let item_id = self.fresh_item_id();
        self.source_map.insert_item(item_id, t.span);
        Item {
            id: item_id,
            name: t.name.clone(),
            kind: ItemKind::Trait(TraitItem {
                trait_id: None,
                methods: t
                    .methods
                    .iter()
                    .map(|method| TraitMethodSig {
                        name: method.name.clone(),
                        params: method
                            .params
                            .iter()
                            .map(|param| Param {
                                local_id: self.fresh_local_id(),
                                name: param.name.clone(),
                                ty: self.to_hir_ty(&self.checker.resolve_type_expr(&param.ty)),
                                value_mode: ValueMode::ValueCopy,
                            })
                            .collect(),
                        return_ty: method
                            .return_type
                            .as_ref()
                            .map(|ty| self.to_hir_ty(&self.checker.resolve_type_expr(ty)))
                            .unwrap_or(Ty::Unit),
                    })
                    .collect(),
                type_params: t.type_params.clone(),
            }),
        }
    }

    fn build_body(
        &mut self,
        block: &ast::Block,
        env: &mut TypeEnv,
        selection_context: Option<(Option<ark_hir::ImplId>, Option<ItemId>, ItemId)>,
    ) -> (BodyId, Body) {
        let body_id = self.fresh_body_id();
        self.source_map.insert_body(body_id, block.span);
        let mut locals = Vec::new();
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            if let Some(stmt) = self.build_stmt(stmt, env, &mut locals, selection_context) {
                stmts.push(stmt);
            }
        }
        let tail = block
            .tail_expr
            .as_ref()
            .map(|expr| self.build_expr(expr, env, selection_context));
        (
            body_id,
            Body {
                id: body_id,
                params: vec![],
                locals,
                stmts,
                tail,
            },
        )
    }

    fn build_stmt(
        &mut self,
        stmt: &ast::Stmt,
        env: &mut TypeEnv,
        locals: &mut Vec<Local>,
        selection_context: Option<(Option<ark_hir::ImplId>, Option<ItemId>, ItemId)>,
    ) -> Option<Stmt> {
        match stmt {
            ast::Stmt::Let {
                name,
                ty,
                init,
                is_mut: _,
                pattern,
                ..
            } => {
                let init_expr = self.build_expr(init, env, selection_context);
                let init_ty = ty
                    .as_ref()
                    .map(|ty| self.to_hir_ty(&self.checker.resolve_type_expr(ty)))
                    .unwrap_or_else(|| init_expr.ty.clone());
                let binding_mode =
                    if matches!(init, ast::Expr::Ident { .. }) && init_ty.is_reference() {
                        ValueMode::SharedRef
                    } else {
                        ValueMode::ValueCopy
                    };
                let local = Local {
                    id: self.fresh_local_id(),
                    name: name.clone(),
                    ty: init_ty.clone(),
                    value_mode: binding_mode,
                };
                locals.push(local.clone());
                env.bind(name.clone(), self.hir_ty_to_type(&init_ty));
                Some(Stmt::Let(LetStmt {
                    local,
                    init: init_expr,
                    pattern: pattern
                        .as_ref()
                        .map(|pattern| self.build_pattern(pattern, env)),
                    binding_mode,
                }))
            }
            ast::Stmt::Expr(expr) => {
                Some(Stmt::Expr(self.build_expr(expr, env, selection_context)))
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond = self.build_expr(cond, env, selection_context);
                let mut child_env = env.child();
                let (body_id, body) = self.build_body(body, &mut child_env, selection_context);
                self.bodies.push(body);
                Some(Stmt::While {
                    cond,
                    body: body_id,
                })
            }
            ast::Stmt::Loop { body, .. } => {
                let mut child_env = env.child();
                let (body_id, body) = self.build_body(body, &mut child_env, selection_context);
                self.bodies.push(body);
                Some(Stmt::Loop { body: body_id })
            }
            ast::Stmt::For {
                target, iter, body, ..
            } => match iter {
                ast::ForIter::Range { start, end } => {
                    let start_expr = self.build_expr(start, env, selection_context);
                    let end_expr = self.build_expr(end, env, selection_context);
                    let local = Local {
                        id: self.fresh_local_id(),
                        name: target.clone(),
                        ty: start_expr.ty.clone(),
                        value_mode: ValueMode::ValueCopy,
                    };
                    let mut child_env = env.child();
                    child_env.bind(target.clone(), self.hir_ty_to_type(&local.ty));
                    let (body_id, body) = self.build_body(body, &mut child_env, selection_context);
                    self.bodies.push(body);
                    Some(Stmt::ForRange {
                        local,
                        start: start_expr,
                        end: end_expr,
                        body: body_id,
                    })
                }
                ast::ForIter::Values(expr) | ast::ForIter::Iter(expr) => {
                    let iter_expr = self.build_expr(expr, env, selection_context);
                    let element_ty = match &iter_expr.ty {
                        Ty::Vec(inner) => (**inner).clone(),
                        _ => Ty::I32,
                    };
                    let value_mode = if element_ty.is_reference() {
                        ValueMode::SharedRef
                    } else {
                        ValueMode::ValueCopy
                    };
                    let local = Local {
                        id: self.fresh_local_id(),
                        name: target.clone(),
                        ty: element_ty.clone(),
                        value_mode,
                    };
                    let mut child_env = env.child();
                    child_env.bind(target.clone(), self.hir_ty_to_type(&element_ty));
                    let (body_id, body) = self.build_body(body, &mut child_env, selection_context);
                    self.bodies.push(body);
                    Some(Stmt::ForValues {
                        local,
                        iter: iter_expr,
                        body: body_id,
                    })
                }
            },
        }
    }

    fn build_expr(
        &mut self,
        expr: &ast::Expr,
        env: &mut TypeEnv,
        selection_context: Option<(Option<ark_hir::ImplId>, Option<ItemId>, ItemId)>,
    ) -> Expr {
        let span = expr.span();
        let expr_id = self.fresh_expr_id();
        self.source_map.insert_expr(expr_id, span);
        let mut type_sink = ark_diagnostics::DiagnosticSink::new();
        let inferred_ty = self.checker.synthesize_expr(expr, env, &mut type_sink);
        let ty = self.to_hir_ty(&inferred_ty);
        let (kind, selection) = match expr {
            ast::Expr::IntLit { value, .. } => (ExprKind::Const(ConstValue::Int(*value)), None),
            ast::Expr::FloatLit { value, .. } => (ExprKind::Const(ConstValue::Float(*value)), None),
            ast::Expr::StringLit { value, .. } => {
                (ExprKind::Const(ConstValue::String(value.clone())), None)
            }
            ast::Expr::CharLit { value, .. } => (ExprKind::Const(ConstValue::Char(*value)), None),
            ast::Expr::BoolLit { value, .. } => (ExprKind::Const(ConstValue::Bool(*value)), None),
            ast::Expr::Ident { name, .. } => {
                if let Some(local_ty) = env.lookup(name) {
                    let local = LocalId(self.next_local_id.saturating_sub(1));
                    let _ = local_ty;
                    (ExprKind::Local(local), None)
                } else {
                    (ExprKind::Global(name.clone()), None)
                }
            }
            ast::Expr::QualifiedIdent { module, name, .. } => (
                ExprKind::QualifiedGlobal {
                    module: module.clone(),
                    name: name.clone(),
                },
                None,
            ),
            ast::Expr::Binary {
                left,
                op,
                right,
                span,
                ..
            } => {
                let left_expr = self.build_expr(left, env, selection_context);
                let right_expr = self.build_expr(right, env, selection_context);
                if let Some((function, self_type)) =
                    self.checker.method_resolutions.get(&span.start).cloned()
                {
                    let selection = make_selection(
                        function.clone(),
                        Some(Ty::Struct {
                            type_id: None,
                            name: self_type,
                            args: vec![],
                        }),
                        selection_context.map(|(_, _, item_id)| item_id),
                        selection_context.and_then(|(impl_id, _, _)| impl_id),
                        selection_kind_for_method_name(&function, true),
                    );
                    (
                        ExprKind::Call {
                            target: CallTarget::Selected {
                                function,
                                selection: selection.clone(),
                            },
                            args: vec![
                                CallArg {
                                    expr: left_expr,
                                    value_mode: ValueMode::SharedRef,
                                },
                                CallArg {
                                    expr: right_expr,
                                    value_mode: ValueMode::SharedRef,
                                },
                            ],
                        },
                        Some(selection),
                    )
                } else {
                    (
                        ExprKind::BuiltinBinary {
                            op: self.map_binop(*op),
                            left: Box::new(left_expr),
                            right: Box::new(right_expr),
                        },
                        None,
                    )
                }
            }
            ast::Expr::Unary { op, operand, .. } => (
                ExprKind::BuiltinUnary {
                    op: self.map_unaryop(*op),
                    operand: Box::new(self.build_expr(operand, env, selection_context)),
                },
                None,
            ),
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                let arg_exprs: Vec<CallArg> = args
                    .iter()
                    .map(|arg| {
                        let expr = self.build_expr(arg, env, selection_context);
                        let value_mode = if expr.ty.is_reference() {
                            ValueMode::SharedRef
                        } else {
                            ValueMode::ValueCopy
                        };
                        CallArg { expr, value_mode }
                    })
                    .collect();
                if let Some((function, self_type)) =
                    self.checker.method_resolutions.get(&span.start).cloned()
                {
                    let selection = make_selection(
                        function.clone(),
                        Some(Ty::Struct {
                            type_id: None,
                            name: self_type,
                            args: vec![],
                        }),
                        selection_context.map(|(_, _, item_id)| item_id),
                        selection_context.and_then(|(impl_id, _, _)| impl_id),
                        selection_kind_for_method_name(&function, false),
                    );
                    (
                        ExprKind::Call {
                            target: CallTarget::Selected {
                                function,
                                selection: selection.clone(),
                            },
                            args: arg_exprs,
                        },
                        Some(selection),
                    )
                } else if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    if name == "concat"
                        && arg_exprs
                            .iter()
                            .all(|arg| matches!(arg.expr.ty, Ty::String))
                    {
                        (
                            ExprKind::StringConcatMany(
                                arg_exprs.into_iter().map(|arg| arg.expr).collect(),
                            ),
                            None,
                        )
                    } else {
                        (
                            ExprKind::Call {
                                target: CallTarget::Direct {
                                    function: name.clone(),
                                },
                                args: arg_exprs,
                            },
                            None,
                        )
                    }
                } else {
                    (
                        ExprKind::Call {
                            target: CallTarget::Indirect(Box::new(self.build_expr(
                                callee,
                                env,
                                selection_context,
                            ))),
                            args: arg_exprs,
                        },
                        None,
                    )
                }
            }
            ast::Expr::FieldAccess { object, field, .. } => (
                ExprKind::FieldAccess {
                    object: Box::new(self.build_expr(object, env, selection_context)),
                    field: field.clone(),
                },
                None,
            ),
            ast::Expr::Index { object, index, .. } => (
                ExprKind::Index {
                    object: Box::new(self.build_expr(object, env, selection_context)),
                    index: Box::new(self.build_expr(index, env, selection_context)),
                },
                None,
            ),
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let cond = self.build_expr(cond, env, selection_context);
                let mut then_env = env.child();
                let (then_body, then_body_data) =
                    self.build_body(then_block, &mut then_env, selection_context);
                self.bodies.push(then_body_data);
                let else_body = else_block.as_ref().map(|else_block| {
                    let mut else_env = env.child();
                    let (body_id, body) =
                        self.build_body(else_block, &mut else_env, selection_context);
                    self.bodies.push(body);
                    body_id
                });
                (
                    ExprKind::If {
                        cond: Box::new(cond),
                        then_body,
                        else_body,
                    },
                    None,
                )
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => (
                ExprKind::Match {
                    scrutinee: Box::new(self.build_expr(scrutinee, env, selection_context)),
                    arms: arms
                        .iter()
                        .map(|arm| MatchArm {
                            pattern: self.build_pattern(&arm.pattern, env),
                            guard: arm
                                .guard
                                .as_ref()
                                .map(|guard| self.build_expr(guard, env, selection_context)),
                            body: self.build_expr(&arm.body, env, selection_context),
                        })
                        .collect(),
                },
                None,
            ),
            ast::Expr::Block(block) => {
                let mut child_env = env.child();
                let (body_id, body) = self.build_body(block, &mut child_env, selection_context);
                self.bodies.push(body);
                (ExprKind::Block(body_id), None)
            }
            ast::Expr::Tuple { elements, .. } => (
                ExprKind::Tuple(
                    elements
                        .iter()
                        .map(|expr| self.build_expr(expr, env, selection_context))
                        .collect(),
                ),
                None,
            ),
            ast::Expr::Array { elements, .. } => (
                ExprKind::Array(
                    elements
                        .iter()
                        .map(|expr| self.build_expr(expr, env, selection_context))
                        .collect(),
                ),
                None,
            ),
            ast::Expr::ArrayRepeat { value, count, .. } => (
                ExprKind::ArrayRepeat {
                    value: Box::new(self.build_expr(value, env, selection_context)),
                    count: Box::new(self.build_expr(count, env, selection_context)),
                },
                None,
            ),
            ast::Expr::StructInit { name, fields, .. } => (
                ExprKind::StructInit {
                    name: name.clone(),
                    fields: fields
                        .iter()
                        .map(|(name, expr)| {
                            (name.clone(), self.build_expr(expr, env, selection_context))
                        })
                        .collect(),
                },
                None,
            ),
            ast::Expr::Closure { params, body, .. } => {
                let mut child_env = env.child();
                let params: Vec<Param> = params
                    .iter()
                    .map(|param| {
                        let ty = param
                            .ty
                            .as_ref()
                            .map(|ty| self.to_hir_ty(&self.checker.resolve_type_expr(ty)))
                            .unwrap_or(Ty::I32);
                        child_env.bind(param.name.clone(), self.hir_ty_to_type(&ty));
                        Param {
                            local_id: self.fresh_local_id(),
                            name: param.name.clone(),
                            ty,
                            value_mode: ValueMode::ValueCopy,
                        }
                    })
                    .collect();
                let (body_id, body) = self.build_body(
                    &ast::Block {
                        stmts: vec![],
                        tail_expr: Some(body.clone()),
                        span,
                    },
                    &mut child_env,
                    selection_context,
                );
                self.bodies.push(body);
                (
                    ExprKind::Closure {
                        params,
                        body: body_id,
                        captures: self.capture_infos(env),
                    },
                    None,
                )
            }
            ast::Expr::Return { value, .. } => (
                ExprKind::Return(
                    value
                        .as_ref()
                        .map(|value| Box::new(self.build_expr(value, env, selection_context))),
                ),
                None,
            ),
            ast::Expr::Break { value, .. } => (
                ExprKind::Break(
                    value
                        .as_ref()
                        .map(|value| Box::new(self.build_expr(value, env, selection_context))),
                ),
                None,
            ),
            ast::Expr::Continue { .. } => (ExprKind::Continue, None),
            ast::Expr::Try { expr, span } => {
                let inner = self.build_expr(expr, env, selection_context);
                if let Some((function, self_type)) =
                    self.checker.method_resolutions.get(&span.start).cloned()
                {
                    let selection = make_selection(
                        function.clone(),
                        Some(Ty::Enum {
                            type_id: None,
                            name: self_type,
                            args: vec![],
                        }),
                        selection_context.map(|(_, _, item_id)| item_id),
                        selection_context.and_then(|(impl_id, _, _)| impl_id),
                        ark_hir::SelectionKind::FromConversion,
                    );
                    (ExprKind::Try(Box::new(inner)), Some(selection))
                } else {
                    (ExprKind::Try(Box::new(inner)), None)
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                let target_expr = self.build_expr(target, env, selection_context);
                let value_expr = self.build_expr(value, env, selection_context);
                let value_mode = if value_expr.ty.is_reference() {
                    ValueMode::SharedRef
                } else {
                    ValueMode::ValueCopy
                };
                (
                    ExprKind::Assign {
                        target: Box::new(target_expr),
                        value: Box::new(value_expr),
                        value_mode,
                    },
                    None,
                )
            }
            ast::Expr::Loop { body, .. } => {
                let mut child_env = env.child();
                let (body_id, body) = self.build_body(body, &mut child_env, selection_context);
                self.bodies.push(body);
                (ExprKind::Loop(body_id), None)
            }
        };
        Expr {
            id: expr_id,
            kind,
            ty,
            selection,
        }
    }

    fn build_pattern(&mut self, pattern: &ast::Pattern, env: &mut TypeEnv) -> Pattern {
        let pattern_id = self.fresh_pattern_id();
        self.source_map.insert_pattern(pattern_id, pattern.span());
        let ty = Ty::I32;
        let kind = match pattern {
            ast::Pattern::Wildcard(_) => PatternKind::Wildcard,
            ast::Pattern::Ident { name, .. } => {
                let local = self.fresh_local_id();
                env.bind(name.clone(), Type::I32);
                PatternKind::Binding {
                    local,
                    name: name.clone(),
                }
            }
            ast::Pattern::IntLit { value, .. } => PatternKind::Const(ConstValue::Int(*value)),
            ast::Pattern::FloatLit { value, .. } => PatternKind::Const(ConstValue::Float(*value)),
            ast::Pattern::StringLit { value, .. } => {
                PatternKind::Const(ConstValue::String(value.clone()))
            }
            ast::Pattern::CharLit { value, .. } => PatternKind::Const(ConstValue::Char(*value)),
            ast::Pattern::BoolLit { value, .. } => PatternKind::Const(ConstValue::Bool(*value)),
            ast::Pattern::Tuple { elements, .. } => PatternKind::Tuple(
                elements
                    .iter()
                    .map(|pattern| self.build_pattern(pattern, env))
                    .collect(),
            ),
            ast::Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => PatternKind::Enum {
                path: path.clone(),
                variant: variant.clone(),
                fields: fields
                    .iter()
                    .map(|pattern| self.build_pattern(pattern, env))
                    .collect(),
            },
            ast::Pattern::Or { patterns, .. } => PatternKind::Or(
                patterns
                    .iter()
                    .map(|pattern| self.build_pattern(pattern, env))
                    .collect(),
            ),
            ast::Pattern::Struct { name, fields, .. } => PatternKind::Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(field, pattern)| {
                        (
                            field.clone(),
                            pattern
                                .as_ref()
                                .map(|pattern| self.build_pattern(pattern, env)),
                        )
                    })
                    .collect(),
            },
        };
        Pattern {
            id: pattern_id,
            kind,
            ty,
        }
    }

    fn capture_infos(&self, env: &TypeEnv) -> Vec<CaptureInfo> {
        env.bindings
            .iter()
            .map(|(name, ty)| {
                let hir_ty = self.to_hir_ty(ty);
                CaptureInfo {
                    local: LocalId(0),
                    name: name.clone(),
                    ty: hir_ty.clone(),
                    value_mode: if hir_ty.is_reference() {
                        ValueMode::SharedRef
                    } else {
                        ValueMode::ValueCopy
                    },
                }
            })
            .collect()
    }

    pub(crate) fn to_hir_ty(&self, ty: &Type) -> Ty {
        match ty {
            Type::I32 => Ty::I32,
            Type::I64 => Ty::I64,
            Type::F32 => Ty::F32,
            Type::F64 => Ty::F64,
            Type::Bool => Ty::Bool,
            Type::Char => Ty::Char,
            Type::Unit => Ty::Unit,
            Type::String => Ty::String,
            Type::Struct(id) => {
                let name = self
                    .checker
                    .struct_defs
                    .iter()
                    .find(|(_, info)| info.type_id == *id)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| format!("struct#{}", id.0));
                Ty::Struct {
                    type_id: Some(ark_hir::TypeId(id.0)),
                    name,
                    args: vec![],
                }
            }
            Type::Enum(id) => {
                let name = self
                    .checker
                    .enum_defs
                    .iter()
                    .find(|(_, info)| info.type_id == *id)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| format!("enum#{}", id.0));
                Ty::Enum {
                    type_id: Some(ark_hir::TypeId(id.0)),
                    name,
                    args: vec![],
                }
            }
            Type::Tuple(types) => Ty::Tuple(types.iter().map(|ty| self.to_hir_ty(ty)).collect()),
            Type::Array(elem, size) => Ty::Array(Box::new(self.to_hir_ty(elem)), *size),
            Type::Slice(elem) => Ty::Slice(Box::new(self.to_hir_ty(elem))),
            Type::Vec(elem) => Ty::Vec(Box::new(self.to_hir_ty(elem))),
            Type::Option(inner) => Ty::Option(Box::new(self.to_hir_ty(inner))),
            Type::Result(ok, err) => {
                Ty::Result(Box::new(self.to_hir_ty(ok)), Box::new(self.to_hir_ty(err)))
            }
            Type::Function { params, ret } => Ty::Function {
                params: params.iter().map(|ty| self.to_hir_ty(ty)).collect(),
                ret: Box::new(self.to_hir_ty(ret)),
            },
            Type::TypeVar(id) => Ty::TypeParam(format!("T{}", id)),
            Type::Never => Ty::Never,
            Type::Error => Ty::Error,
            Type::Any => Ty::Error, // erased generic — no concrete HIR type
        }
    }

    fn hir_ty_to_type(&self, ty: &Ty) -> Type {
        match ty {
            Ty::I32 => Type::I32,
            Ty::I64 => Type::I64,
            Ty::F32 => Type::F32,
            Ty::F64 => Type::F64,
            Ty::Bool => Type::Bool,
            Ty::Char => Type::Char,
            Ty::Unit => Type::Unit,
            Ty::String => Type::String,
            Ty::Struct { type_id, name, .. } => type_id
                .map(|id| Type::Struct(crate::types::TypeId(id.0)))
                .or_else(|| {
                    self.checker
                        .struct_defs
                        .get(name)
                        .map(|info| Type::Struct(info.type_id))
                })
                .unwrap_or(Type::Error),
            Ty::Enum { type_id, name, .. } => type_id
                .map(|id| Type::Enum(crate::types::TypeId(id.0)))
                .or_else(|| {
                    self.checker
                        .enum_defs
                        .get(name)
                        .map(|info| Type::Enum(info.type_id))
                })
                .unwrap_or(Type::Error),
            Ty::Tuple(types) => {
                Type::Tuple(types.iter().map(|ty| self.hir_ty_to_type(ty)).collect())
            }
            Ty::Array(elem, size) => Type::Array(Box::new(self.hir_ty_to_type(elem)), *size),
            Ty::Slice(elem) => Type::Slice(Box::new(self.hir_ty_to_type(elem))),
            Ty::Vec(elem) => Type::Vec(Box::new(self.hir_ty_to_type(elem))),
            Ty::Option(inner) => Type::Option(Box::new(self.hir_ty_to_type(inner))),
            Ty::Result(ok, err) => Type::Result(
                Box::new(self.hir_ty_to_type(ok)),
                Box::new(self.hir_ty_to_type(err)),
            ),
            Ty::Function { params, ret } => Type::Function {
                params: params.iter().map(|ty| self.hir_ty_to_type(ty)).collect(),
                ret: Box::new(self.hir_ty_to_type(ret)),
            },
            Ty::TypeParam(_) => Type::Error,
            Ty::Never => Type::Never,
            Ty::Error => Type::Error,
        }
    }

    fn map_binop(&self, op: ast::BinOp) -> BuiltinBinaryOp {
        match op {
            ast::BinOp::Add => BuiltinBinaryOp::Add,
            ast::BinOp::Sub => BuiltinBinaryOp::Sub,
            ast::BinOp::Mul => BuiltinBinaryOp::Mul,
            ast::BinOp::Div => BuiltinBinaryOp::Div,
            ast::BinOp::Mod => BuiltinBinaryOp::Mod,
            ast::BinOp::Eq => BuiltinBinaryOp::Eq,
            ast::BinOp::Ne => BuiltinBinaryOp::Ne,
            ast::BinOp::Lt => BuiltinBinaryOp::Lt,
            ast::BinOp::Le => BuiltinBinaryOp::Le,
            ast::BinOp::Gt => BuiltinBinaryOp::Gt,
            ast::BinOp::Ge => BuiltinBinaryOp::Ge,
            ast::BinOp::And => BuiltinBinaryOp::And,
            ast::BinOp::Or => BuiltinBinaryOp::Or,
            ast::BinOp::BitAnd => BuiltinBinaryOp::BitAnd,
            ast::BinOp::BitOr => BuiltinBinaryOp::BitOr,
            ast::BinOp::BitXor => BuiltinBinaryOp::BitXor,
            ast::BinOp::Shl => BuiltinBinaryOp::Shl,
            ast::BinOp::Shr => BuiltinBinaryOp::Shr,
        }
    }

    fn map_unaryop(&self, op: ast::UnaryOp) -> BuiltinUnaryOp {
        match op {
            ast::UnaryOp::Neg => BuiltinUnaryOp::Neg,
            ast::UnaryOp::Not => BuiltinUnaryOp::Not,
            ast::UnaryOp::BitNot => BuiltinUnaryOp::BitNot,
        }
    }

    fn fresh_module_id(&mut self) -> ModuleId {
        let id = ModuleId(self.next_module_id);
        self.next_module_id += 1;
        id
    }

    fn fresh_item_id(&mut self) -> ItemId {
        let id = ItemId(self.next_item_id);
        self.next_item_id += 1;
        id
    }

    fn fresh_body_id(&mut self) -> BodyId {
        let id = BodyId(self.next_body_id);
        self.next_body_id += 1;
        id
    }

    fn fresh_expr_id(&mut self) -> HirExprId {
        let id = HirExprId(self.next_expr_id);
        self.next_expr_id += 1;
        id
    }

    fn fresh_pattern_id(&mut self) -> PatternId {
        let id = PatternId(self.next_pattern_id);
        self.next_pattern_id += 1;
        id
    }

    fn fresh_local_id(&mut self) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        id
    }
}
