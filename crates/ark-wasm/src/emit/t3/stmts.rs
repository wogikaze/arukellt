//! Statement emission for the T3 Wasm GC emitter.
//!
//! Handles MIR statement lowering to Wasm instructions, including
//! assignment, control flow, and builtin call dispatch.

use ark_mir::mir::*;
use ark_typecheck::types::Type;
use wasm_encoder::{HeapType, Instruction, MemArg, RefType as WasmRefType, ValType};

use super::peephole::PeepholeWriter;
use super::{Ctx, SCRATCH, normalize_intrinsic, ref_nullable};

impl Ctx {
    pub(super) fn emit_stmt(&mut self, f: &mut PeepholeWriter<'_>, stmt: &MirStmt) {
        match stmt {
            MirStmt::Assign(Place::Local(id), Rvalue::Use(op)) => {
                self.emit_operand(f, op);
                // Unbox anyref from __tupleN_any FieldAccess to the destination local's concrete type
                if let Operand::FieldAccess { struct_name, .. } = op
                    && struct_name.starts_with("__tuple")
                    && struct_name.ends_with("_any")
                {
                    if self.string_locals.contains(&id.0) {
                        f.instruction(&Instruction::RefCastNullable(HeapType::Concrete(
                            self.string_ty,
                        )));
                    } else {
                        // Default: i32 — unbox from i31ref
                        f.instruction(&Instruction::RefCastNullable(HeapType::Abstract {
                            shared: false,
                            ty: wasm_encoder::AbstractHeapType::I31,
                        }));
                        f.instruction(&Instruction::I31GetS);
                    }
                }
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(Place::Field(inner, field_name), Rvalue::Use(op)) => {
                // struct.set: obj_ref field_idx value
                if let Place::Local(id) = inner.as_ref() {
                    let struct_name = self.local_struct.get(&id.0).cloned();
                    if let Some(ref sname) = struct_name
                        && let Some(&ty_idx) = self.struct_gc_types.get(sname)
                    {
                        let field_idx = self
                            .struct_layouts
                            .get(sname)
                            .and_then(|fields| fields.iter().position(|(n, _)| n == field_name))
                            .unwrap_or(0) as u32;
                        let local_idx = self.local_wasm_idx(id.0);
                        f.instruction(&Instruction::LocalGet(local_idx));
                        self.emit_operand(f, op);
                        f.instruction(&Instruction::StructSet {
                            struct_type_index: ty_idx,
                            field_index: field_idx,
                        });
                        return;
                    }
                }
                // Fallback: drop
                self.emit_operand(f, op);
                f.instruction(&Instruction::Drop);
            }
            MirStmt::Assign(_place, Rvalue::Use(op)) => {
                self.emit_operand(f, op);
                // For non-local places, just drop for now
                f.instruction(&Instruction::Drop);
            }
            MirStmt::Assign(Place::Local(id), Rvalue::BinaryOp(op, lhs, rhs)) => {
                let lhs_i64 = self.is_i64_like_operand(lhs);
                let lhs_f64 = self.is_f64_like_operand(lhs);
                let rhs_i64 = self.is_i64_like_operand(rhs);
                let rhs_f64 = self.is_f64_like_operand(rhs);
                let need_i64 = lhs_i64 || rhs_i64;
                let need_f64 = lhs_f64 || rhs_f64;
                let lhs_str = self.is_string_like_operand(lhs);
                if lhs_str && matches!(op, BinOp::Eq | BinOp::Ne) {
                    // String comparison: byte-by-byte GC array compare
                    self.emit_operand(f, lhs);
                    self.emit_operand(f, rhs);
                    self.emit_string_eq_gc(f);
                    if *op == BinOp::Ne {
                        f.instruction(&Instruction::I32Eqz);
                    }
                } else {
                    self.emit_operand_coerced(f, lhs, need_i64, need_f64);
                    self.emit_operand_coerced(f, rhs, need_i64, need_f64);
                    self.emit_binop(f, *op, Some(lhs));
                }
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(Place::Local(id), Rvalue::UnaryOp(op, inner)) => {
                self.emit_operand(f, inner);
                let is_f64 = self.f64_locals.contains(&id.0) || self.is_f64_like_operand(inner);
                let is_i64 = self.i64_locals.contains(&id.0) || self.is_i64_like_operand(inner);
                match op {
                    UnaryOp::Neg => {
                        if is_f64 {
                            f.instruction(&Instruction::F64Neg);
                        } else if is_i64 {
                            let scratch = self.si(6);
                            f.instruction(&Instruction::LocalSet(scratch));
                            f.instruction(&Instruction::I64Const(0));
                            f.instruction(&Instruction::LocalGet(scratch));
                            f.instruction(&Instruction::I64Sub);
                        } else {
                            f.instruction(&Instruction::I32Const(-1));
                            f.instruction(&Instruction::I32Mul);
                        }
                    }
                    UnaryOp::Not | UnaryOp::BitNot => {
                        f.instruction(&Instruction::I32Const(-1));
                        f.instruction(&Instruction::I32Xor);
                    }
                    UnaryOp::SignExtend8 => {
                        if is_i64 {
                            f.instruction(&Instruction::I64Extend8S);
                        } else {
                            f.instruction(&Instruction::I32Extend8S);
                        }
                    }
                    UnaryOp::SignExtend16 => {
                        if is_i64 {
                            f.instruction(&Instruction::I64Extend16S);
                        } else {
                            f.instruction(&Instruction::I32Extend16S);
                        }
                    }
                    UnaryOp::SignExtend32 => {
                        f.instruction(&Instruction::I64Extend32S);
                    }
                }
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(_, _) => {
                // Other rvalue patterns — skip
            }
            MirStmt::CallBuiltin { dest, name, args } => {
                let canonical = normalize_intrinsic(name);
                self.emit_call_builtin(f, canonical, args, dest.as_ref());
            }
            MirStmt::Call {
                dest,
                func: fn_id,
                args,
            } => {
                // fn_id.0 is the MIR function index into mir.functions
                let fn_idx_mir = fn_id.0 as usize;
                if let Some(fn_name) = self.fn_names.get(fn_idx_mir).cloned() {
                    // Check if this is a stdlib wrapper function — redirect to CallBuiltin path
                    let canonical = normalize_intrinsic(&fn_name);
                    let lookup_name = fn_name.rsplit("::").next().unwrap_or(fn_name.as_str());
                    // Host-module builtins (e.g. env::arg_count) are qualified with "::" but
                    // must still be inlined rather than forwarded to a separate function.
                    let is_lookup_builtin = self.is_builtin_name(lookup_name);
                    let prefer_user_fn = fn_name.contains("::") && !is_lookup_builtin;
                    let effective_builtin = if self.is_builtin_name(canonical) {
                        canonical
                    } else {
                        lookup_name
                    };
                    let is_http_wrapper = self.http_wrapper_fns.contains(&fn_name);
                    if (self.is_builtin_name(canonical) || is_lookup_builtin)
                        && !prefer_user_fn
                        && !is_http_wrapper
                    {
                        self.emit_call_builtin(f, effective_builtin, args, dest.as_ref());
                    } else {
                        let param_types = self
                            .fn_param_types
                            .get(&fn_name)
                            .or_else(|| self.fn_param_types.get(lookup_name))
                            .cloned();
                        for (i, arg) in args.iter().enumerate() {
                            let need_i64 = param_types
                                .as_ref()
                                .and_then(|pt| pt.get(i))
                                .is_some_and(|t| matches!(t, Type::I64));
                            let need_f64 = param_types
                                .as_ref()
                                .and_then(|pt| pt.get(i))
                                .is_some_and(|t| matches!(t, Type::F64));
                            let need_any = param_types
                                .as_ref()
                                .and_then(|pt| pt.get(i))
                                .is_some_and(|t| matches!(t, Type::Any));
                            self.emit_operand_coerced(f, arg, need_i64, need_f64);
                            // Box i32/bool → ref.i31 for anyref params
                            if need_any {
                                let arg_vt = self.infer_operand_type(arg);
                                if arg_vt == ValType::I32 {
                                    f.instruction(&Instruction::RefI31);
                                }
                            }
                        }
                        if let Some(&fn_idx) = self
                            .fn_map
                            .get(&fn_name)
                            .or_else(|| self.fn_map.get(lookup_name))
                        {
                            f.instruction(&Instruction::Call(fn_idx));
                        }
                        // Unbox anyref return if needed
                        let ret_ty = self
                            .fn_ret_types
                            .get(&fn_name)
                            .or_else(|| self.fn_ret_types.get(lookup_name))
                            .cloned();
                        if let Some(ref rt) = ret_ty
                            && *rt == Type::Any
                            && dest.is_some()
                        {
                            let concrete = self.infer_generic_return_type(&fn_name, args);
                            self.emit_anyref_unbox(f, &concrete);
                        }
                        if let Some(Place::Local(id)) = dest.as_ref() {
                            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                        }
                    }
                }
            }
            MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                // Const-if elimination: skip the branch structure for constant conditions
                if let Operand::ConstBool(value) = cond {
                    let body = if *value { then_body } else { else_body };
                    for s in body {
                        self.emit_stmt(f, s);
                    }
                    return;
                }
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                self.loop_break_extra_depth += 1;
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                if !else_body.is_empty() {
                    f.instruction(&Instruction::Else);
                    for s in else_body {
                        self.emit_stmt(f, s);
                    }
                }
                self.loop_break_extra_depth -= 1;
                f.instruction(&Instruction::End);
            }
            MirStmt::WhileStmt { cond, body } => {
                let saved_depth = self.loop_break_extra_depth;
                self.loop_break_extra_depth = 0;
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.emit_operand(f, cond);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1)); // break to block
                for s in body {
                    self.emit_stmt(f, s);
                }
                f.instruction(&Instruction::Br(0)); // continue loop
                f.instruction(&Instruction::End); // loop
                f.instruction(&Instruction::End); // block
                self.loop_break_extra_depth = saved_depth;
            }
            MirStmt::Break => {
                // br targets the outer block of the while loop
                // depth: block(outer) > loop > [if*] > here
                f.instruction(&Instruction::Br(1 + self.loop_break_extra_depth));
            }
            MirStmt::Continue => {
                // br targets the loop (beginning)
                f.instruction(&Instruction::Br(self.loop_break_extra_depth));
            }
            MirStmt::Return(Some(op)) => {
                // ── Tail-call optimisation (return_call) ──
                // When opt_level >= 1 and the returned value is a direct call
                // whose Wasm return type matches the current function, emit
                // `return_call` instead of `call` + `return`.
                if self.opt_level >= 1 && !self.is_start_fn {
                    if let Operand::Call(name, args) = op {
                        let canonical = normalize_intrinsic(name).to_string();
                        if !self.is_builtin_name(&canonical) {
                            let callee_ret_is_any = self
                                .fn_ret_types
                                .get(canonical.as_str())
                                .is_some_and(|t| *t == Type::Any);
                            let current_ret_is_any = self.current_fn_return_ty == Type::Any;
                            // return_call is valid only when no boxing/unboxing
                            // is needed between the callee result and our return.
                            if callee_ret_is_any == current_ret_is_any
                                && let Some(&fn_idx) = self.fn_map.get(canonical.as_str())
                            {
                                let param_types =
                                    self.fn_param_types.get(canonical.as_str()).cloned();
                                for (i, arg) in args.iter().enumerate() {
                                    self.emit_operand(f, arg);
                                    if let Some(ref pts) = param_types
                                        && i < pts.len()
                                        && pts[i] == Type::Any
                                    {
                                        let arg_vt = self.infer_operand_type(arg);
                                        if arg_vt == ValType::I32 {
                                            f.instruction(&Instruction::RefI31);
                                        }
                                    }
                                }
                                f.instruction(&Instruction::ReturnCall(fn_idx));
                                return;
                            }
                        }
                    }
                    if let Operand::CallIndirect { callee, args } = op
                        && self.current_fn_return_ty != Type::Any
                    {
                        for arg in args {
                            self.emit_operand(f, arg);
                        }
                        self.emit_operand(f, callee);
                        let params: Vec<ValType> = args
                            .iter()
                            .map(|a| {
                                if self.is_f64_like_operand(a) {
                                    ValType::F64
                                } else if self.is_i64_like_operand(a) {
                                    ValType::I64
                                } else {
                                    ValType::I32
                                }
                            })
                            .collect();
                        let results = vec![ValType::I32];
                        let type_index = self
                            .indirect_types
                            .get(&(params, results))
                            .copied()
                            .unwrap_or(0);
                        f.instruction(&Instruction::ReturnCallIndirect {
                            type_index,
                            table_index: 0,
                        });
                        return;
                    }
                }
                // ── Normal (non-tail-call) path ──
                if self.is_start_fn && !matches!(op, Operand::Unit) {
                    self.emit_operand(f, op);
                    if self.operand_produces_value(op) {
                        f.instruction(&Instruction::Drop);
                    }
                } else {
                    self.emit_operand(f, op);
                    // Box value types when returning from generic function with anyref return
                    if self.current_fn_return_ty == Type::Any {
                        let op_vt = self.infer_operand_type(op);
                        if op_vt == ValType::I32 {
                            f.instruction(&Instruction::RefI31);
                        }
                    }
                }
                f.instruction(&Instruction::Return);
            }
            MirStmt::Return(None) => {
                f.instruction(&Instruction::Return);
            }
            MirStmt::GcHint { .. } => {}
        }
    }

