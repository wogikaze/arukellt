//! Statement checking (let, while, for, loop) and block checking.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_parser::ast;

use crate::types::Type;

use super::{TypeChecker, TypeEnv};

impl TypeChecker {
    pub(crate) fn check_block(
        &mut self,
        block: &ast::Block,
        env: &mut TypeEnv,
        _expected: &Type,
        sink: &mut DiagnosticSink,
    ) -> Type {
        for stmt in &block.stmts {
            self.check_stmt(stmt, env, sink);
        }

        if let Some(tail) = &block.tail_expr {
            self.synthesize_expr(tail, env, sink)
        } else {
            Type::Unit
        }
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt, env: &mut TypeEnv, sink: &mut DiagnosticSink) {
        match stmt {
            ast::Stmt::Let {
                name,
                ty,
                init,
                is_mut,
                pattern,
                ..
            } => {
                // Handle tuple destructuring: let (a, b) = expr
                if let Some(ast::Pattern::Tuple { elements, .. }) = pattern {
                    let init_type = self.synthesize_expr(init, env, sink);
                    // Bind each element name as i32 (our universal runtime type)
                    for elem in elements {
                        if let ast::Pattern::Ident {
                            name: elem_name, ..
                        } = elem
                        {
                            env.bind(elem_name.clone(), Type::I32);
                        }
                    }
                    let _ = init_type;
                    return;
                }
                let init_type = if let Some(type_expr) = ty {
                    let declared_type = self.resolve_type_expr(type_expr);
                    // For numeric literals, allow them to match declared type
                    let init_type = match (init, &declared_type) {
                        (ast::Expr::IntLit { .. }, Type::I64) => Type::I64,
                        (ast::Expr::IntLit { .. }, Type::F64) => Type::F64,
                        (ast::Expr::IntLit { .. }, Type::F32) => Type::F32,
                        (ast::Expr::IntLit { .. }, Type::U8) => Type::U8,
                        (ast::Expr::IntLit { .. }, Type::U16) => Type::U16,
                        (ast::Expr::IntLit { .. }, Type::U32) => Type::U32,
                        (ast::Expr::IntLit { .. }, Type::U64) => Type::U64,
                        (ast::Expr::IntLit { .. }, Type::I8) => Type::I8,
                        (ast::Expr::IntLit { .. }, Type::I16) => Type::I16,
                        _ => self.synthesize_expr(init, env, sink),
                    };
                    if !self.types_compatible(&init_type, &declared_type) {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0200)
                                .with_message(format!(
                                    "expected `{}`, found `{}`",
                                    declared_type, init_type
                                ))
                                .with_note(format!(
                                    "the declared type `{}` does not match the initializer type `{}`",
                                    declared_type, init_type
                                ))
                                .with_help(
                                    "ensure the initializer expression matches the declared type, or use `as` for explicit numeric conversion",
                                ),
                        );
                    }
                    if *is_mut {
                        env.bind_mut(name.clone(), declared_type.clone());
                    } else {
                        env.bind(name.clone(), declared_type.clone());
                    }
                    declared_type
                } else {
                    let init_type = self.synthesize_expr(init, env, sink);
                    if *is_mut {
                        env.bind_mut(name.clone(), init_type.clone());
                    } else {
                        env.bind(name.clone(), init_type.clone());
                    }
                    init_type
                };
                let _ = init_type;
                // W0001: warn on aliasing mutable reference types
                if let ast::Expr::Ident {
                    name: src_name,
                    span: src_span,
                    ..
                } = init
                    && env.is_mutable(src_name)
                {
                    let src_ty = env.lookup(src_name).cloned().unwrap_or(Type::Error);
                    if matches!(src_ty, Type::Vec(_)) {
                        sink.emit(
                                Diagnostic::new(DiagnosticCode::W0001)
                                    .with_label(*src_span, format!(
                                        "mutable `{}` is shared by reference; mutations affect both variables",
                                        src_name
                                    )),
                            );
                    }
                }
            }
            ast::Stmt::Expr(expr) => {
                self.synthesize_expr(expr, env, sink);
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond_ty = self.synthesize_expr(cond, env, sink);
                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "while condition must be `bool`, found `{}`",
                        cond_ty
                    )));
                }
                self.check_block(body, env, &Type::Unit, sink);
            }
            ast::Stmt::Loop { body, .. } => {
                self.check_block(body, env, &Type::Unit, sink);
            }
            ast::Stmt::For {
                target, iter, body, ..
            } => {
                let elem_ty = match iter {
                    ast::ForIter::Range { start, end, .. } => {
                        let start_ty = self.synthesize_expr(start, env, sink);
                        let end_ty = self.synthesize_expr(end, env, sink);
                        if start_ty != Type::I32 && start_ty != Type::I64 && start_ty != Type::Error
                        {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                format!("range start must be integer, found `{}`", start_ty),
                            ));
                        }
                        if end_ty != Type::I32 && end_ty != Type::I64 && end_ty != Type::Error {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                format!("range end must be integer, found `{}`", end_ty),
                            ));
                        }
                        start_ty
                    }
                    ast::ForIter::Values(expr) => {
                        let vec_ty = self.synthesize_expr(expr, env, sink);
                        // Element type from Vec<T> — currently always i32
                        match &vec_ty {
                            Type::Vec(inner) => *inner.clone(),
                            _ => Type::I32,
                        }
                    }
                    ast::ForIter::Iter(expr) => {
                        let iter_ty = self.synthesize_expr(expr, env, sink);
                        // Check if the type has a `next` method returning Option<T>
                        if let Type::Struct(type_id) = &iter_ty {
                            let struct_name = self
                                .struct_defs
                                .values()
                                .find(|s| s.type_id == *type_id)
                                .map(|s| s.name.clone());
                            if let Some(ref sname) = struct_name {
                                let mangled = format!("{}__next", sname);
                                if let Some(sig) = self.fn_sigs.get(&mangled).cloned() {
                                    // Extract T from Option<T> return type
                                    match &sig.ret {
                                        Type::Option(inner) => *inner.clone(),
                                        _ => {
                                            sink.emit(
                                                Diagnostic::new(DiagnosticCode::E0200)
                                                    .with_message(format!(
                                                    "`{}`::next() must return Option<T>, found `{}`",
                                                    sname, sig.ret
                                                )),
                                            );
                                            Type::Error
                                        }
                                    }
                                } else {
                                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                        format!(
                                            "`{}` does not implement Iterator (no `next` method)",
                                            sname
                                        ),
                                    ));
                                    Type::Error
                                }
                            } else {
                                sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                    "for..in requires a type that implements Iterator",
                                ));
                                Type::Error
                            }
                        } else {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                format!("for..in requires an Iterator type, found `{}`", iter_ty),
                            ));
                            Type::Error
                        }
                    }
                };
                let mut child_env = env.child();
                child_env.bind_mut(target.clone(), elem_ty);
                self.check_block(body, &mut child_env, &Type::Unit, sink);
            }
        }
    }
}
