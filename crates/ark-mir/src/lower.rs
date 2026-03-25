//! Lower typed AST to MIR.

use std::collections::{HashMap, HashSet};

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

    // Collect enum variant tags: "EnumName::Variant" -> tag index
    let mut enum_tags: HashMap<String, i32> = HashMap::new();
    for item in &module.items {
        if let ast::Item::EnumDef(e) = item {
            for (i, variant) in e.variants.iter().enumerate() {
                let vname = match variant {
                    ast::Variant::Unit { name, .. } => name,
                    ast::Variant::Tuple { name, .. } => name,
                    ast::Variant::Struct { name, .. } => name,
                };
                enum_tags.insert(format!("{}::{}", e.name, vname), i as i32);
            }
        }
    }

    for item in &module.items {
        if let ast::Item::FnDef(f) = item {
            let fn_id = FnId(next_fn_id);
            next_fn_id += 1;

            let mut ctx = LowerCtx::new(enum_tags.clone());

            for param in &f.params {
                let pid = ctx.declare_local(&param.name);
                if is_string_type(&param.ty) {
                    ctx.string_locals.insert(pid.0);
                }
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
                return_ty: match &f.return_type {
                    Some(ty) if is_string_type(ty) => ark_typecheck::types::Type::String,
                    Some(_) => ark_typecheck::types::Type::I32,
                    None => ark_typecheck::types::Type::Unit,
                },
                locals: ctx.locals.iter().map(|(name, id)| MirLocal {
                    id: *id,
                    name: Some(name.clone()),
                    ty: if ctx.string_locals.contains(&id.0) {
                        ark_typecheck::types::Type::String
                    } else {
                        ark_typecheck::types::Type::I32
                    },
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
    string_locals: HashSet<u32>,
    enum_tags: HashMap<String, i32>,
}

impl LowerCtx {
    fn new(enum_tags: HashMap<String, i32>) -> Self {
        Self { locals: Vec::new(), next_local: 0, string_locals: HashSet::new(), enum_tags }
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
            ast::Stmt::Let { name, init, ty, .. } => {
                if name == "_" {
                    // Wildcard binding: evaluate for side effects only
                    self.lower_expr_stmt(init, out);
                    return;
                }
                let local_id = self.declare_local(name);
                if let Some(type_expr) = ty {
                    if is_string_type(type_expr) {
                        self.string_locals.insert(local_id.0);
                    }
                }
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
            ast::Expr::Match { scrutinee, arms, .. } => {
                self.lower_match_stmt(scrutinee, arms, out);
            }
            _ => {}
        }
    }

    /// Lower a match expression used as a statement (result discarded).
    /// Converts to nested if-else chains.
    fn lower_match_stmt(&mut self, scrutinee: &ast::Expr, arms: &[ast::MatchArm], out: &mut Vec<MirStmt>) {
        let scrut = self.lower_expr(scrutinee);
        // Build a chain of if-else from the arms
        let stmt = self.build_match_if_chain(&scrut, arms, 0, true);
        if let Some(s) = stmt {
            out.push(s);
        }
    }

    /// Build a nested if-else chain from match arms starting at `idx`.
    /// `as_stmt` indicates whether arm bodies should be lowered as statements.
    fn build_match_if_chain(&mut self, scrut: &Operand, arms: &[ast::MatchArm], idx: usize, as_stmt: bool) -> Option<MirStmt> {
        if idx >= arms.len() {
            return None;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) => {
                // Default arm — just emit the body
                let mut body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut body);
                if body.len() == 1 {
                    Some(body.remove(0))
                } else {
                    // Wrap in an always-true if
                    Some(MirStmt::IfStmt {
                        cond: Operand::ConstBool(true),
                        then_body: body,
                        else_body: vec![],
                    })
                }
            }
            ast::Pattern::IntLit { value, .. } => {
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                    vec![next]
                } else {
                    vec![]
                };
                Some(MirStmt::IfStmt { cond, then_body, else_body })
            }
            ast::Pattern::BoolLit { value, .. } => {
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                    vec![next]
                } else {
                    vec![]
                };
                Some(MirStmt::IfStmt { cond, then_body, else_body })
            }
            ast::Pattern::StringLit { value, .. } => {
                // String match — for now, treat as literal comparison
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstString(value.clone())),
                );
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                    vec![next]
                } else {
                    vec![]
                };
                Some(MirStmt::IfStmt { cond, then_body, else_body })
            }
            ast::Pattern::Ident { name, .. } => {
                // Binding pattern — bind the scrutinee to the name
                let local_id = self.declare_local(name);
                let mut then_body = vec![
                    MirStmt::Assign(Place::Local(local_id), Rvalue::Use(scrut.clone()))
                ];
                self.lower_expr_stmt(&arm.body, &mut then_body);
                Some(MirStmt::IfStmt {
                    cond: Operand::ConstBool(true),
                    then_body,
                    else_body: vec![],
                })
            }
            ast::Pattern::Enum { path, variant, .. } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(scrut.clone()),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    let mut then_body = Vec::new();
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                    Some(MirStmt::IfStmt { cond, then_body, else_body })
                } else {
                    self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                }
            }
            _ => {
                // Skip unsupported patterns, try next arm
                self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
            }
        }
    }

    /// Build a nested IfExpr from match arms for value-returning match.
    fn build_match_if_expr(&mut self, scrut: &Operand, arms: &[ast::MatchArm], idx: usize) -> Operand {
        if idx >= arms.len() {
            return Operand::Unit;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) | ast::Pattern::Ident { .. } => {
                // Default/binding arm — just return the body value
                self.lower_expr(&arm.body)
            }
            ast::Pattern::IntLit { value, .. } => {
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
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
                let cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
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
            ast::Pattern::Enum { path, variant, .. } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(scrut.clone()),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    let then_result = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    Operand::IfExpr {
                        cond: Box::new(cond),
                        then_body: vec![],
                        then_result: Some(Box::new(then_result)),
                        else_body: vec![],
                        else_result: Some(Box::new(else_result)),
                    }
                } else {
                    self.build_match_if_expr(scrut, arms, idx + 1)
                }
            }
            _ => {
                // Skip unsupported patterns
                self.build_match_if_expr(scrut, arms, idx + 1)
            }
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
            ast::Expr::Match { scrutinee, arms, .. } => {
                let scrut = self.lower_expr(scrutinee);
                self.build_match_if_expr(&scrut, arms, 0)
            }
            ast::Expr::QualifiedIdent { module, name, .. } => {
                // Enum variant constructor: Direction::South -> tag integer
                let key = format!("{}::{}", module, name);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    Operand::ConstI32(tag)
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
            match &then_block.tail_expr {
                None => true,
                Some(tail) => is_void_expr(tail),
            }
        }
        ast::Expr::Match { arms, .. } => {
            // Match is void if its first arm body is void
            arms.first().map_or(true, |arm| is_void_expr(&arm.body))
        }
        _ => false,
    }
}

fn is_string_type(ty: &ast::TypeExpr) -> bool {
    matches!(ty, ast::TypeExpr::Named { name, .. } if name == "String")
}