    pub(super) fn is_builtin_name(&self, name: &str) -> bool {
        matches!(
            name,
            "println"
                | "print"
                | "i32_to_string"
                | "i64_to_string"
                | "f64_to_string"
                | "bool_to_string"
                | "to_string"
                | "concat"
                | "String_from"
                | "len"
                | "push"
                | "get"
                | "get_unchecked"
                | "set"
                | "pop"
                | "join"
                | "panic"
                | "assert"
                | "assert_eq"
                | "Vec_new_i32"
                | "Vec_new_i64"
                | "Vec_new_f64"
                | "Vec_new_String"
                | "sort_i32"
                | "parse_i32"
                | "parse_i64"
                | "parse_f64"
                | "char_to_string"
                | "read_line"
                | "string_len"
                | "char_at"
                | "substring"
                | "string_slice"
                | "contains"
                | "index_of"
                | "to_uppercase"
                | "to_lowercase"
                | "to_upper"
                | "to_lower"
                | "trim"
                | "split"
                | "starts_with"
                | "ends_with"
                | "__intrinsic_replace"
                | "clock_now_ms"
                | "clock_now"
                | "random_i32"
                | "random_f64"
                | "exit"
                | "proc_exit"
                | "args"
                | "HashMap_new_i32_i32"
                | "HashMap_new_i32_String"
                | "HashMap_new_String_i32"
                | "HashMap_new_String_String"
                | "HashMap_i32_i32_new"
                | "HashMap_i32_i32_insert"
                | "HashMap_i32_i32_get"
                | "HashMap_i32_i32_contains_key"
                | "HashMap_i32_i32_len"
                | "insert"
                | "get_or_default"
                | "contains_key"
                | "remove"
                | "keys"
                | "values"
                | "sort_i64"
                | "sort_f64"
                | "map_i32_i32"
                | "map_i32_String"
                | "filter_i32"
                | "filter_String"
                | "filter_i64"
                | "filter_f64"
                | "map_i64_i64"
                | "map_f64_f64"
                | "fold_i64_i64"
                | "String_new"
                | "string_new"
                | "is_empty"
                | "clone"
                | "contains_i32"
                | "contains_String"
                | "reverse_i32"
                | "reverse_String"
                | "remove_i32"
                | "sum_i32"
                | "sum_i64"
                | "sum_f64"
                | "product_i32"
                | "product_i64"
                | "product_f64"
                | "any_i32"
                | "any_String"
                | "find_i32"
                | "find_String"
                | "fold_i32_i32"
                | "fold_f64_f64"
                | "map_String_String"
                | "map_i64_String"
                | "map_f64_String"
                | "eprintln"
                | "sqrt"
                | "abs"
                | "min"
                | "max"
                | "pow"
                | "floor"
                | "ceil"
                | "round"
                | "clamp"
                | "fs_read_file"
                | "fs_write_file"
                | "http_get"
                | "http_request"
                | "memory_copy"
                | "memory_fill"
                | "arg_count"
                | "env_var"
                | "f64_bits_lo"
                | "f64_bits_hi"
                | "eq"
                | "ne"
        ) || (name.starts_with("Vec_new_") && self.custom_vec_types.contains_key(&name[8..]))
    }

