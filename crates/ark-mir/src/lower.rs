//! Lower typed AST to MIR.

use ark_diagnostics::DiagnosticSink;
use ark_parser::ast;
use ark_typecheck::TypeChecker;

use crate::mir::*;

/// Lower a type-checked module to MIR.
pub fn lower_to_mir(
    module: &ast::Module,
    _checker: &TypeChecker,
    _sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = MirModule::new();
    let mut next_fn_id = 0u32;

    for item in &module.items {
        if let ast::Item::FnDef(f) = item {
            let fn_id = FnId(next_fn_id);
            next_fn_id += 1;

            let mut ctx = LowerCtx::new();

            for param in &f.params {
                ctx.declare_local(&param.name);
            }

            let entry = BlockId(0);
            let mut stmts = ctx.lower_block(&f.body);
            
            // Handle tail expression: if it's a void call (println etc.), 
            // lower it as a statement. Otherwise, it's the return value.
            let tail_op = if let Some(tail) = &f.body.tail_expr {
                if is_void_expr(tail) {
                    ctx.lower_expr_stmt(tail, &mut stmts);
                    None
                } else {
                    Some(ctx.lower_expr(tail))
                }
            } else {
                None
            };

            let mir_fn = MirFunction {
                id: fn_id,
                name: f.name.clone(),
                params: f.params.iter().enumerate().map(|(i, p)| MirLocal {
                    id: LocalId(i as u32),
                    name: Some(p.name.clone()),
                    ty: ark_typecheck::types::Type::I32,
                }).collect(),
                return_ty: if f.return_type.is_some() {
                    ark_typecheck::types::Type::I32
                } else {
                    ark_typecheck::types::Type::Unit
                },
                locals: ctx.locals.iter().map(|(name, id)| MirLocal {
                    id: *id,
                    name: Some(name.clone()),
                    ty: ark_typecheck::types::Type::I32,
                }).collect(),
                blocks: vec![BasicBlock {
                    id: entry,
                    stmts,
                    terminator: if let Some(op) = tail_op {
                        Terminator::Return(Some(op))
                    } else {
                        Terminator::Return(None)
                    },
                }],
                entry,
            };

            if f.name == "main" {
                mir.entry_fn = Some(fn_id);
            }

            mir.functions.push(mir_fn);
        }
    }

    mir
}

struct LowerCtx {
    locals: Vec<(String, LocalId)>,
    next_local: u32,
}

impl LowerCtx {
    fn new() -> Self {
        Self { locals: Vec::new(), next_local: 0 }
    }

    fn declare_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        self.locals.push((name.to_string(), id));
        id
    }

    fn lookup_local(&self, name: &str) -> Option<LocalId> {
        self.locals.iter().rev().find(|(n, _)| n == name).map(|(_, id)| *id)
    }

    fn lower_block(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            self.lower_stmt(stmt, &mut stmts);
        }
        stmts
    }

    /// Lower a block including its tail expression as a statement.
    fn lower_block_all(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = self.lower_block(block);
        if let Some(tail) = &block.tail_expr {
            self.lower_expr_stmt(tail, &mut stmts);
        }
        stmts
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt, out: &mut Vec<MirStmt>) {
        match stmt {
            ast::Stmt::Let { name, init, .. } => {
                if name == "_" {
                    // Wildcard binding: evaluate for side effects only
                    self.lower_expr_stmt(init, out);
                    return;
                }
                let local_id = self.declare_local(name);
                let op = self.lower_expr(init);
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
        }
    }

    fn lower_expr_stmt(&mut self, expr: &ast::Expr, out: &mut Vec<MirStmt>) {
        match expr {
            ast::Expr::Call { callee, args, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    out.push(MirStmt::CallBuiltin {
                        dest: None,
                        name: name.clone(),
                        args: mir_args,
                    });
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                if let ast::Expr::Ident { name, .. } = target.as_ref() {
                    if let Some(local_id) = self.lookup_local(name) {
                        let op = self.lower_expr(value);
                        out.push(MirStmt::Assign(Place::Local(local_id), Rvalue::Use(op)));
                    }
                }
            }
            ast::Expr::If { cond, then_block, else_block, .. } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block_all(then_block);
                let else_stmts = else_block.as_ref()
                    .map(|b| self.lower_block_all(b))
                    .unwrap_or_default();
                out.push(MirStmt::IfStmt {
                    cond: c,
                    then_body: then_stmts,
                    else_body: else_stmts,
                });
            }
            ast::Expr::Break { .. } => {
                out.push(MirStmt::Break);
            }
            ast::Expr::Continue { .. } => {
                out.push(MirStmt::Continue);
            }
            ast::Expr::Return { value, .. } => {
                let op = value.as_ref().map(|v| self.lower_expr(v));
                out.push(MirStmt::Return(op));
            }
            _ => {}
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr) -> Operand {
        match expr {
            ast::Expr::StringLit { value, .. } => Operand::ConstString(value.clone()),
            ast::Expr::IntLit { value, .. } => Operand::ConstI32(*value as i32),
            ast::Expr::FloatLit { value, .. } => Operand::ConstF64(*value),
            ast::Expr::BoolLit { value, .. } => Operand::ConstBool(*value),
            ast::Expr::CharLit { value, .. } => Operand::ConstChar(*value),
            ast::Expr::Ident { name, .. } => {
                if let Some(local_id) = self.lookup_local(name) {
                    Operand::Place(Place::Local(local_id))
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Binary { op, left, right, .. } => {
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
            ast::Expr::Call { callee, args, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    Operand::Call(name.clone(), mir_args)
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::If { cond, then_block, else_block, .. } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block(then_block);
                let then_tail = then_block.tail_expr.as_ref().map(|e| self.lower_expr(e));
                let else_stmts = else_block.as_ref().map(|b| self.lower_block(b)).unwrap_or_default();
                let else_tail = else_block.as_ref()
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
                if let Some(tail) = &block.tail_expr {
                    self.lower_expr(tail)
                } else {
                    Operand::Unit
                }
            }
            _ => Operand::Unit,
        }
    }
}

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

/// Check if an expression is void (should be emitted as statement, not value).
fn is_void_expr(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Call { callee, .. } => {
            if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                matches!(name.as_str(), "println" | "print" | "eprintln")
            } else {
                false
            }
        }
        ast::Expr::Assign { .. } => true,
        ast::Expr::If { then_block, .. } => {
            // If the then branch's tail is void, the whole if is void
            match &then_block.tail_expr {
                None => true,
                Some(tail) => is_void_expr(tail),
            }
        }
        _ => false,
    }
}
