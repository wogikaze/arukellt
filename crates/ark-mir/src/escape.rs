//! Escape analysis for MIR.
//!
//! Determines whether heap-allocated locals (String, Vec, struct, enum)
//! escape their defining function. Non-escaping allocations are candidates
//! for stack placement in backends that support it (T3 Wasm GC, T4 native).
//!
//! A local escapes if:
//!   - It is returned from the function
//!   - It is passed as an argument to a function call
//!   - It is stored into a struct field or array element
//!   - It is used in a CallBuiltin that may capture (push, set, etc.)

use crate::mir::*;
use std::collections::{HashMap, HashSet};

/// Allocation hint for a local variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocHint {
    /// This local never escapes; can be stack-allocated.
    Stack,
    /// This local may escape; must be heap-allocated.
    Heap,
}

/// Result of escape analysis on a single function.
#[derive(Debug, Clone)]
pub struct EscapeInfo {
    /// Map from local ID to allocation hint.
    pub hints: HashMap<u32, AllocHint>,
    /// Set of locals that are heap-typed (String, Vec, struct, enum).
    pub heap_locals: HashSet<u32>,
    /// Number of locals that can be stack-allocated.
    pub stack_eligible: usize,
    /// Number of locals that must be heap-allocated.
    pub heap_required: usize,
}

/// Result of escape analysis on an entire module.
#[derive(Debug, Clone)]
pub struct ModuleEscapeInfo {
    pub functions: HashMap<String, EscapeInfo>,
}

/// Analyze escape behavior for all functions in a MIR module.
pub fn analyze_module(module: &MirModule) -> ModuleEscapeInfo {
    let mut results = HashMap::new();
    for func in &module.functions {
        let info = analyze_function(func);
        results.insert(func.name.clone(), info);
    }
    ModuleEscapeInfo { functions: results }
}

/// Analyze escape behavior for a single function.
pub fn analyze_function(func: &MirFunction) -> EscapeInfo {
    let mut heap_locals = HashSet::new();
    let mut escaping = HashSet::new();

    // Identify heap-typed locals
    for local in &func.locals {
        if is_heap_type(&local.ty) {
            heap_locals.insert(local.id.0);
        }
    }

    // Also mark params as heap-typed if applicable
    for param in &func.params {
        if is_heap_type(&param.ty) {
            heap_locals.insert(param.id.0);
        }
    }

    // Walk all statements to find escaping locals
    for block in &func.blocks {
        for stmt in &block.stmts {
            collect_escaping_locals(stmt, &mut escaping);
        }
        // Check terminator
        if let Terminator::Return(Some(op)) = &block.terminator {
            collect_escaping_from_operand(op, &mut escaping);
        }
    }

    // Build hints
    let mut hints = HashMap::new();
    let mut stack_eligible = 0usize;
    let mut heap_required = 0usize;

    for &local_id in &heap_locals {
        if escaping.contains(&local_id) {
            hints.insert(local_id, AllocHint::Heap);
            heap_required += 1;
        } else {
            hints.insert(local_id, AllocHint::Stack);
            stack_eligible += 1;
        }
    }

    EscapeInfo {
        hints,
        heap_locals,
        stack_eligible,
        heap_required,
    }
}

fn is_heap_type(ty: &ark_typecheck::types::Type) -> bool {
    use ark_typecheck::types::Type;
    matches!(
        ty,
        Type::String
            | Type::Vec(_)
            | Type::Struct(_)
            | Type::Enum(_)
            | Type::Option(_)
            | Type::Result(_, _)
            | Type::Array(_, _)
            | Type::Slice(_)
            | Type::Tuple(_)
    )
}

fn collect_escaping_locals(stmt: &MirStmt, escaping: &mut HashSet<u32>) {
    match stmt {
        MirStmt::Call { args, .. } => {
            for arg in args {
                collect_escaping_from_operand(arg, escaping);
            }
        }
        MirStmt::CallBuiltin { args, name, .. } => {
            // push, set, insert, etc. capture their arguments
            let capturing = matches!(
                name.as_str(),
                "push"
                    | "set"
                    | "insert"
                    | "__intrinsic_push"
                    | "__intrinsic_set"
                    | "__intrinsic_HashMap_insert_i32"
            );
            if capturing {
                for arg in args {
                    collect_escaping_from_operand(arg, escaping);
                }
            } else {
                // Other builtins: args with heap types passed by value escape
                for arg in args {
                    collect_escaping_from_operand(arg, escaping);
                }
            }
        }
        MirStmt::Assign(_, Rvalue::Use(op)) => {
            // If storing into a field, the value escapes
            collect_struct_store_escapes(op, escaping);
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            collect_escaping_from_operand(cond, escaping);
            for s in then_body {
                collect_escaping_locals(s, escaping);
            }
            for s in else_body {
                collect_escaping_locals(s, escaping);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            collect_escaping_from_operand(cond, escaping);
            for s in body {
                collect_escaping_locals(s, escaping);
            }
        }
        MirStmt::Return(Some(op)) => {
            collect_escaping_from_operand(op, escaping);
        }
        _ => {}
    }
}

fn collect_escaping_from_operand(op: &Operand, escaping: &mut HashSet<u32>) {
    match op {
        Operand::Place(Place::Local(id)) => {
            escaping.insert(id.0);
        }
        Operand::Call(_, args) => {
            for arg in args {
                collect_escaping_from_operand(arg, escaping);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, val) in fields {
                collect_escaping_from_operand(val, escaping);
            }
        }
        Operand::FieldAccess { object, .. } => {
            collect_escaping_from_operand(object, escaping);
        }
        Operand::EnumInit { payload, .. } => {
            for p in payload {
                collect_escaping_from_operand(p, escaping);
            }
        }
        Operand::ArrayInit { elements } => {
            for e in elements {
                collect_escaping_from_operand(e, escaping);
            }
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
            ..
        } => {
            collect_escaping_from_operand(cond, escaping);
            for s in then_body {
                collect_escaping_locals(s, escaping);
            }
            if let Some(r) = then_result {
                collect_escaping_from_operand(r, escaping);
            }
            for s in else_body {
                collect_escaping_locals(s, escaping);
            }
            if let Some(r) = else_result {
                collect_escaping_from_operand(r, escaping);
            }
        }
        _ => {}
    }
}

fn collect_struct_store_escapes(op: &Operand, escaping: &mut HashSet<u32>) {
    // Values used in struct/enum init fields escape
    match op {
        Operand::StructInit { fields, .. } => {
            for (_, val) in fields {
                collect_escaping_from_operand(val, escaping);
            }
        }
        Operand::EnumInit { payload, .. } => {
            for p in payload {
                collect_escaping_from_operand(p, escaping);
            }
        }
        _ => {}
    }
}

/// Format escape analysis results as human-readable text (for --emit metadata).
pub fn format_escape_info(info: &ModuleEscapeInfo) -> String {
    let mut out = String::new();
    out.push_str("=== Escape Analysis ===\n");
    for (fname, finfo) in &info.functions {
        if finfo.heap_locals.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "fn {}: {} heap locals ({} stack-eligible, {} heap-required)\n",
            fname,
            finfo.heap_locals.len(),
            finfo.stack_eligible,
            finfo.heap_required
        ));
        for (&local_id, hint) in &finfo.hints {
            out.push_str(&format!("  local_{}: {:?}\n", local_id, hint));
        }
    }
    out
}