    pub(super) fn emit_call_builtin(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
        dest: Option<&Place>,
    ) {
        match canonical {
            "println" => {
                if let Some(arg) = args.first() {
                    self.emit_println(f, arg);
                }
            }
            "print" => {
                if let Some(arg) = args.first() {
                    self.emit_print(f, arg);
                }
            }
            "eprintln" => {
                if let Some(arg) = args.first() {
                    self.emit_eprintln(f, arg);
                }
            }
            "sqrt" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Sqrt);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "floor" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Floor);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "ceil" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Ceil);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "round" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Nearest);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "abs" => {
                if let Some(arg) = args.first() {
                    if self.is_f64_like_operand(arg) {
                        self.emit_operand_coerced(f, arg, false, true);
                        f.instruction(&Instruction::F64Abs);
                    } else {
                        self.emit_operand(f, arg);
                        // i32 abs: (x < 0) ? (0 - x) : x
                        let scratch = self.si(0);
                        f.instruction(&Instruction::LocalSet(scratch));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS);
                        f.instruction(&Instruction::Select);
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "min" => {
                if args.len() >= 2 {
                    let is_f64 =
                        self.is_f64_like_operand(&args[0]) || self.is_f64_like_operand(&args[1]);
                    if is_f64 {
                        self.emit_operand_coerced(f, &args[0], false, true);
                        self.emit_operand_coerced(f, &args[1], false, true);
                        f.instruction(&Instruction::F64Min);
                    } else {
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        // i32 min: select(a, b, a <= b)
                        let s0 = self.si(0);
                        let s1 = self.si(1);
                        f.instruction(&Instruction::LocalSet(s1));
                        f.instruction(&Instruction::LocalSet(s0));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::I32LeS);
                        f.instruction(&Instruction::Select);
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "max" => {
                if args.len() >= 2 {
                    let is_f64 =
                        self.is_f64_like_operand(&args[0]) || self.is_f64_like_operand(&args[1]);
                    if is_f64 {
                        self.emit_operand_coerced(f, &args[0], false, true);
                        self.emit_operand_coerced(f, &args[1], false, true);
                        f.instruction(&Instruction::F64Max);
                    } else {
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        // i32 max: select(a, b, a >= b)
                        let s0 = self.si(0);
                        let s1 = self.si(1);
                        f.instruction(&Instruction::LocalSet(s1));
                        f.instruction(&Instruction::LocalSet(s0));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::I32GeS);
                        f.instruction(&Instruction::Select);
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "pow" => {
                // pow(base, exp) — f64 only. No native Wasm instruction.
                // Emit inline: result = 1.0; loop exp times: result *= base
                if args.len() >= 2 {
                    let s_base = self.si(0);
                    let s_exp = self.si(1);
                    let s_result = self.si(2);
                    // We need the operands evaluated to i32 scratch locals.
                    // base → f64 local, exp → i32 counter
                    self.emit_operand(f, &args[0]);
                    f.instruction(&Instruction::I32TruncF64S);
                    f.instruction(&Instruction::LocalSet(s_base));
                    self.emit_operand(f, &args[1]);
                    if self.is_f64_like_operand(&args[1]) {
                        f.instruction(&Instruction::I32TruncF64S);
                    }
                    f.instruction(&Instruction::LocalSet(s_exp));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::LocalSet(s_result));
                    // loop
                    f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                    f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                    f.instruction(&Instruction::LocalGet(s_exp));
                    f.instruction(&Instruction::I32Eqz);
                    f.instruction(&Instruction::BrIf(1));
                    f.instruction(&Instruction::LocalGet(s_result));
                    f.instruction(&Instruction::LocalGet(s_base));
                    f.instruction(&Instruction::I32Mul);
                    f.instruction(&Instruction::LocalSet(s_result));
                    f.instruction(&Instruction::LocalGet(s_exp));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::I32Sub);
                    f.instruction(&Instruction::LocalSet(s_exp));
                    f.instruction(&Instruction::Br(0));
                    f.instruction(&Instruction::End);
                    f.instruction(&Instruction::End);
                    f.instruction(&Instruction::LocalGet(s_result));
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "clamp" => {
                // clamp(value, lo, hi) — uses min/max pattern
                if args.len() >= 3 {
                    self.emit_operand(f, &args[0]);
                    self.emit_operand(f, &args[1]);
                    if self.is_f64_like_operand(&args[0]) {
                        f.instruction(&Instruction::F64Max); // max(value, lo)
                        self.emit_operand(f, &args[2]);
                        f.instruction(&Instruction::F64Min); // min(result, hi)
                    } else {
                        // i32: max(value, lo) then min(result, hi)
                        let s0 = self.si(0);
                        let s1 = self.si(1);
                        f.instruction(&Instruction::LocalSet(s1)); // lo
                        f.instruction(&Instruction::LocalSet(s0)); // value
                        // max(value, lo)
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::I32GeS);
                        f.instruction(&Instruction::Select);
                        // min(result, hi)
                        f.instruction(&Instruction::LocalSet(s0));
                        self.emit_operand(f, &args[2]);
                        f.instruction(&Instruction::LocalSet(s1));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::I32LeS);
                        f.instruction(&Instruction::Select);
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "assert" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::I32Eqz);
                    f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                    f.instruction(&Instruction::Unreachable);
                    f.instruction(&Instruction::End);
                }
            }
            "assert_eq" => {
                if args.len() >= 2 {
                    let lhs = &args[0];
                    let rhs = &args[1];
                    if self.is_string_like_operand(lhs) || self.is_string_like_operand(rhs) {
                        self.emit_operand(f, lhs);
                        self.emit_operand(f, rhs);
                        self.emit_string_eq_gc(f);
                    } else if self.is_f64_like_operand(lhs) || self.is_f64_like_operand(rhs) {
                        self.emit_operand_coerced(f, lhs, false, true);
                        self.emit_operand_coerced(f, rhs, false, true);
                        f.instruction(&Instruction::F64Eq);
                    } else if self.is_i64_like_operand(lhs) || self.is_i64_like_operand(rhs) {
                        self.emit_operand_coerced(f, lhs, true, false);
                        self.emit_operand_coerced(f, rhs, true, false);
                        f.instruction(&Instruction::I64Eq);
                    } else {
                        self.emit_operand(f, lhs);
                        self.emit_operand(f, rhs);
                        f.instruction(&Instruction::I32Eq);
                    }
                    f.instruction(&Instruction::I32Eqz);
                    f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                    f.instruction(&Instruction::Unreachable);
                    f.instruction(&Instruction::End);
                }
            }
            "i32_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_i32_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "concat" => {
                self.emit_concat(f, args, dest);
            }
            "join" => {
                self.emit_join_gc(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_i32" => {
                self.emit_vec_new_gc(f, self.vec_i32_ty, self.arr_i32_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_i64" => {
                self.emit_vec_new_gc(f, self.vec_i64_ty, self.arr_i64_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_f64" => {
                self.emit_vec_new_gc(f, self.vec_f64_ty, self.arr_f64_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_String" => {
                self.emit_vec_new_gc(f, self.vec_string_ty, self.arr_string_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "push" => {
                self.emit_push(f, args);
            }
            "set" => {
                self.emit_set(f, args);
            }
            "len" => {
                self.emit_len_inline(f, args.first());
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "get" => {
                self.emit_get_inline(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "get_unchecked" => {
                self.emit_get_unchecked_inline(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "pop" => {
                self.emit_pop_inline(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "string_len"
            | "char_at"
            | "substring"
            | "string_slice"
            | "clone"
            | "to_uppercase"
            | "to_lowercase"
            | "to_upper"
            | "to_lower"
            | "trim"
            | "contains"
            | "index_of"
            | "starts_with"
            | "ends_with"
            | "__intrinsic_replace"
            | "split"
            | "eq"
            | "ne" => {
                // Delegate to operand version then store/drop
                self.emit_call_builtin_operand(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "String_from" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "String_new" | "string_new" => {
                // Empty GC string: array.new $string 0
                f.instruction(&Instruction::I32Const(0)); // fill value
                f.instruction(&Instruction::I32Const(0)); // length
                f.instruction(&Instruction::ArrayNew(self.string_ty));
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "is_empty" => {
                // is_empty(s) → array.len(s) == 0
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::ArrayLen);
                    f.instruction(&Instruction::I32Eqz);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "to_string" => {
                if let Some(arg) = args.first() {
                    if self.is_string_like_operand(arg) {
                        self.emit_operand(f, arg);
                    } else if self.is_f64_like_operand(arg) {
                        let converted = Operand::Call("f64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_i64_like_operand(arg) {
                        let converted = Operand::Call("i64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_bool_like_operand(arg) {
                        let converted = Operand::Call("bool_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if matches!(arg, Operand::ConstChar(_))
                        || matches!(arg, Operand::Place(Place::Local(id)) if self.char_locals.contains(&id.0))
                    {
                        let converted = Operand::Call("char_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else {
                        let converted = Operand::Call("i32_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "filter_i64" | "filter_f64" | "filter_i32" | "filter_String" => {
                self.emit_filter_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "map_i64_i64" | "map_f64_f64" | "map_i32_i32" | "map_i32_String" => {
                self.emit_map_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "fold_i64_i64" | "fold_i32_i32" | "fold_f64_f64" => {
                self.emit_fold_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "map_String_String" | "map_i64_String" | "map_f64_String" => {
                self.emit_map_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "contains_i32" | "contains_String" => {
                self.emit_contains_inline(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "reverse_i32" | "reverse_String" => {
                self.emit_reverse_inline(f, canonical, args);
            }
            "remove_i32" => {
                self.emit_remove_inline(f, args);
            }
            "sum_i32" | "sum_i64" | "sum_f64" | "product_i32" | "product_i64" | "product_f64" => {
                self.emit_sum_product_inline(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "any_i32" | "any_String" => {
                self.emit_any_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "find_i32" | "find_String" => {
                self.emit_find_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "fs_read_file" => {
                self.emit_fs_read_file_gc(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "fs_write_file" => {
                self.emit_fs_write_file_gc(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "fs_write_bytes" => {
                self.emit_fs_write_bytes_gc(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "http_get" => {
                self.emit_http_get_builtin(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "http_request" => {
                self.emit_http_request_builtin(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "memory_copy" => {
                // memory.copy: (dst, src, len) → copies len bytes from src to dst
                if args.len() >= 3 {
                    self.emit_operand(f, &args[0]); // dst
                    self.emit_operand(f, &args[1]); // src
                    self.emit_operand(f, &args[2]); // len
                    f.instruction(&Instruction::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                }
            }
            "memory_fill" => {
                // memory.fill: (ptr, val, len) → fills len bytes at ptr with val
                if args.len() >= 3 {
                    self.emit_operand(f, &args[0]); // ptr
                    self.emit_operand(f, &args[1]); // val
                    self.emit_operand(f, &args[2]); // len
                    f.instruction(&Instruction::MemoryFill(0));
                }
            }
            "clock_now" => {
                // clock_time_get(clock_id=0 (realtime), precision=0, result_ptr=SCRATCH)
                f.instruction(&Instruction::I32Const(0)); // clock_id = REALTIME
                f.instruction(&Instruction::I64Const(0)); // precision
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::Call(self.wasi_clock_time_get));
                f.instruction(&Instruction::Drop); // drop errno
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "clock_now_ms" => {
                // clock_time_get returns nanoseconds; divide by 1_000_000 for ms
                f.instruction(&Instruction::I32Const(0)); // clock_id = REALTIME
                f.instruction(&Instruction::I64Const(0)); // precision
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::Call(self.wasi_clock_time_get));
                f.instruction(&Instruction::Drop); // drop errno
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I64Const(1_000_000));
                f.instruction(&Instruction::I64DivU);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "random_i32" => {
                // random_get(buf_ptr=SCRATCH, buf_len=4)
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::Call(self.wasi_random_get));
                f.instruction(&Instruction::Drop); // drop errno
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "random_f64" => {
                // random_get(buf_ptr=SCRATCH, buf_len=8), then convert u64 → f64 in [0,1)
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Const(8));
                f.instruction(&Instruction::Call(self.wasi_random_get));
                f.instruction(&Instruction::Drop); // drop errno
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                // Mask to 53 bits, convert to f64, divide by 2^53 → [0, 1)
                f.instruction(&Instruction::I64Const((1i64 << 53) - 1));
                f.instruction(&Instruction::I64And);
                f.instruction(&Instruction::F64ConvertI64U);
                f.instruction(&Instruction::F64Const((1u64 << 53) as f64));
                f.instruction(&Instruction::F64Div);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "exit" | "proc_exit" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                f.instruction(&Instruction::Call(self.wasi_proc_exit));
            }
            "args" => {
                self.emit_args_builtin(f);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "arg_count" => {
                self.emit_arg_count_builtin(f);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "arg_at" => {
                let idx_op = args.first().cloned().unwrap_or(Operand::ConstI32(0));
                self.emit_arg_at_builtin(f, &idx_op);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "f64_bits_lo" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::I64ReinterpretF64);
                    f.instruction(&Instruction::I32WrapI64);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "f64_bits_hi" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::I64ReinterpretF64);
                    f.instruction(&Instruction::I64Const(32));
                    f.instruction(&Instruction::I64ShrU);
                    f.instruction(&Instruction::I32WrapI64);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "env_var" => {
                let name_op = args
                    .first()
                    .cloned()
                    .unwrap_or(Operand::ConstString(String::new()));
                self.emit_env_var_builtin(f, &name_op);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            _ if canonical.starts_with("Vec_new_") => {
                let sname = &canonical[8..];
                if let Some(&(arr_ty, vec_ty)) = self.custom_vec_types.get(sname) {
                    self.emit_vec_new_gc(f, vec_ty, arr_ty);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "HashMap_i32_i32_new" => {
                // Create keys and values arrays (initial capacity 16), then struct
                let hm_ty = self.hashmap_i32_i32_ty;
                let arr_ty = self.arr_i32_ty;
                f.instruction(&Instruction::I32Const(0)); // fill value
                f.instruction(&Instruction::I32Const(16)); // initial capacity
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(16));
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0)); // count = 0
                f.instruction(&Instruction::StructNew(hm_ty));
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "HashMap_i32_i32_insert" => {
                self.emit_hashmap_i32_i32_insert(f, args);
            }
            "HashMap_i32_i32_get" => {
                self.emit_hashmap_i32_i32_get(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "HashMap_i32_i32_len" => {
                // struct.get $hashmap_i32_i32 2 (count field)
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: self.hashmap_i32_i32_ty,
                        field_index: 2,
                    });
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "HashMap_i32_i32_contains_key" => {
                self.emit_hashmap_i32_i32_contains_key(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            _ => {
                for arg in args {
                    self.emit_operand(f, arg);
                }
                if let Some(&fn_idx) = self.fn_map.get(canonical) {
                    f.instruction(&Instruction::Call(fn_idx));
                    let returns_value = self
                        .fn_ret_types
                        .get(canonical)
                        .is_some_and(|ty| !matches!(ty, Type::Unit | Type::Never));
                    if let Some(Place::Local(id)) = dest {
                        if returns_value {
                            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                        }
                    } else if returns_value {
                        f.instruction(&Instruction::Drop);
                    }
                } else if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                }
            }
        }
    }

    /// Handle builtin calls as operands (result stays on the stack).
    pub(super) fn emit_call_builtin_operand(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        match canonical {
            "i32_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_i32_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "bool_to_string" => {
                // GC-native: if arg then "true" else "false" as GC arrays
                if let Some(arg) = args.first() {
                    let str_ref = ref_nullable(self.string_ty);
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(str_ref)));
                    let seg = self.alloc_string_data(b"true");
                    let abs_seg = self.data_segs.len() as u32 + seg;
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(4));
                    f.instruction(&Instruction::ArrayNewData {
                        array_type_index: self.string_ty,
                        array_data_index: abs_seg,
                    });
                    f.instruction(&Instruction::Else);
                    let seg = self.alloc_string_data(b"false");
                    let abs_seg = self.data_segs.len() as u32 + seg;
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(5));
                    f.instruction(&Instruction::ArrayNewData {
                        array_type_index: self.string_ty,
                        array_data_index: abs_seg,
                    });
                    f.instruction(&Instruction::End);
                }
            }
            "char_to_string" => {
                // GC-native: create 1-byte GC string from char value
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg); // char value (i32 = byte)
                    f.instruction(&Instruction::I32Const(1)); // length = 1
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "i64_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_i64_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "f64_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_f64_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "String_from" | "String_new" | "string_new" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                } else {
                    // String::new() → empty GC string
                    f.instruction(&Instruction::I32Const(0)); // fill
                    f.instruction(&Instruction::I32Const(0)); // length
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "to_string" => {
                if let Some(arg) = args.first() {
                    if self.is_string_like_operand(arg) {
                        self.emit_operand(f, arg);
                    } else if self.is_f64_like_operand(arg) {
                        let converted = Operand::Call("f64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_i64_like_operand(arg) {
                        let converted = Operand::Call("i64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_bool_like_operand(arg) {
                        let converted = Operand::Call("bool_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if matches!(arg, Operand::ConstChar(_))
                        || matches!(arg, Operand::Place(Place::Local(id)) if self.char_locals.contains(&id.0))
                    {
                        let converted = Operand::Call("char_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else {
                        let converted = Operand::Call("i32_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    }
                }
            }
            "concat" => {
                self.emit_concat_gc(f, args);
            }
            "join" => {
                self.emit_join_gc(f, args);
            }
            "Vec_new_i32" => {
                self.emit_vec_new_gc(f, self.vec_i32_ty, self.arr_i32_ty);
            }
            "Vec_new_i64" => {
                self.emit_vec_new_gc(f, self.vec_i64_ty, self.arr_i64_ty);
            }
            "Vec_new_f64" => {
                self.emit_vec_new_gc(f, self.vec_f64_ty, self.arr_f64_ty);
            }
            "Vec_new_String" => {
                self.emit_vec_new_gc(f, self.vec_string_ty, self.arr_string_ty);
            }
            "len" => {
                self.emit_len_inline(f, args.first());
            }
            "get" => {
                self.emit_get_inline(f, args);
            }
            "get_unchecked" => {
                self.emit_get_unchecked_inline(f, args);
            }
            "pop" => {
                self.emit_pop_inline(f, args);
            }
            "string_len" => {
                // GC-native: array.len on GC string ref
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::ArrayLen);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "char_at" => {
                // GC-native: array.get_u $string ref idx
                if args.len() >= 2 {
                    self.emit_operand(f, &args[0]); // string ref
                    self.emit_operand(f, &args[1]); // index
                    f.instruction(&Instruction::ArrayGetU(self.string_ty));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "substring" | "string_slice" => {
                // GC-native: new array + array.copy slice
                if args.len() >= 3 {
                    self.emit_substring_gc(f, &args[0], &args[1], &args[2]);
                } else {
                    f.instruction(&Instruction::I32Const(0)); // fill
                    f.instruction(&Instruction::I32Const(0)); // len
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "clone" => {
                // GC-native: copy entire string to new GC array
                if let Some(arg) = args.first() {
                    self.emit_string_clone_gc(f, arg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "to_uppercase" | "to_lowercase" | "to_upper" | "to_lower" => {
                if let Some(arg) = args.first() {
                    self.emit_case_transform_gc(
                        f,
                        arg,
                        canonical == "to_uppercase" || canonical == "to_upper",
                    );
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "trim" => {
                if let Some(arg) = args.first() {
                    self.emit_trim_gc(f, arg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "contains" => {
                if args.len() >= 2 {
                    self.emit_contains_gc(f, &args[0], &args[1]);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "index_of" => {
                if args.len() >= 2 {
                    self.emit_index_of_gc(f, &args[0], &args[1]);
                } else {
                    f.instruction(&Instruction::I32Const(-1i32));
                }
            }
            "starts_with" => {
                if args.len() >= 2 {
                    self.emit_starts_with_gc(f, &args[0], &args[1]);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "ends_with" => {
                if args.len() >= 2 {
                    self.emit_ends_with_gc(f, &args[0], &args[1]);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "__intrinsic_replace" => {
                // Stub: return clone of input
                if let Some(arg) = args.first() {
                    self.emit_string_clone_gc(f, arg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "split" => {
                if args.len() >= 2 {
                    self.emit_split_gc(f, args);
                } else {
                    // Stub: return empty Vec<String>
                    self.emit_vec_new_gc(f, self.vec_string_ty, self.arr_string_ty);
                }
            }
            "filter_i64" | "filter_f64" | "filter_i32" | "filter_String" => {
                self.emit_filter_hof_gc(f, canonical, args);
            }
            "map_i64_i64" | "map_f64_f64" | "map_i32_i32" | "map_i32_String" => {
                self.emit_map_hof_gc(f, canonical, args);
            }
            "fold_i64_i64" => {
                self.emit_fold_hof_gc(f, canonical, args);
            }
            "contains_i32" | "contains_String" => {
                self.emit_contains_inline(f, canonical, args);
            }
            "reverse_i32" | "reverse_String" => {
                self.emit_reverse_inline(f, canonical, args);
                // reverse is void; push dummy 0
                f.instruction(&Instruction::I32Const(0));
            }
            "remove_i32" => {
                self.emit_remove_inline(f, args);
                f.instruction(&Instruction::I32Const(0));
            }
            "sum_i32" | "sum_i64" | "sum_f64" | "product_i32" | "product_i64" | "product_f64" => {
                self.emit_sum_product_inline(f, canonical, args);
            }
            "fold_i32_i32" | "fold_f64_f64" => {
                self.emit_fold_hof_gc(f, canonical, args);
            }
            "map_String_String" | "map_i64_String" | "map_f64_String" => {
                self.emit_map_hof_gc(f, canonical, args);
            }
            "any_i32" | "any_String" => {
                self.emit_any_hof_gc(f, canonical, args);
            }
            "find_i32" | "find_String" => {
                self.emit_find_hof_gc(f, canonical, args);
            }
            "is_empty" => {
                // is_empty(s) → array.len(s) == 0
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::ArrayLen);
                    f.instruction(&Instruction::I32Eqz);
                } else {
                    f.instruction(&Instruction::I32Const(1));
                }
            }
            "eq" => {
                // eq(a, b) → type-dispatched equality
                if args.len() >= 2 {
                    let lhs = &args[0];
                    let rhs = &args[1];
                    if self.is_string_like_operand(lhs) || self.is_string_like_operand(rhs) {
                        self.emit_operand(f, lhs);
                        self.emit_operand(f, rhs);
                        self.emit_string_eq_gc(f);
                    } else if self.is_f64_like_operand(lhs) || self.is_f64_like_operand(rhs) {
                        self.emit_operand_coerced(f, lhs, false, true);
                        self.emit_operand_coerced(f, rhs, false, true);
                        f.instruction(&Instruction::F64Eq);
                    } else if self.is_i64_like_operand(lhs) || self.is_i64_like_operand(rhs) {
                        self.emit_operand_coerced(f, lhs, true, false);
                        self.emit_operand_coerced(f, rhs, true, false);
                        f.instruction(&Instruction::I64Eq);
                    } else {
                        self.emit_operand(f, lhs);
                        self.emit_operand(f, rhs);
                        f.instruction(&Instruction::I32Eq);
                    }
                } else {
                    f.instruction(&Instruction::I32Const(1)); // zero-arg eq = true
                }
            }
            "ne" => {
                // ne(a, b) → !eq(a, b)
                if args.len() >= 2 {
                    let lhs = &args[0];
                    let rhs = &args[1];
                    if self.is_string_like_operand(lhs) || self.is_string_like_operand(rhs) {
                        self.emit_operand(f, lhs);
                        self.emit_operand(f, rhs);
                        self.emit_string_eq_gc(f);
                        f.instruction(&Instruction::I32Eqz);
                    } else if self.is_f64_like_operand(lhs) || self.is_f64_like_operand(rhs) {
                        self.emit_operand_coerced(f, lhs, false, true);
                        self.emit_operand_coerced(f, rhs, false, true);
                        f.instruction(&Instruction::F64Ne);
                    } else if self.is_i64_like_operand(lhs) || self.is_i64_like_operand(rhs) {
                        self.emit_operand_coerced(f, lhs, true, false);
                        self.emit_operand_coerced(f, rhs, true, false);
                        f.instruction(&Instruction::I64Ne);
                    } else {
                        self.emit_operand(f, lhs);
                        self.emit_operand(f, rhs);
                        f.instruction(&Instruction::I32Ne);
                    }
                } else {
                    f.instruction(&Instruction::I32Const(0)); // zero-arg ne = false
                }
            }
            _ if canonical.starts_with("Vec_new_") => {
                let sname = &canonical[8..];
                if let Some(&(arr_ty, vec_ty)) = self.custom_vec_types.get(sname) {
                    self.emit_vec_new_gc(f, vec_ty, arr_ty);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "parse_i32" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_parse_i32 {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "parse_i64" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_parse_i64 {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "parse_f64" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_parse_f64 {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "fs_read_file" => {
                self.emit_fs_read_file_gc(f, args);
            }
            "fs_write_file" => {
                self.emit_fs_write_file_gc(f, args);
            }
            "fs_write_bytes" => {
                self.emit_fs_write_bytes_gc(f, args);
            }
            "http_get" => {
                self.emit_http_get_builtin(f, args);
            }
            "http_request" => {
                self.emit_http_request_builtin(f, args);
            }
            "memory_copy" => {
                // memory.copy returns Unit; emit instruction, push dummy i32 for stack
                if args.len() >= 3 {
                    self.emit_operand(f, &args[0]);
                    self.emit_operand(f, &args[1]);
                    self.emit_operand(f, &args[2]);
                    f.instruction(&Instruction::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                }
                f.instruction(&Instruction::I32Const(0));
            }
            "memory_fill" => {
                // memory.fill returns Unit; emit instruction, push dummy i32 for stack
                if args.len() >= 3 {
                    self.emit_operand(f, &args[0]);
                    self.emit_operand(f, &args[1]);
                    self.emit_operand(f, &args[2]);
                    f.instruction(&Instruction::MemoryFill(0));
                }
                f.instruction(&Instruction::I32Const(0));
            }
            "HashMap_i32_i32_new" => {
                let hm_ty = self.hashmap_i32_i32_ty;
                let arr_ty = self.arr_i32_ty;
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(16));
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(16));
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::StructNew(hm_ty));
            }
            "HashMap_i32_i32_get" => {
                self.emit_hashmap_i32_i32_get(f, args);
            }
            "HashMap_i32_i32_len" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: self.hashmap_i32_i32_ty,
                        field_index: 2,
                    });
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "HashMap_i32_i32_contains_key" => {
                self.emit_hashmap_i32_i32_contains_key(f, args);
            }
            "clock_now" => {
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I64Const(0));
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::Call(self.wasi_clock_time_get));
                f.instruction(&Instruction::Drop);
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
            }
            "clock_now_ms" => {
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I64Const(0));
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::Call(self.wasi_clock_time_get));
                f.instruction(&Instruction::Drop);
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I64Const(1_000_000));
                f.instruction(&Instruction::I64DivU);
            }
            "random_i32" => {
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::Call(self.wasi_random_get));
                f.instruction(&Instruction::Drop);
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
            }
            "random_f64" => {
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Const(8));
                f.instruction(&Instruction::Call(self.wasi_random_get));
                f.instruction(&Instruction::Drop);
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I64Const((1i64 << 53) - 1));
                f.instruction(&Instruction::I64And);
                f.instruction(&Instruction::F64ConvertI64U);
                f.instruction(&Instruction::F64Const((1u64 << 53) as f64));
                f.instruction(&Instruction::F64Div);
            }
            "arg_count" => {
                self.emit_arg_count_builtin(f);
            }
            "arg_at" => {
                let idx_op = args.first().cloned().unwrap_or(Operand::ConstI32(0));
                self.emit_arg_at_builtin(f, &idx_op);
            }
            "args" => {
                self.emit_args_builtin(f);
            }
            "env_var" => {
                let name_op = args
                    .first()
                    .cloned()
                    .unwrap_or(Operand::ConstString(String::new()));
                self.emit_env_var_builtin(f, &name_op);
            }
            "sqrt" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Sqrt);
                }
            }
            "f64_bits_lo" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::I64ReinterpretF64);
                    f.instruction(&Instruction::I32WrapI64);
                }
            }
            "f64_bits_hi" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::I64ReinterpretF64);
                    f.instruction(&Instruction::I64Const(32));
                    f.instruction(&Instruction::I64ShrU);
                    f.instruction(&Instruction::I32WrapI64);
                }
            }
            "floor" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Floor);
                }
            }
            "ceil" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Ceil);
                }
            }
            "round" => {
                if let Some(arg) = args.first() {
                    self.emit_operand_coerced(f, arg, false, true);
                    f.instruction(&Instruction::F64Nearest);
                }
            }
            "abs" => {
                if let Some(arg) = args.first() {
                    if self.is_f64_like_operand(arg) {
                        self.emit_operand_coerced(f, arg, false, true);
                        f.instruction(&Instruction::F64Abs);
                    } else {
                        self.emit_operand(f, arg);
                        let scratch = self.si(0);
                        f.instruction(&Instruction::LocalSet(scratch));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS);
                        f.instruction(&Instruction::Select);
                    }
                }
            }
            "min" => {
                if args.len() >= 2 {
                    let is_f64 =
                        self.is_f64_like_operand(&args[0]) || self.is_f64_like_operand(&args[1]);
                    if is_f64 {
                        self.emit_operand_coerced(f, &args[0], false, true);
                        self.emit_operand_coerced(f, &args[1], false, true);
                        f.instruction(&Instruction::F64Min);
                    } else {
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        let s0 = self.si(0);
                        let s1 = self.si(1);
                        f.instruction(&Instruction::LocalSet(s1));
                        f.instruction(&Instruction::LocalSet(s0));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::I32LeS);
                        f.instruction(&Instruction::Select);
                    }
                }
            }
            "max" => {
                if args.len() >= 2 {
                    let is_f64 =
                        self.is_f64_like_operand(&args[0]) || self.is_f64_like_operand(&args[1]);
                    if is_f64 {
                        self.emit_operand_coerced(f, &args[0], false, true);
                        self.emit_operand_coerced(f, &args[1], false, true);
                        f.instruction(&Instruction::F64Max);
                    } else {
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        let s0 = self.si(0);
                        let s1 = self.si(1);
                        f.instruction(&Instruction::LocalSet(s1));
                        f.instruction(&Instruction::LocalSet(s0));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::LocalGet(s0));
                        f.instruction(&Instruction::LocalGet(s1));
                        f.instruction(&Instruction::I32GeS);
                        f.instruction(&Instruction::Select);
                    }
                }
            }
            _ => {
                // Unimplemented builtin as operand — push null ref for string types
                // or zero for scalars
                f.instruction(&Instruction::I32Const(0));
            }
        }
    }

    /// Emit the ? operator: evaluate expr (Result enum), extract Ok payload or early-return Err
    pub(super) fn emit_try_expr(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        expr: &Operand,
        from_fn: Option<&String>,
    ) {
        // Determine which Result enum type the expr produces
        let result_type = self.infer_operand_type(expr);

        // Find the matching enum and its variants
        let (enum_name, ok_ty, err_ty) = if let ValType::Ref(rt) = result_type {
            match rt.heap_type {
                HeapType::Concrete(idx) => {
                    // Find which enum has this base type
                    let mut found = None;
                    for (name, &base) in &self.enum_base_types {
                        if base == idx {
                            found = Some(name.clone());
                            break;
                        }
                    }
                    if let Some(ename) = found {
                        let variants = self.enum_variant_types.get(&ename).cloned();
                        if let Some(v) = variants {
                            let ok_idx = v.get("Ok").copied();
                            let err_idx = v.get("Err").copied();
                            (Some(ename), ok_idx, err_idx)
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    }
                }
                _ => (None, None, None),
            }
        } else {
            (None, None, None)
        };

        if let (Some(ename), Some(ok_variant), Some(err_variant)) = (enum_name, ok_ty, err_ty) {
            let anyref_scratch = self.si(10);
            let outer_result_enum = self
                .current_result_enum_name()
                .unwrap_or_else(|| ename.clone());
            let outer_err_variant = self
                .enum_variant_types
                .get(outer_result_enum.as_str())
                .and_then(|variants| variants.get("Err"))
                .copied();

            // Emit inner expression → ref $Result on stack
            self.emit_operand(f, expr);

            // Save to anyref scratch
            f.instruction(&Instruction::LocalSet(anyref_scratch));

            // Check if Ok variant using ref.test
            f.instruction(&Instruction::LocalGet(anyref_scratch));
            f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(ok_variant)));

            // Determine Ok payload type from enum_defs
            let ok_payload_ty = self.get_ok_payload_type(&ename);

            f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                ok_payload_ty,
            )));
            {
                // Ok path: cast and extract payload
                f.instruction(&Instruction::LocalGet(anyref_scratch));
                f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(ok_variant)));
                f.instruction(&Instruction::StructGet {
                    struct_type_index: ok_variant,
                    field_index: 0,
                });
            }
            f.instruction(&Instruction::Else);
            {
                if let Some(outer_err_variant) = outer_err_variant {
                    f.instruction(&Instruction::LocalGet(anyref_scratch));
                    f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
                        err_variant,
                    )));
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: err_variant,
                        field_index: 0,
                    });
                    if outer_result_enum != "Result"
                        && let Some(from_fn_name) = from_fn
                        && let Some(&from_idx) = self.fn_map.get(from_fn_name.as_str())
                    {
                        f.instruction(&Instruction::Call(from_idx));
                    }
                    f.instruction(&Instruction::StructNew(outer_err_variant));
                    f.instruction(&Instruction::Return);
                } else {
                    f.instruction(&Instruction::LocalGet(anyref_scratch));
                    f.instruction(&Instruction::Return);
                }
            }
            f.instruction(&Instruction::End);
        } else {
            // Fallback: just emit inner expression
            self.emit_operand(f, expr);
        }
    }

    /// Get the Wasm ValType of an Ok variant's payload from enum_defs
    pub(super) fn get_ok_payload_type(&self, enum_name: &str) -> ValType {
        if let Some(variants) = self.enum_defs.get(enum_name) {
            for (vname, fields) in variants {
                if vname == "Ok"
                    && let Some(field_type) = fields.first()
                {
                    return self.type_name_to_val(field_type);
                }
            }
        }
        ValType::I32 // default
    }

    pub(super) fn emit_println(&mut self, f: &mut PeepholeWriter<'_>, arg: &Operand) {
        self.emit_operand(f, arg);
        if self.is_string_like_operand(arg) {
            if let Some(idx) = self.helper_print_str_ln {
                f.instruction(&Instruction::Call(idx));
            }
        } else if self.is_bool_like_operand(arg) {
            if let Some(idx) = self.helper_print_bool_ln {
                f.instruction(&Instruction::Call(idx));
            }
        } else if let Some(idx) = self.helper_print_i32_ln {
            f.instruction(&Instruction::Call(idx));
        }
    }

    pub(super) fn emit_print(&mut self, f: &mut PeepholeWriter<'_>, arg: &Operand) {
        self.emit_operand(f, arg);
        if self.is_string_like_operand(arg) {
            if let Some(idx) = self.helper_print_str {
                f.instruction(&Instruction::Call(idx));
            }
        } else if self.is_bool_like_operand(arg) {
            if let Some(idx) = self.helper_print_bool {
                f.instruction(&Instruction::Call(idx));
            }
        } else if let Some(idx) = self.helper_print_i32 {
            f.instruction(&Instruction::Call(idx));
        }
    }

    pub(super) fn emit_eprintln(&mut self, f: &mut PeepholeWriter<'_>, arg: &Operand) {
        self.emit_operand(f, arg);
        // eprintln always uses the string version — convert if needed
        if !self.is_string_like_operand(arg)
            && let Some(to_str) = self.helper_i32_to_str
        {
            f.instruction(&Instruction::Call(to_str));
        }
        if let Some(idx) = self.helper_eprint_str_ln {
            f.instruction(&Instruction::Call(idx));
        }
    }

    pub(super) fn is_string_like_operand(&self, operand: &Operand) -> bool {
        let vt = self.infer_operand_type(operand);
        matches!(vt, ValType::Ref(rt) if rt.heap_type == HeapType::Concrete(self.string_ty))
    }

    pub(super) fn is_bool_like_operand(&self, operand: &Operand) -> bool {
        match operand {
            Operand::ConstBool(_) => true,
            Operand::Place(Place::Local(id)) => self.bool_locals.contains(&id.0),
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                matches!(
                    canonical,
                    "eq" | "starts_with" | "ends_with" | "contains" | "assert" | "assert_eq"
                ) || self
                    .fn_ret_types
                    .get(name)
                    .or_else(|| self.fn_ret_types.get(canonical))
                    == Some(&Type::Bool)
            }
            Operand::BinOp(op, _, _) => matches!(
                op,
                BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::And
                    | BinOp::Or
            ),
            Operand::UnaryOp(op, _) => matches!(op, UnaryOp::Not),
            _ => false,
        }
    }

    pub(super) fn is_f64_like_operand(&self, operand: &Operand) -> bool {
        match operand {
            Operand::ConstF64(_) | Operand::ConstF32(_) => true,
            Operand::Place(Place::Local(id)) => self.f64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_f64_like_operand(l) || self.is_f64_like_operand(r),
            Operand::UnaryOp(_, inner) => self.is_f64_like_operand(inner),
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                matches!(canonical, "sqrt" | "random_f64")
                    || self
                        .fn_ret_types
                        .get(name)
                        .or_else(|| self.fn_ret_types.get(canonical))
                        == Some(&Type::F64)
            }
            _ => false,
        }
    }

    pub(super) fn is_i64_like_operand(&self, operand: &Operand) -> bool {
        match operand {
            Operand::ConstI64(_) | Operand::ConstU64(_) => true,
            Operand::Place(Place::Local(id)) => self.i64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_i64_like_operand(l) || self.is_i64_like_operand(r),
            Operand::UnaryOp(_, inner) => self.is_i64_like_operand(inner),
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                matches!(canonical, "clock_now" | "clock_now_ms")
                    || self
                        .fn_ret_types
                        .get(name)
                        .or_else(|| self.fn_ret_types.get(canonical))
                        == Some(&Type::I64)
            }
            _ => false,
        }
    }

    /// Determine Vec element size from the vec operand (checks f64_vec_locals/i64_vec_locals).
    pub(super) fn vec_elem_size(&self, vec_operand: &Operand) -> i32 {
        match vec_operand {
            Operand::Place(Place::Local(id)) => {
                if self.f64_vec_locals.contains(&id.0) || self.i64_vec_locals.contains(&id.0) {
                    8
                } else {
                    4
                }
            }
            _ => 4,
        }
    }

    /// Check if a Vec operand holds f64 elements.
    pub(super) fn is_f64_vec_operand(&self, operand: &Operand) -> bool {
        matches!(operand, Operand::Place(Place::Local(id)) if self.f64_vec_locals.contains(&id.0))
    }

    /// Check if a Vec operand holds i64 elements.
    pub(super) fn is_i64_vec_operand(&self, operand: &Operand) -> bool {
        matches!(operand, Operand::Place(Place::Local(id)) if self.i64_vec_locals.contains(&id.0))
    }

    /// Check if a Vec operand holds string elements.
    pub(super) fn is_string_vec_operand(&self, operand: &Operand) -> bool {
        let vt = self.infer_operand_type(operand);
        matches!(vt, ValType::Ref(rt) if rt.heap_type == HeapType::Concrete(self.vec_string_ty))
    }

    /// Infer the GC vec struct type index for a vec operand.
    pub(super) fn infer_vec_type_idx(&self, operand: &Operand) -> u32 {
        if self.is_f64_vec_operand(operand) {
            self.vec_f64_ty
        } else if self.is_i64_vec_operand(operand) {
            self.vec_i64_ty
        } else if self.is_string_vec_operand(operand) {
            self.vec_string_ty
        } else if let Some(sname) = self.get_struct_vec_name(operand) {
            self.custom_vec_types
                .get(&sname)
                .map(|&(_, v)| v)
                .unwrap_or(self.vec_i32_ty)
        } else {
            self.vec_i32_ty
        }
    }

    /// Infer the GC array type index for a vec operand's backing array.
    pub(super) fn infer_arr_type_idx(&self, operand: &Operand) -> u32 {
        if self.is_f64_vec_operand(operand) {
            self.arr_f64_ty
        } else if self.is_i64_vec_operand(operand) {
            self.arr_i64_ty
        } else if self.is_string_vec_operand(operand) {
            self.arr_string_ty
        } else if let Some(sname) = self.get_struct_vec_name(operand) {
            self.custom_vec_types
                .get(&sname)
                .map(|&(a, _)| a)
                .unwrap_or(self.arr_i32_ty)
        } else {
            self.arr_i32_ty
        }
    }

    /// If operand is a local that holds a Vec<Struct>, return the struct name.
    pub(super) fn get_struct_vec_name(&self, operand: &Operand) -> Option<String> {
        match operand {
            Operand::Place(Place::Local(id)) => self.struct_vec_locals.get(&id.0).cloned(),
            _ => None,
        }
    }

    pub(super) fn emit_concat(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        _args: &[Operand],
        dest: Option<&Place>,
    ) {
        self.emit_concat_gc(f, _args);
        if let Some(Place::Local(id)) = dest {
            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
        } else {
            f.instruction(&Instruction::Drop);
        }
    }

    // ── GC-native string operations ─────────────────────────────────

    /// scratch local indices relative to scratch_base
    pub(super) fn si(&self, offset: u32) -> u32 {
        self.scratch_base + offset
    }

    /// Infer the enum name from an operand (for EnumTag dispatch).
    pub(super) fn infer_enum_name(&self, operand: &Operand) -> String {
        match operand {
            Operand::Place(Place::Local(id)) => self
                .local_enum
                .get(&id.0)
                .cloned()
                .or_else(|| self.enum_name_from_val_type(&self.infer_operand_type(operand)))
                .unwrap_or_default(),
            _ => self
                .enum_name_from_val_type(&self.infer_operand_type(operand))
                .unwrap_or_default(),
        }
    }

    pub(super) fn enum_name_from_val_type(&self, val_type: &ValType) -> Option<String> {
        match val_type {
            ValType::Ref(rt) => match rt.heap_type {
                HeapType::Concrete(idx) => self
                    .enum_base_types
                    .iter()
                    .find_map(|(name, &base_idx)| (base_idx == idx).then(|| name.clone())),
                _ => None,
            },
            _ => None,
        }
    }

    /// Infer the Wasm ValType for an if-expression result from its branches.
    pub(super) fn infer_if_result_type(
        &self,
        then_result: Option<&Operand>,
        else_result: Option<&Operand>,
    ) -> ValType {
        let mut types = vec![];
        for result in [then_result, else_result] {
            match result {
                Some(Operand::Unit) | None => continue,
                Some(op) => types.push(self.infer_operand_type(op)),
            }
        }
        let anyref_vt = ValType::Ref(WasmRefType {
            nullable: true,
            heap_type: HeapType::Abstract {
                shared: false,
                ty: wasm_encoder::AbstractHeapType::Any,
            },
        });
        // If any branch produces anyref, the result type must be anyref
        // (value types in the other branch will be boxed via ref.i31)
        if types.contains(&anyref_vt) {
            return anyref_vt;
        }
        // If both branches produce enum refs and one is non-null, prefer nullable
        types.first().cloned().unwrap_or(ValType::I32)
    }

    /// Check whether an operand produces a value on the Wasm stack.
    pub(super) fn operand_produces_value(&self, op: &Operand) -> bool {
        match op {
            Operand::Unit => false,
            Operand::Call(name, _) => self
                .fn_ret_types
                .get(name.as_str())
                .is_some_and(|ty| !matches!(ty, Type::Unit | Type::Never)),
            _ => true,
        }
    }

    /// Infer the Wasm ValType that an operand produces.
    pub(super) fn infer_operand_type(&self, op: &Operand) -> ValType {
        match op {
            Operand::ConstI32(_)
            | Operand::ConstBool(_)
            | Operand::ConstChar(_)
            | Operand::ConstU8(_)
            | Operand::ConstU16(_)
            | Operand::ConstU32(_)
            | Operand::ConstI8(_)
            | Operand::ConstI16(_) => ValType::I32,
            Operand::ConstI64(_) | Operand::ConstU64(_) => ValType::I64,
            Operand::ConstF64(_) => ValType::F64,
            Operand::ConstF32(_) => ValType::F32,
            Operand::ConstString(_) => ref_nullable(self.string_ty),
            Operand::Place(Place::Local(id)) => {
                if self.string_locals.contains(&id.0) {
                    return ref_nullable(self.string_ty);
                }
                if self.f64_locals.contains(&id.0) {
                    return ValType::F64;
                }
                if self.i64_locals.contains(&id.0) {
                    return ValType::I64;
                }
                if self.any_locals.contains(&id.0) {
                    return ValType::Ref(WasmRefType {
                        nullable: true,
                        heap_type: HeapType::ANY,
                    });
                }
                if let Some(sname) = self.local_struct.get(&id.0)
                    && let Some(&ty_idx) = self.struct_gc_types.get(sname)
                {
                    return ref_nullable(ty_idx);
                }
                if let Some(ename) = self.local_enum.get(&id.0)
                    && let Some(&base_idx) = self.enum_base_types.get(ename)
                {
                    return ref_nullable(base_idx);
                }
                // Vec locals
                if self.i32_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_i32_ty);
                }
                if self.i64_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_i64_ty);
                }
                if self.f64_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_f64_ty);
                }
                if self.string_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_string_ty);
                }
                if let Some(sname) = self.struct_vec_locals.get(&id.0)
                    && let Some(&(_, vec_ty)) = self.custom_vec_types.get(sname)
                {
                    return ref_nullable(vec_ty);
                }
                ValType::I32
            }
            Operand::StructInit { name, .. } => {
                if let Some(&ty_idx) = self.struct_gc_types.get(name) {
                    ref_nullable(ty_idx)
                } else {
                    ValType::I32
                }
            }
            Operand::EnumInit {
                enum_name, variant, ..
            } => {
                let effective_enum_name = if matches!(variant.as_str(), "Ok" | "Err") {
                    self.current_result_enum_name()
                        .unwrap_or_else(|| enum_name.clone())
                } else {
                    enum_name.clone()
                };
                if let Some(&base_idx) = self.enum_base_types.get(effective_enum_name.as_str()) {
                    ref_nullable(base_idx)
                } else {
                    ValType::I32
                }
            }
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic(name);
                // Handle Vec_new_* builtins directly
                match canonical {
                    "Vec_new_i32" => return ref_nullable(self.vec_i32_ty),
                    "Vec_new_i64" => return ref_nullable(self.vec_i64_ty),
                    "Vec_new_f64" => return ref_nullable(self.vec_f64_ty),
                    "Vec_new_String" => return ref_nullable(self.vec_string_ty),
                    // get_unchecked/get on typed vec → infer element type
                    "get_unchecked" | "get" if !args.is_empty() => {
                        if self.is_string_vec_operand(&args[0]) {
                            return ref_nullable(self.string_ty);
                        } else if self.is_f64_vec_operand(&args[0]) {
                            return ValType::F64;
                        } else if self.is_i64_vec_operand(&args[0]) {
                            return ValType::I64;
                        } else if let Some(sname) = self.get_struct_vec_name(&args[0])
                            && let Some(&ty_idx) = self.struct_gc_types.get(&sname)
                        {
                            return ref_nullable(ty_idx);
                        }
                        // For get(), it returns Result — fall through to fn_ret_type_names
                        if canonical == "get_unchecked" {
                            return ValType::I32;
                        }
                    }
                    "concat"
                    | "clone"
                    | "to_uppercase"
                    | "to_lowercase"
                    | "to_upper"
                    | "to_lower"
                    | "trim"
                    | "__intrinsic_replace"
                    | "substring"
                    | "string_slice"
                    | "String_from"
                    | "String_new"
                    | "string_new"
                    | "char_to_string"
                    | "i32_to_string"
                    | "i64_to_string"
                    | "f64_to_string"
                    | "bool_to_string"
                    | "to_string" => {
                        return ref_nullable(self.string_ty);
                    }
                    "contains_i32" | "contains_String" | "len" | "string_len" | "char_at"
                    | "sum_i32" | "product_i32" => {
                        return ValType::I32;
                    }
                    "sum_i64" | "product_i64" => return ValType::I64,
                    "sum_f64" | "product_f64" => return ValType::F64,
                    "parse_i32" => {
                        if let Some(&base_idx) = self.enum_base_types.get("Result") {
                            return ref_nullable(base_idx);
                        }
                    }
                    "parse_i64" => {
                        if let Some(&base_idx) = self.enum_base_types.get("Result_i64_String") {
                            return ref_nullable(base_idx);
                        }
                    }
                    "parse_f64" => {
                        if let Some(&base_idx) = self.enum_base_types.get("Result_f64_String") {
                            return ref_nullable(base_idx);
                        }
                    }
                    _ => {}
                }
                // Check Vec_new_* for struct names
                if let Some(sname) = canonical.strip_prefix("Vec_new_")
                    && let Some(&(_, vec_ty)) = self.custom_vec_types.get(sname)
                {
                    return ref_nullable(vec_ty);
                }
                if let Some(ret_name) = self
                    .fn_ret_type_names
                    .get(name)
                    .or_else(|| self.fn_ret_type_names.get(canonical))
                {
                    // Check if the function returns Any (generic) — infer concrete type
                    if let Some(ret_ty) = self
                        .fn_ret_types
                        .get(name)
                        .or_else(|| self.fn_ret_types.get(canonical))
                        && *ret_ty == Type::Any
                    {
                        return self.infer_generic_return_type(name, args);
                    }
                    self.type_name_to_val(ret_name)
                } else if let Some(ret_ty) = self
                    .fn_ret_types
                    .get(name)
                    .or_else(|| self.fn_ret_types.get(canonical))
                {
                    if *ret_ty == Type::Any {
                        return self.infer_generic_return_type(name, args);
                    }
                    self.type_to_val(ret_ty)
                } else {
                    ValType::I32
                }
            }
            Operand::FieldAccess {
                struct_name, field, ..
            } => {
                let layout = self
                    .struct_layouts
                    .get(struct_name)
                    .cloned()
                    .unwrap_or_default();
                if let Some((_, fty)) = layout.iter().find(|(n, _)| n == field) {
                    self.type_name_to_val(fty)
                } else {
                    ValType::I32
                }
            }
            Operand::BinOp(op, lhs, _rhs) => {
                match op {
                    // Comparison ops always return bool (I32)
                    BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::And
                    | BinOp::Or => ValType::I32,
                    // Arithmetic ops return the type of their operands
                    _ => self.infer_operand_type(lhs),
                }
            }
            Operand::UnaryOp(_op, inner) => self.infer_operand_type(inner),
            Operand::EnumTag(_) => ValType::I32,
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => self.infer_if_result_type(then_result.as_deref(), else_result.as_deref()),
            _ => ValType::I32,
        }
    }

    /// Infer the concrete return type of a generic function call from the argument types.
    /// For `identity<T>(x: T) -> T`, if called with a String arg, returns String.
    pub(super) fn infer_generic_return_type(&self, fn_name: &str, args: &[Operand]) -> ValType {
        let param_types = self.fn_param_types.get(fn_name);
        if let Some(pts) = param_types {
            // Find the first Any-typed param and use its corresponding arg's type
            for (i, pt) in pts.iter().enumerate() {
                if *pt == Type::Any
                    && let Some(arg) = args.get(i)
                {
                    return self.infer_operand_type(arg);
                }
            }
        }
        // Fallback: return i32
        ValType::I32
    }

    /// Emit unboxing instructions to convert an anyref on the stack to a concrete type.
    pub(super) fn emit_anyref_unbox(&self, f: &mut PeepholeWriter<'_>, target_vt: &ValType) {
        match target_vt {
            ValType::I32 => {
                // anyref → ref.cast (ref i31) → i31.get_s
                f.instruction(&Instruction::RefCastNullable(HeapType::Abstract {
                    shared: false,
                    ty: wasm_encoder::AbstractHeapType::I31,
                }));
                f.instruction(&Instruction::I31GetS);
            }
            ValType::Ref(rt) => {
                // anyref → ref.cast (ref $concrete_type)
                f.instruction(&Instruction::RefCastNullable(rt.heap_type));
            }
            _ => {
                // For i64/f64 we'd need struct boxing — not yet implemented
            }
        }
    }

    /// Emit a default/zero value for a given ValType (used for Unit branches in if-expressions).
    pub(super) fn emit_default_value(&self, f: &mut PeepholeWriter<'_>, vt: &ValType) {
        match vt {
            ValType::I32 => f.instruction(&Instruction::I32Const(0)),
            ValType::I64 => f.instruction(&Instruction::I64Const(0)),
            ValType::F32 => f.instruction(&Instruction::F32Const(0.0)),
            ValType::F64 => f.instruction(&Instruction::F64Const(0.0)),
            ValType::Ref(rt) => f.instruction(&Instruction::RefNull(rt.heap_type)),
            _ => f.instruction(&Instruction::I32Const(0)),
        };
    }
}
