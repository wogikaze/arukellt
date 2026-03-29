//! Function reachability analysis for the T3 Wasm GC emitter.
//!
//! Determines which MIR functions are transitively reachable from entry points,
//! scans for filesystem-related builtins to minimize WASI imports, and
//! determines which stdlib helper functions are actually needed.

use ark_mir::mir::*;
use ark_typecheck::Type;
use std::collections::{HashMap, HashSet, VecDeque};

use super::{is_component_export_candidate, normalize_intrinsic, Ctx};

/// Tracks which stdlib helper functions are needed by user code.
#[derive(Debug, Default)]
pub(super) struct NeededHelpers {
    pub print_str: bool,
    pub print_i32: bool,
    pub print_bool: bool,
    pub print_str_ln: bool,
    pub print_i32_ln: bool,
    pub print_bool_ln: bool,
    pub i32_to_str: bool,
    pub print_newline: bool,
    pub i64_to_str: bool,
    pub f64_to_str: bool,
    pub parse_i32: bool,
    pub parse_i64: bool,
    pub parse_f64: bool,
}

impl Ctx {
    /// Scan MIR for filesystem builtins (fs_read_file, fs_write_file) to
    /// determine if path_open/fd_read/fd_close WASI imports are needed.
    pub(super) fn mir_uses_fs(mir: &MirModule, reachable: &[usize]) -> bool {
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                if block.stmts.iter().any(Self::stmt_uses_fs)
                    || Self::terminator_uses_fs(&block.terminator)
                {
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn stmt_uses_fs(stmt: &MirStmt) -> bool {
        match stmt {
            MirStmt::CallBuiltin { name, .. } => {
                name == "fs_read_file"
                    || name == "fs_write_file"
                    || name == "__intrinsic_fs_read_file"
                    || name == "__intrinsic_fs_write_file"
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
                if name == "fs_read_file"
                    || name == "fs_write_file"
                    || name == "__intrinsic_fs_read_file"
                    || name == "__intrinsic_fs_write_file"
                {
                    return true;
                }
                args.iter().any(|a| Self::operand_uses_fs(a))
            }
            _ => false,
        }
    }

    fn terminator_uses_fs(terminator: &Terminator) -> bool {
        match terminator {
            Terminator::Return(Some(op)) => Self::operand_uses_fs(op),
            Terminator::If { cond, .. } => Self::operand_uses_fs(cond),
            Terminator::Switch { scrutinee, .. } => Self::operand_uses_fs(scrutinee),
            _ => false,
        }
    }

    // ── Clock reachability ───────────────────────────────────────────

    pub(super) fn mir_uses_clock(mir: &MirModule, reachable: &[usize]) -> bool {
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                if block.stmts.iter().any(Self::stmt_uses_clock)
                    || Self::terminator_uses_clock(&block.terminator)
                {
                    return true;
                }
            }
        }
        false
    }

    fn stmt_uses_clock(stmt: &MirStmt) -> bool {
        match stmt {
            MirStmt::CallBuiltin { name, .. } => {
                name == "clock_now" || name == "clock_now_ms"
                    || name == "__intrinsic_clock_now" || name == "__intrinsic_clock_now_ms"
            }
            MirStmt::Assign(_, rvalue) => Self::rvalue_uses_clock(rvalue),
            MirStmt::IfStmt { cond, then_body, else_body } => {
                Self::operand_uses_clock(cond)
                    || then_body.iter().any(|s| Self::stmt_uses_clock(s))
                    || else_body.iter().any(|s| Self::stmt_uses_clock(s))
            }
            MirStmt::WhileStmt { cond, body } => {
                Self::operand_uses_clock(cond)
                    || body.iter().any(|s| Self::stmt_uses_clock(s))
            }
            MirStmt::Return(Some(op)) => Self::operand_uses_clock(op),
            _ => false,
        }
    }

