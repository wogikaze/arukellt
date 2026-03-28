//! Function reachability analysis for the T3 Wasm GC emitter.
//!
//! Determines which MIR functions are transitively reachable from entry points,
//! and scans for filesystem-related builtins to minimize WASI imports.

use ark_mir::mir::*;
use std::collections::{HashMap, HashSet, VecDeque};

use super::{normalize_intrinsic, Ctx};

impl Ctx {
    /// Scan MIR for filesystem builtins (fs_read_file, fs_write_file) to
    /// determine if path_open/fd_read/fd_close WASI imports are needed.
    pub(super) fn mir_uses_fs(mir: &MirModule, reachable: &[usize]) -> bool {
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                for stmt in &block.stmts {
                    if Self::stmt_uses_fs(stmt) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub(super) fn stmt_uses_fs(stmt: &MirStmt) -> bool {
        match stmt {
            MirStmt::CallBuiltin { name, .. } => {
                name == "fs_read_file" || name == "fs_write_file"
            }
            MirStmt::Assign(_, rvalue) => Self::rvalue_uses_fs(rvalue),
            MirStmt::IfStmt { cond, then_body, else_body } => {
                Self::operand_uses_fs(cond)
                    || then_body.iter().any(|s| Self::stmt_uses_fs(s))
                    || else_body.iter().any(|s| Self::stmt_uses_fs(s))
            }
            MirStmt::WhileStmt { cond, body } => {
                Self::operand_uses_fs(cond)
                    || body.iter().any(|s| Self::stmt_uses_fs(s))
            }
            MirStmt::Return(Some(op)) => Self::operand_uses_fs(op),
            _ => false,
        }
    }

    pub(super) fn rvalue_uses_fs(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Use(op) => Self::operand_uses_fs(op),
            Rvalue::BinaryOp(_, l, r) => Self::operand_uses_fs(l) || Self::operand_uses_fs(r),
            Rvalue::UnaryOp(_, op) => Self::operand_uses_fs(op),
            _ => false,
        }
    }

    pub(super) fn operand_uses_fs(op: &Operand) -> bool {
        match op {
            Operand::Call(name, args) => {
                if name == "fs_read_file" || name == "fs_write_file" {
                    return true;
                }
                args.iter().any(|a| Self::operand_uses_fs(a))
            }
            _ => false,
        }
    }

    pub(super) fn reachable_function_indices(&self, mir: &MirModule) -> Vec<usize> {
        let mut name_to_idx = HashMap::new();
        for (idx, func) in mir.functions.iter().enumerate() {
            name_to_idx.insert(func.name.as_str(), idx);
        }

        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        let push_root =
            |idx: usize, reachable: &mut HashSet<usize>, queue: &mut VecDeque<usize>| {
                if reachable.insert(idx) {
                    queue.push_back(idx);
                }
            };

        if let Some(entry) = mir.entry_fn {
            push_root(entry.0 as usize, &mut reachable, &mut queue);
        }
        if queue.is_empty() {
            for root_name in ["_start", "main"] {
                if let Some(&idx) = name_to_idx.get(root_name) {
                    push_root(idx, &mut reachable, &mut queue);
                }
            }
        }

        // Exported functions are also roots (for Component Model exports)
        for (idx, func) in mir.functions.iter().enumerate() {
            if func.is_exported && func.name != "main" && !func.name.starts_with("__") {
                push_root(idx, &mut reachable, &mut queue);
            }
        }

        while let Some(func_idx) = queue.pop_front() {
            let func = &mir.functions[func_idx];
            self.collect_reachable_from_function(func, &name_to_idx, &mut reachable, &mut queue);
        }

        let mut ordered: Vec<_> = reachable.into_iter().collect();
        ordered.sort_unstable();
        ordered
    }

    pub(super) fn collect_reachable_from_function(
        &self,
        func: &MirFunction,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        for block in &func.blocks {
            for stmt in &block.stmts {
                self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
            }
            self.collect_reachable_from_terminator(
                &block.terminator,
                name_to_idx,
                reachable,
                queue,
            );
        }
    }

    pub(super) fn collect_reachable_from_stmt(
        &self,
        stmt: &MirStmt,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match stmt {
            MirStmt::Assign(place, rvalue) => {
                self.collect_reachable_from_place(place, name_to_idx, reachable, queue);
                self.collect_reachable_from_rvalue(rvalue, name_to_idx, reachable, queue);
            }
            MirStmt::Call { func, args, .. } => {
                self.push_reachable_fn(func.0 as usize, reachable, queue);
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            MirStmt::CallBuiltin { name, args, .. } => {
                let canonical = normalize_intrinsic(name);
                if let Some(&idx) = name_to_idx.get(canonical) {
                    self.push_reachable_fn(idx, reachable, queue);
                }
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
                for stmt in then_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                for stmt in else_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
            }
            MirStmt::WhileStmt { cond, body } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
                for stmt in body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
            }
            MirStmt::Return(Some(op)) => {
                self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
            }
            MirStmt::Break | MirStmt::Continue | MirStmt::Return(None) | MirStmt::GcHint { .. } => {}
        }
    }

    pub(super) fn collect_reachable_from_terminator(
        &self,
        terminator: &Terminator,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match terminator {
            Terminator::If { cond, .. } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
            }
            Terminator::Switch { scrutinee, .. } => {
                self.collect_reachable_from_operand(scrutinee, name_to_idx, reachable, queue);
            }
            Terminator::Return(Some(op)) => {
                self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    pub(super) fn collect_reachable_from_rvalue(
        &self,
        rvalue: &Rvalue,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match rvalue {
            Rvalue::Use(op) | Rvalue::UnaryOp(_, op) => {
                self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
            }
            Rvalue::BinaryOp(_, lhs, rhs) => {
                self.collect_reachable_from_operand(lhs, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(rhs, name_to_idx, reachable, queue);
            }
            Rvalue::Aggregate(_, ops) => {
                for op in ops {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Rvalue::Ref(place) => {
                self.collect_reachable_from_place(place, name_to_idx, reachable, queue);
            }
        }
    }

    pub(super) fn collect_reachable_from_place(
        &self,
        place: &Place,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match place {
            Place::Local(_) => {}
            Place::Field(inner, _) => {
                self.collect_reachable_from_place(inner, name_to_idx, reachable, queue);
            }
            Place::Index(inner, index) => {
                self.collect_reachable_from_place(inner, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(index, name_to_idx, reachable, queue);
            }
        }
    }

    pub(super) fn collect_reachable_from_operand(
        &self,
        operand: &Operand,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match operand {
            Operand::Place(place) => {
                self.collect_reachable_from_place(place, name_to_idx, reachable, queue);
            }
            Operand::BinOp(_, lhs, rhs) => {
                self.collect_reachable_from_operand(lhs, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(rhs, name_to_idx, reachable, queue);
            }
            Operand::UnaryOp(_, inner)
            | Operand::EnumTag(inner)
            | Operand::TryExpr { expr: inner, .. } => {
                self.collect_reachable_from_operand(inner, name_to_idx, reachable, queue);
            }
            Operand::Call(name, args) => {
                if let Some(&idx) = name_to_idx.get(name.as_str()) {
                    self.push_reachable_fn(idx, reachable, queue);
                }
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            Operand::IfExpr {
                cond,
                then_body,
                then_result,
                else_body,
                else_result,
            } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
                for stmt in then_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                if let Some(op) = then_result {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
                for stmt in else_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                if let Some(op) = else_result {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::StructInit { fields, .. } => {
                for (_, op) in fields {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::FieldAccess { object, .. } => {
                self.collect_reachable_from_operand(object, name_to_idx, reachable, queue);
            }
            Operand::EnumInit { payload, .. } => {
                for op in payload {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::EnumPayload { object, .. } => {
                self.collect_reachable_from_operand(object, name_to_idx, reachable, queue);
            }
            Operand::LoopExpr { init, body, result } => {
                self.collect_reachable_from_operand(init, name_to_idx, reachable, queue);
                for stmt in body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                self.collect_reachable_from_operand(result, name_to_idx, reachable, queue);
            }
            Operand::FnRef(name) => {
                if let Some(&idx) = name_to_idx.get(name.as_str()) {
                    self.push_reachable_fn(idx, reachable, queue);
                }
            }
            Operand::CallIndirect { callee, args } => {
                self.collect_reachable_from_operand(callee, name_to_idx, reachable, queue);
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            Operand::ArrayInit { elements } => {
                for op in elements {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::IndexAccess { object, index } => {
                self.collect_reachable_from_operand(object, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(index, name_to_idx, reachable, queue);
            }
            Operand::ConstI32(_)
            | Operand::ConstI64(_)
            | Operand::ConstF32(_)
            | Operand::ConstF64(_)
            | Operand::ConstU8(_)
            | Operand::ConstU16(_)
            | Operand::ConstU32(_)
            | Operand::ConstU64(_)
            | Operand::ConstI8(_)
            | Operand::ConstI16(_)
            | Operand::ConstBool(_)
            | Operand::ConstChar(_)
            | Operand::ConstString(_)
            | Operand::Unit => {}
        }
    }

    pub(super) fn push_reachable_fn(
        &self,
        idx: usize,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        if reachable.insert(idx) {
            queue.push_back(idx);
        }
    }
}
