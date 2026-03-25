//! Lower typed AST to MIR.
//!
//! This pass converts the high-level AST into a CFG-based MIR,
//! performing monomorphization along the way.

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

            let entry = BlockId(0);
            let stmts = lower_block_stmts(&f.body);

            let mir_fn = MirFunction {
                id: fn_id,
                name: f.name.clone(),
                params: Vec::new(),
                return_ty: ark_typecheck::types::Type::Unit,
                locals: Vec::new(),
                blocks: vec![BasicBlock {
                    id: entry,
                    stmts,
                    terminator: Terminator::Return(None),
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

fn lower_block_stmts(block: &ast::Block) -> Vec<MirStmt> {
    let mut stmts = Vec::new();
    for stmt in &block.stmts {
        lower_stmt(stmt, &mut stmts);
    }
    if let Some(tail) = &block.tail_expr {
        lower_expr_stmt(tail, &mut stmts);
    }
    stmts
}

fn lower_stmt(stmt: &ast::Stmt, out: &mut Vec<MirStmt>) {
    match stmt {
        ast::Stmt::Expr(expr) => lower_expr_stmt(expr, out),
        _ => {} // TODO: let bindings, while, loop
    }
}

fn lower_expr_stmt(expr: &ast::Expr, out: &mut Vec<MirStmt>) {
    if let ast::Expr::Call { callee, args, .. } = expr {
        if let ast::Expr::Ident { name, .. } = callee.as_ref() {
            // Check if this is a builtin function call
            if is_builtin(name) {
                let mir_args: Vec<Operand> = args.iter().map(|a| lower_operand(a)).collect();
                out.push(MirStmt::CallBuiltin {
                    dest: None,
                    name: name.clone(),
                    args: mir_args,
                });
                return;
            }
        }
    }
    // TODO: general expression lowering
}

fn lower_operand(expr: &ast::Expr) -> Operand {
    match expr {
        ast::Expr::StringLit { value, .. } => Operand::ConstString(value.clone()),
        ast::Expr::IntLit { value, .. } => Operand::ConstI32(*value as i32),
        ast::Expr::FloatLit { value, .. } => Operand::ConstF64(*value),
        ast::Expr::BoolLit { value, .. } => Operand::ConstBool(*value),
        ast::Expr::CharLit { value, .. } => Operand::ConstChar(*value),
        _ => Operand::Unit, // TODO: complex expressions
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(name, "println" | "print" | "eprintln")
}