    fn rvalue_uses_clock(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Use(op) => Self::operand_uses_clock(op),
            Rvalue::BinaryOp(_, l, r) => Self::operand_uses_clock(l) || Self::operand_uses_clock(r),
            Rvalue::UnaryOp(_, op) => Self::operand_uses_clock(op),
            _ => false,
        }
    }

    fn operand_uses_clock(op: &Operand) -> bool {
        match op {
            Operand::Call(name, args) => {
                if name == "clock_now" || name == "clock_now_ms"
                    || name == "__intrinsic_clock_now" || name == "__intrinsic_clock_now_ms"
                {
                    return true;
                }
                args.iter().any(|a| Self::operand_uses_clock(a))
            }
            _ => false,
        }
    }

    fn terminator_uses_clock(terminator: &Terminator) -> bool {
        match terminator {
            Terminator::Return(Some(op)) => Self::operand_uses_clock(op),
            Terminator::If { cond, .. } => Self::operand_uses_clock(cond),
            Terminator::Switch { scrutinee, .. } => Self::operand_uses_clock(scrutinee),
            _ => false,
        }
    }

    // ── Random reachability ──────────────────────────────────────────

    pub(super) fn mir_uses_random(mir: &MirModule, reachable: &[usize]) -> bool {
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                if block.stmts.iter().any(Self::stmt_uses_random)
                    || Self::terminator_uses_random(&block.terminator)
                {
                    return true;
                }
            }
        }
        false
    }

    fn stmt_uses_random(stmt: &MirStmt) -> bool {
        match stmt {
            MirStmt::CallBuiltin { name, .. } => {
                name == "random_i32" || name == "random_f64"
                    || name == "__intrinsic_random_i32" || name == "__intrinsic_random_f64"
            }
            MirStmt::Assign(_, rvalue) => Self::rvalue_uses_random(rvalue),
            MirStmt::IfStmt { cond, then_body, else_body } => {
                Self::operand_uses_random(cond)
                    || then_body.iter().any(|s| Self::stmt_uses_random(s))
                    || else_body.iter().any(|s| Self::stmt_uses_random(s))
            }
            MirStmt::WhileStmt { cond, body } => {
                Self::operand_uses_random(cond)
                    || body.iter().any(|s| Self::stmt_uses_random(s))
            }
            MirStmt::Return(Some(op)) => Self::operand_uses_random(op),
            _ => false,
        }
    }

    fn rvalue_uses_random(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Use(op) => Self::operand_uses_random(op),
            Rvalue::BinaryOp(_, l, r) => Self::operand_uses_random(l) || Self::operand_uses_random(r),
            Rvalue::UnaryOp(_, op) => Self::operand_uses_random(op),
            _ => false,
        }
    }

    fn operand_uses_random(op: &Operand) -> bool {
        match op {
            Operand::Call(name, args) => {
                if name == "random_i32" || name == "random_f64"
                    || name == "__intrinsic_random_i32" || name == "__intrinsic_random_f64"
                {
                    return true;
                }
                args.iter().any(|a| Self::operand_uses_random(a))
            }
            _ => false,
        }
    }

    fn terminator_uses_random(terminator: &Terminator) -> bool {
        match terminator {
            Terminator::Return(Some(op)) => Self::operand_uses_random(op),
            Terminator::If { cond, .. } => Self::operand_uses_random(cond),
            Terminator::Switch { scrutinee, .. } => Self::operand_uses_random(scrutinee),
            _ => false,
        }
    }

    // ── proc_exit reachability ───────────────────────────────────────

    pub(super) fn mir_uses_proc_exit(mir: &MirModule, reachable: &[usize]) -> bool {
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                if block.stmts.iter().any(Self::stmt_uses_proc_exit)
                    || Self::terminator_uses_proc_exit(&block.terminator)
                {
                    return true;
                }
            }
        }
        false
    }

    fn stmt_uses_proc_exit(stmt: &MirStmt) -> bool {
        match stmt {
            MirStmt::CallBuiltin { name, .. } => {
                name == "exit" || name == "proc_exit"
                    || name == "__intrinsic_exit" || name == "__intrinsic_proc_exit"
            }
            MirStmt::Assign(_, rvalue) => Self::rvalue_uses_proc_exit(rvalue),
            MirStmt::IfStmt { cond, then_body, else_body } => {
                Self::operand_uses_proc_exit(cond)
                    || then_body.iter().any(|s| Self::stmt_uses_proc_exit(s))
                    || else_body.iter().any(|s| Self::stmt_uses_proc_exit(s))
            }
            MirStmt::WhileStmt { cond, body } => {
                Self::operand_uses_proc_exit(cond)
                    || body.iter().any(|s| Self::stmt_uses_proc_exit(s))
            }
            MirStmt::Return(Some(op)) => Self::operand_uses_proc_exit(op),
            _ => false,
        }
    }

    fn terminator_uses_proc_exit(terminator: &Terminator) -> bool {
        match terminator {
            Terminator::Return(Some(op)) => Self::operand_uses_proc_exit(op),
            Terminator::If { cond, .. } => Self::operand_uses_proc_exit(cond),
            Terminator::Switch { scrutinee, .. } => Self::operand_uses_proc_exit(scrutinee),
            _ => false,
        }
    }

    fn rvalue_uses_proc_exit(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Use(op) => Self::operand_uses_proc_exit(op),
            Rvalue::BinaryOp(_, l, r) => Self::operand_uses_proc_exit(l) || Self::operand_uses_proc_exit(r),
            Rvalue::UnaryOp(_, op) => Self::operand_uses_proc_exit(op),
            _ => false,
        }
    }

    fn operand_uses_proc_exit(op: &Operand) -> bool {
        match op {
            Operand::Call(name, args) => {
                if name == "exit" || name == "proc_exit"
                    || name == "__intrinsic_exit" || name == "__intrinsic_proc_exit"
                {
                    return true;
                }
                args.iter().any(|a| Self::operand_uses_proc_exit(a))
            }
            _ => false,
        }
    }

    // ── args reachability ────────────────────────────────────────────

    pub(super) fn mir_uses_args(mir: &MirModule, reachable: &[usize]) -> bool {
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                for stmt in &block.stmts {
                    if Self::stmt_uses_args(stmt) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn stmt_uses_args(stmt: &MirStmt) -> bool {
        match stmt {
            MirStmt::CallBuiltin { name, .. } => {
                name == "args" || name == "__intrinsic_args"
            }
            MirStmt::Assign(_, rvalue) => Self::rvalue_uses_args(rvalue),
            MirStmt::IfStmt { cond, then_body, else_body } => {
                Self::operand_uses_args(cond)
                    || then_body.iter().any(|s| Self::stmt_uses_args(s))
                    || else_body.iter().any(|s| Self::stmt_uses_args(s))
            }
            MirStmt::WhileStmt { cond, body } => {
                Self::operand_uses_args(cond)
                    || body.iter().any(|s| Self::stmt_uses_args(s))
            }
            MirStmt::Return(Some(op)) => Self::operand_uses_args(op),
            _ => false,
        }
    }

    fn rvalue_uses_args(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Use(op) => Self::operand_uses_args(op),
            Rvalue::BinaryOp(_, l, r) => Self::operand_uses_args(l) || Self::operand_uses_args(r),
            Rvalue::UnaryOp(_, op) => Self::operand_uses_args(op),
            _ => false,
        }
    }

    fn operand_uses_args(op: &Operand) -> bool {
        match op {
            Operand::Call(name, args) => {
                if name == "args" || name == "__intrinsic_args" {
                    return true;
                }
                args.iter().any(|a| Self::operand_uses_args(a))
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

        for (idx, func) in mir.functions.iter().enumerate() {
            if func.is_exported && is_component_export_candidate(&func.name) {
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

    /// Scan reachable MIR functions to determine which stdlib helpers are needed.
    pub(super) fn scan_needed_helpers(
        mir: &MirModule,
        reachable: &[usize],
    ) -> NeededHelpers {
        let mut needed = NeededHelpers::default();
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                for stmt in &block.stmts {
                    Self::scan_stmt_for_helpers(stmt, func, &mut needed);
                }
                Self::scan_terminator_for_helpers(&block.terminator, func, &mut needed);
            }
        }
        // Inter-helper deps: __print_i32_ln internally calls __i32_to_str
        if needed.print_i32_ln {
            needed.i32_to_str = true;
        }
        needed
    }

    fn scan_stmt_for_helpers(stmt: &MirStmt, func: &MirFunction, needed: &mut NeededHelpers) {
        match stmt {
            MirStmt::CallBuiltin { name, args, .. } => {
                let canonical = normalize_intrinsic(name);
                Self::mark_builtin_helpers(canonical, args, func, needed);
                for arg in args {
                    Self::scan_operand_for_helpers(arg, func, needed);
                }
            }
            MirStmt::Assign(_, rvalue) => {
                Self::scan_rvalue_for_helpers(rvalue, func, needed);
            }
            MirStmt::Call { args, .. } => {
                for arg in args {
                    Self::scan_operand_for_helpers(arg, func, needed);
                }
            }
            MirStmt::IfStmt { cond, then_body, else_body } => {
                Self::scan_operand_for_helpers(cond, func, needed);
                for s in then_body { Self::scan_stmt_for_helpers(s, func, needed); }
                for s in else_body { Self::scan_stmt_for_helpers(s, func, needed); }
            }
            MirStmt::WhileStmt { cond, body } => {
                Self::scan_operand_for_helpers(cond, func, needed);
                for s in body { Self::scan_stmt_for_helpers(s, func, needed); }
            }
            MirStmt::Return(Some(op)) => {
                Self::scan_operand_for_helpers(op, func, needed);
            }
            _ => {}
        }
    }

    fn scan_terminator_for_helpers(term: &Terminator, func: &MirFunction, needed: &mut NeededHelpers) {
        match term {
            Terminator::Return(Some(op)) | Terminator::If { cond: op, .. }
            | Terminator::Switch { scrutinee: op, .. } => {
                Self::scan_operand_for_helpers(op, func, needed);
            }
            _ => {}
        }
    }

    fn scan_rvalue_for_helpers(rvalue: &Rvalue, func: &MirFunction, needed: &mut NeededHelpers) {
        match rvalue {
            Rvalue::Use(op) | Rvalue::UnaryOp(_, op) => {
                Self::scan_operand_for_helpers(op, func, needed);
            }
            Rvalue::BinaryOp(_, l, r) => {
                Self::scan_operand_for_helpers(l, func, needed);
                Self::scan_operand_for_helpers(r, func, needed);
            }
            Rvalue::Aggregate(_, ops) => {
                for op in ops { Self::scan_operand_for_helpers(op, func, needed); }
            }
            _ => {}
        }
    }

    fn scan_operand_for_helpers(op: &Operand, func: &MirFunction, needed: &mut NeededHelpers) {
        match op {
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic(name);
                Self::mark_builtin_helpers(canonical, args, func, needed);
                for a in args { Self::scan_operand_for_helpers(a, func, needed); }
            }
            Operand::BinOp(_, l, r) => {
                Self::scan_operand_for_helpers(l, func, needed);
                Self::scan_operand_for_helpers(r, func, needed);
            }
            Operand::UnaryOp(_, inner) | Operand::EnumTag(inner)
            | Operand::TryExpr { expr: inner, .. } => {
                Self::scan_operand_for_helpers(inner, func, needed);
            }
            Operand::IfExpr { cond, then_body, then_result, else_body, else_result } => {
                Self::scan_operand_for_helpers(cond, func, needed);
                for s in then_body { Self::scan_stmt_for_helpers(s, func, needed); }
                if let Some(r) = then_result { Self::scan_operand_for_helpers(r, func, needed); }
                for s in else_body { Self::scan_stmt_for_helpers(s, func, needed); }
                if let Some(r) = else_result { Self::scan_operand_for_helpers(r, func, needed); }
            }
            Operand::StructInit { fields, .. } => {
                for (_, f) in fields { Self::scan_operand_for_helpers(f, func, needed); }
            }
            Operand::FieldAccess { object, .. } | Operand::EnumPayload { object, .. } => {
                Self::scan_operand_for_helpers(object, func, needed);
            }
            Operand::EnumInit { payload, .. } | Operand::ArrayInit { elements: payload } => {
                for p in payload { Self::scan_operand_for_helpers(p, func, needed); }
            }
            Operand::LoopExpr { init, body, result } => {
                Self::scan_operand_for_helpers(init, func, needed);
                for s in body { Self::scan_stmt_for_helpers(s, func, needed); }
                Self::scan_operand_for_helpers(result, func, needed);
            }
            Operand::CallIndirect { callee, args } => {
                Self::scan_operand_for_helpers(callee, func, needed);
                for a in args { Self::scan_operand_for_helpers(a, func, needed); }
            }
            Operand::IndexAccess { object, index } => {
                Self::scan_operand_for_helpers(object, func, needed);
                Self::scan_operand_for_helpers(index, func, needed);
            }
            _ => {}
        }
    }

    /// Map a builtin name + its args to which helpers it requires.
    fn mark_builtin_helpers(
        canonical: &str,
        args: &[Operand],
        func: &MirFunction,
        needed: &mut NeededHelpers,
    ) {
        match canonical {
            "println" => {
                if let Some(arg) = args.first() {
                    match Self::infer_arg_category(arg, func) {
                        ArgCategory::String => needed.print_str_ln = true,
                        ArgCategory::Bool => needed.print_bool_ln = true,
                        ArgCategory::I64 | ArgCategory::F64 | ArgCategory::I32 => {
                            needed.print_i32_ln = true;
                        }
                        ArgCategory::Unknown => {
                            needed.print_str_ln = true;
                            needed.print_i32_ln = true;
                            needed.print_bool_ln = true;
                        }
                    }
                }
            }
            "print" => {
                if let Some(arg) = args.first() {
                    match Self::infer_arg_category(arg, func) {
                        ArgCategory::String => needed.print_str = true,
                        ArgCategory::Bool => needed.print_bool = true,
                        ArgCategory::I64 | ArgCategory::F64 | ArgCategory::I32 => {
                            needed.print_i32 = true;
                        }
                        ArgCategory::Unknown => {
                            needed.print_str = true;
                            needed.print_i32 = true;
                            needed.print_bool = true;
                            needed.print_str_ln = true;
                            needed.print_i32_ln = true;
                            needed.print_bool_ln = true;
                        }
                    }
                }
            }
            "i32_to_string" | "bool_to_string" | "char_to_string" => needed.i32_to_str = true,
            "i64_to_string" => needed.i64_to_str = true,
            "f64_to_string" => needed.f64_to_str = true,
            // String interpolation (concat with non-string args) needs to_str helpers
            "concat" | "join" | "string_interpolation" => {
                for arg in args {
                    match Self::infer_arg_category(arg, func) {
                        ArgCategory::I32 => needed.i32_to_str = true,
                        ArgCategory::I64 => needed.i64_to_str = true,
                        ArgCategory::F64 => needed.f64_to_str = true,
                        _ => {}
                    }
                }
            }
            // to_string dispatches based on arg type at emit time
            "to_string" => {
                if let Some(arg) = args.first() {
                    match Self::infer_arg_category(arg, func) {
                        ArgCategory::I32 => needed.i32_to_str = true,
                        ArgCategory::Bool => needed.i32_to_str = true,
                        ArgCategory::I64 => needed.i64_to_str = true,
                        ArgCategory::F64 => needed.f64_to_str = true,
                        ArgCategory::String => {} // no helper needed
                        ArgCategory::Unknown => {
                            // Conservative: include all to_str helpers
                            needed.i32_to_str = true;
                            needed.i64_to_str = true;
                            needed.f64_to_str = true;
                        }
                    }
                }
            }
            "parse_i32" => needed.parse_i32 = true,
            "parse_i64" => needed.parse_i64 = true,
            "parse_f64" => needed.parse_f64 = true,
            _ => {}
        }
    }

    fn infer_arg_category(op: &Operand, func: &MirFunction) -> ArgCategory {
        match op {
            Operand::ConstString(_) => ArgCategory::String,
            Operand::ConstBool(_) => ArgCategory::Bool,
            Operand::ConstI32(_) | Operand::ConstU8(_) | Operand::ConstU16(_)
            | Operand::ConstU32(_) | Operand::ConstI8(_) | Operand::ConstI16(_)
            | Operand::ConstChar(_) => ArgCategory::I32,
            Operand::ConstI64(_) | Operand::ConstU64(_) => ArgCategory::I64,
            Operand::ConstF32(_) | Operand::ConstF64(_) => ArgCategory::F64,
            Operand::Place(Place::Local(id)) => {
                if let Some(local) = func.locals.iter().find(|l| l.id == *id) {
                    Self::type_to_category(&local.ty)
                } else if let Some(param) = func.params.iter().find(|p| p.id == *id) {
                    Self::type_to_category(&param.ty)
                } else {
                    ArgCategory::Unknown
                }
            }
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                match canonical {
                    "concat" | "join" | "i32_to_string" | "i64_to_string"
                    | "f64_to_string" | "bool_to_string" | "to_string"
                    | "fs_read_file" | "trim" | "substring" | "replace"
                    | "to_uppercase" | "to_lowercase" | "repeat" | "char_at" => ArgCategory::String,
                    "eq" | "starts_with" | "ends_with" | "contains"
                    | "assert" | "assert_eq" | "contains_i32" | "contains_String" => ArgCategory::Bool,
                    _ => ArgCategory::Unknown,
                }
            }
            Operand::FieldAccess { .. } | Operand::EnumPayload { .. } => ArgCategory::Unknown,
            Operand::IfExpr { then_result, .. } => {
                if let Some(r) = then_result {
                    Self::infer_arg_category(r, func)
                } else {
                    ArgCategory::Unknown
                }
            }
            _ => ArgCategory::Unknown,
        }
    }

    fn type_to_category(ty: &Type) -> ArgCategory {
        match ty {
            Type::String => ArgCategory::String,
            Type::Bool => ArgCategory::Bool,
            Type::I64 | Type::U64 => ArgCategory::I64,
            Type::F64 | Type::F32 => ArgCategory::F64,
            _ => ArgCategory::I32,
        }
    }
}

enum ArgCategory {
    String,
    Bool,
    I32,
    I64,
    F64,
    Unknown,
}
