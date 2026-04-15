//! Operand emission for the T3 Wasm GC emitter.
//!
//! Handles MIR operand lowering to Wasm instructions, including constants,
//! locals, field access, binary/unary operations, and type coercion.

use ark_mir::mir::*;
use ark_typecheck::types::Type;
use wasm_encoder::{BlockType, HeapType, Instruction, MemArg, RefType as WasmRefType, ValType};

use super::i31ref as i31;
use super::peephole::PeepholeWriter;
use super::{Ctx, normalize_intrinsic};

impl Ctx {
    pub(super) fn emit_operand(&mut self, f: &mut PeepholeWriter<'_>, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::I32Const(*v));
            }
            Operand::ConstI64(v) => {
                f.instruction(&Instruction::I64Const(*v));
            }
            Operand::ConstF32(v) => {
                f.instruction(&Instruction::F32Const(*v));
            }
            Operand::ConstF64(v) => {
                f.instruction(&Instruction::F64Const(*v));
            }
            Operand::ConstU8(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstU16(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstU32(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstU64(v) => {
                f.instruction(&Instruction::I64Const(*v as i64));
            }
            Operand::ConstI8(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstI16(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstBool(v) => {
                f.instruction(&Instruction::I32Const(if *v { 1 } else { 0 }));
            }
            Operand::ConstChar(c) => {
                f.instruction(&Instruction::I32Const(*c as i32));
            }
            Operand::ConstString(s) => {
                // GC-native: create a GC string via array.new_data
                let bytes = s.as_bytes();
                let len = bytes.len() as u32;
                let seg_idx = self.alloc_string_data(bytes);
                let abs_seg = self.data_segs.len() as u32 + seg_idx;

                if self.opt_level >= 1 {
                    // Static string interning: cache in a global, lazy-init on first access.
                    let global_idx = if let Some(&gidx) = self.string_intern_globals.get(s.as_str())
                    {
                        gidx
                    } else {
                        // Allocate a new global (index 0 = heap_ptr, interned start at 1)
                        let gidx = 1 + self.string_intern_count;
                        self.string_intern_globals.insert(s.clone(), gidx);
                        self.string_intern_count += 1;
                        gidx
                    };

                    let str_ref_nullable = ValType::Ref(WasmRefType {
                        nullable: true,
                        heap_type: HeapType::Concrete(self.string_ty),
                    });
                    // global.get $str_N
                    f.instruction(&Instruction::GlobalGet(global_idx));
                    // ref.is_null
                    f.instruction(&Instruction::RefIsNull);
                    // if (result (ref null $string))
                    f.instruction(&Instruction::If(BlockType::Result(str_ref_nullable)));
                    {
                        // First use: create string and cache it
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(len as i32));
                        f.instruction(&Instruction::ArrayNewData {
                            array_type_index: self.string_ty,
                            array_data_index: abs_seg,
                        });
                        let scratch = self.si(8); // ref scratch local
                        // Use local.set + local.get instead of local.tee to avoid
                        // a wasmtime DRC GC tracking issue: local.tee on a newly
                        // allocated GC ref doesn't register the stack residual in
                        // the VMGcRefActivationsTable, causing a GC panic.
                        f.instruction(&Instruction::LocalSet(scratch));
                        f.instruction(&Instruction::LocalGet(scratch));
                        f.instruction(&Instruction::GlobalSet(global_idx));
                        f.instruction(&Instruction::LocalGet(scratch));
                    }
                    f.instruction(&Instruction::Else);
                    {
                        // Already cached: reuse from global
                        f.instruction(&Instruction::GlobalGet(global_idx));
                    }
                    f.instruction(&Instruction::End);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(len as i32));
                    f.instruction(&Instruction::ArrayNewData {
                        array_type_index: self.string_ty,
                        array_data_index: abs_seg,
                    });
                }
            }
            Operand::Unit => {
                // Unit doesn't push a value
            }
            Operand::Place(Place::Local(id)) => {
                f.instruction(&Instruction::LocalGet(self.local_wasm_idx(id.0)));
            }
            Operand::Place(Place::Field(_, _)) | Operand::Place(Place::Index(_, _)) => {
                f.instruction(&Instruction::I32Const(0));
            }
            Operand::BinOp(op, lhs, rhs) => {
                let lhs_i64 = self.is_i64_like_operand(lhs);
                let lhs_f64 = self.is_f64_like_operand(lhs);
                let rhs_i64 = self.is_i64_like_operand(rhs);
                let rhs_f64 = self.is_f64_like_operand(rhs);
                let need_i64 = lhs_i64 || rhs_i64;
                let need_f64 = lhs_f64 || rhs_f64;
                let lhs_str = self.is_string_like_operand(lhs);
                if lhs_str && matches!(op, BinOp::Eq | BinOp::Ne) {
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
            }
            Operand::UnaryOp(op, inner) => {
                self.emit_operand(f, inner);
                let is_f64 = self.is_f64_like_operand(inner);
                let is_i64 = self.is_i64_like_operand(inner);
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
                    UnaryOp::Not => {
                        f.instruction(&Instruction::I32Eqz);
                    }
                    UnaryOp::BitNot => {
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
            }
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic(name).to_string();
                let lookup_name = name.rsplit("::").next().unwrap_or(name.as_str());
                let is_lookup_builtin = self.is_builtin_name(lookup_name);
                let prefer_user_fn = name.contains("::") && !is_lookup_builtin;
                let effective_builtin = if self.is_builtin_name(&canonical) {
                    canonical.as_str()
                } else {
                    lookup_name
                };
                // Check if this is a builtin — redirect to inline implementation
                let is_http_wrapper = self.http_wrapper_fns.contains(name);
                if (self.is_builtin_name(&canonical) || is_lookup_builtin)
                    && !prefer_user_fn
                    && !is_http_wrapper
                {
                    self.emit_call_builtin_operand(f, effective_builtin, args);
                } else {
                    // Check if callee has Any-typed (generic) params needing boxing
                    let param_types = self
                        .fn_param_types
                        .get(name.as_str())
                        .or_else(|| self.fn_param_types.get(canonical.as_str()))
                        .or_else(|| self.fn_param_types.get(lookup_name))
                        .cloned();
                    for (i, arg) in args.iter().enumerate() {
                        self.emit_operand(f, arg);
                        // Box small integers (bool, char, i32) → i31ref when callee
                        // expects anyref (T = Any generic parameter).  i31ref is an
                        // unboxed tagged immediate — no GC heap allocation.
                        if let Some(ref pts) = param_types
                            && i < pts.len()
                            && pts[i] == Type::Any
                        {
                            let arg_vt = self.infer_operand_type(arg);
                            if arg_vt == ValType::I32 {
                                // i32 → i31ref: WasmGC unboxed scalar (issue #070)
                                i31::emit_bool_to_anyref(f);
                            }
                            // ref types (String, struct, enum) are already anyref-compatible
                        }
                    }
                    if let Some(&fn_idx) = self
                        .fn_map
                        .get(name.as_str())
                        .or_else(|| self.fn_map.get(canonical.as_str()))
                        .or_else(|| self.fn_map.get(lookup_name))
                    {
                        f.instruction(&Instruction::Call(fn_idx));
                    } else {
                        // Unknown function: push zero
                        f.instruction(&Instruction::I32Const(0));
                    }
                    // Unbox anyref return → concrete type based on arg-inferred substitution
                    if let Some(ret_ty) = self
                        .fn_ret_types
                        .get(name.as_str())
                        .or_else(|| self.fn_ret_types.get(canonical.as_str()))
                        .or_else(|| self.fn_ret_types.get(lookup_name))
                        .cloned()
                        && ret_ty == Type::Any
                    {
                        // Infer concrete type from first Any-typed arg
                        let concrete = self.infer_generic_return_type(name, args);
                        self.emit_anyref_unbox(f, &concrete);
                    }
                }
            }
            Operand::IfExpr {
                cond,
                then_body,
                then_result,
                else_body,
                else_result,
            } => {
                // Determine the result type from non-Unit branches
                let result_vt =
                    self.infer_if_result_type(then_result.as_deref(), else_result.as_deref());
                let result_is_anyref = matches!(
                    result_vt,
                    ValType::Ref(WasmRefType {
                        heap_type: HeapType::Abstract {
                            ty: wasm_encoder::AbstractHeapType::Any,
                            ..
                        },
                        ..
                    })
                );
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(result_vt)));
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                match then_result.as_deref() {
                    Some(Operand::Unit) | None => {
                        self.emit_default_value(f, &result_vt);
                    }
                    Some(r) => {
                        self.emit_operand(f, r);
                        // Box small integers to i31ref when if-expression block
                        // returns anyref (generic T = Any result type, issue #070)
                        if result_is_anyref {
                            let r_vt = self.infer_operand_type(r);
                            if r_vt == ValType::I32 {
                                i31::emit_box(f);
                            }
                        }
                    }
                }
                f.instruction(&Instruction::Else);
                for s in else_body {
                    self.emit_stmt(f, s);
                }
                match else_result.as_deref() {
                    Some(Operand::Unit) | None => {
                        self.emit_default_value(f, &result_vt);
                    }
                    Some(r) => {
                        self.emit_operand(f, r);
                        // Box small integers to i31ref when if-expression block
                        // returns anyref (generic T = Any result type, issue #070)
                        if result_is_anyref {
                            let r_vt = self.infer_operand_type(r);
                            if r_vt == ValType::I32 {
                                i31::emit_box(f);
                            }
                        }
                    }
                }
                f.instruction(&Instruction::End);
            }
            Operand::StructInit { name, fields } => {
                // GC-native: push field values in order, then struct.new
                let layout = self.struct_layouts.get(name).cloned().unwrap_or_default();
                let ty_idx = self.struct_gc_types.get(name).copied().unwrap_or(0);
                for (fname, _fty) in &layout {
                    if let Some((_, val)) = fields.iter().find(|(n, _)| n == fname) {
                        self.emit_operand(f, val);
                    } else {
                        // Default value for missing field
                        f.instruction(&Instruction::I32Const(0));
                    }
                }
                f.instruction(&Instruction::StructNew(ty_idx));
            }
            Operand::FieldAccess {
                object,
                struct_name,
                field,
            } => {
                // GC-native: struct.get $T $field_idx
                let layout = self
                    .struct_layouts
                    .get(struct_name)
                    .cloned()
                    .unwrap_or_default();
                let ty_idx = self.struct_gc_types.get(struct_name).copied().unwrap_or(0);
                let field_idx = layout
                    .iter()
                    .position(|(fname, _)| fname == field)
                    .unwrap_or(0) as u32;
                self.emit_operand(f, object);
                f.instruction(&Instruction::StructGet {
                    struct_type_index: ty_idx,
                    field_index: field_idx,
                });
            }
            Operand::EnumInit {
                enum_name,
                variant,
                tag: _,
                payload,
            } => {
                // GC-native: push payload fields, then struct.new $Variant
                let effective_enum_name = if matches!(variant.as_str(), "Ok" | "Err") {
                    self.current_result_enum_name()
                        .unwrap_or_else(|| enum_name.clone())
                } else {
                    enum_name.clone()
                };
                let ty_idx = self
                    .enum_variant_types
                    .get(effective_enum_name.as_str())
                    .and_then(|vs| vs.get(variant.as_str()))
                    .copied()
                    .unwrap_or(0);
                for p in payload.iter() {
                    self.emit_operand(f, p);
                }
                f.instruction(&Instruction::StructNew(ty_idx));
            }
            Operand::EnumTag(inner) => {
                // GC-native: produce ordinal via ref.test chain
                let enum_name = self.infer_enum_name(inner);
                if let Some(variants) = self.enum_defs.get(&enum_name).cloned() {
                    let variant_types: Vec<u32> = variants
                        .iter()
                        .map(|(vname, _)| {
                            self.enum_variant_types
                                .get(enum_name.as_str())
                                .and_then(|vs| vs.get(vname.as_str()))
                                .copied()
                                .unwrap_or(0)
                        })
                        .collect();
                    if variant_types.len() <= 1 {
                        f.instruction(&Instruction::I32Const(0));
                    } else if variant_types.len() >= 3 && self.opt_level >= 1 {
                        // br_on_cast chain: emit once, branch on successful cast
                        let n = variant_types.len();
                        let from_ref = WasmRefType {
                            nullable: true,
                            heap_type: HeapType::Abstract {
                                shared: false,
                                ty: wasm_encoder::AbstractHeapType::Any,
                            },
                        };

                        // Outer block for the i32 result
                        f.instruction(&Instruction::Block(BlockType::Result(ValType::I32)));

                        // Inner blocks (reversed: innermost = first variant)
                        for &vty in variant_types.iter().rev() {
                            f.instruction(&Instruction::Block(BlockType::Result(ValType::Ref(
                                WasmRefType {
                                    nullable: false,
                                    heap_type: HeapType::Concrete(vty),
                                },
                            ))));
                        }

                        // Emit enum value once
                        self.emit_operand(f, inner);

                        // br_on_cast chain: try each variant
                        for (i, &vty) in variant_types.iter().enumerate() {
                            f.instruction(&Instruction::BrOnCast {
                                relative_depth: i as u32,
                                from_ref_type: from_ref,
                                to_ref_type: WasmRefType {
                                    nullable: false,
                                    heap_type: HeapType::Concrete(vty),
                                },
                            });
                        }

                        f.instruction(&Instruction::Unreachable);

                        // End each variant block: drop casted ref, push tag
                        for i in 0..n {
                            f.instruction(&Instruction::End);
                            f.instruction(&Instruction::Drop);
                            f.instruction(&Instruction::I32Const(i as i32));
                            if i < n - 1 {
                                f.instruction(&Instruction::Br((n - 1 - i) as u32));
                            }
                        }

                        // End $done block
                        f.instruction(&Instruction::End);
                    } else {
                        // Nested if-else: ref.test $V0 ? 0 : ref.test $V1 ? 1 : ...
                        for (i, &vty) in variant_types.iter().enumerate() {
                            if i == variant_types.len() - 1 {
                                f.instruction(&Instruction::I32Const(i as i32));
                            } else {
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(
                                    vty,
                                )));
                                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                                    ValType::I32,
                                )));
                                f.instruction(&Instruction::I32Const(i as i32));
                                f.instruction(&Instruction::Else);
                            }
                        }
                        for _ in 0..variant_types.len() - 1 {
                            f.instruction(&Instruction::End);
                        }
                    }
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            Operand::EnumPayload {
                object,
                index,
                enum_name,
                variant_name,
            } => {
                // GC-native: ref.cast to variant type, then struct.get
                let effective_enum_name = if matches!(enum_name.as_str(), "Result" | "Option") {
                    let inferred = self.infer_enum_name(object);
                    if inferred.is_empty() {
                        enum_name.clone()
                    } else {
                        inferred
                    }
                } else {
                    enum_name.clone()
                };
                let effective_variant_name = if enum_name == "Option"
                    && effective_enum_name != *enum_name
                    && effective_enum_name.starts_with("Result")
                {
                    match variant_name.as_str() {
                        "Some" => "Ok",
                        "None" => "Err",
                        _ => variant_name.as_str(),
                    }
                } else {
                    variant_name.as_str()
                };
                let variant_ty = self
                    .enum_variant_types
                    .get(effective_enum_name.as_str())
                    .and_then(|vs| vs.get(effective_variant_name))
                    .copied()
                    .unwrap_or(0);
                self.emit_operand(f, object);
                f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(variant_ty)));
                f.instruction(&Instruction::StructGet {
                    struct_type_index: variant_ty,
                    field_index: *index,
                });
            }
            Operand::LoopExpr { body, result, .. } => {
                for s in body {
                    self.emit_stmt(f, s);
                }
                self.emit_operand(f, result);
            }
            Operand::TryExpr { expr, from_fn } => {
                // ? operator: evaluate expr (returns Result enum ref),
                // if Ok → extract payload, if Err → early return
                self.emit_try_expr(f, expr, from_fn.as_ref());
            }
            Operand::FnRef(name) => {
                if let Some(&idx) = self.fn_map.get(name.as_str()) {
                    f.instruction(&Instruction::I32Const(idx as i32));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            Operand::CallIndirect { callee, args } => {
                for arg in args {
                    self.emit_operand(f, arg);
                }
                self.emit_operand(f, callee);
                // Determine signature from arg types
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
                f.instruction(&Instruction::CallIndirect {
                    type_index,
                    table_index: 0,
                });
            }
            Operand::ArrayInit { elements } => {
                // Allocate array in linear memory: [len:4][elem0:4][elem1:4]...
                let len = elements.len() as u32;
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                f.instruction(&Instruction::GlobalGet(0)); // base ptr (result)
                // Store length
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(len as i32));
                f.instruction(&Instruction::I32Store(ma));
                // Store elements
                for (i, elem) in elements.iter().enumerate() {
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(4 + (i as i32) * 4));
                    f.instruction(&Instruction::I32Add);
                    self.emit_operand(f, elem);
                    f.instruction(&Instruction::I32Store(ma));
                }
                // Bump heap
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const((4 + len * 4) as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
            }
            Operand::IndexAccess { object, index } => {
                self.emit_operand(f, object);
                f.instruction(&Instruction::I32Const(4)); // skip length
                f.instruction(&Instruction::I32Add);
                self.emit_operand(f, index);
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Mul);
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
            }
        }
    }

    pub(super) fn scratch_local_for_valtype(&self, vt: ValType) -> u32 {
        match vt {
            ValType::I64 => self.si(6),
            ValType::F64 => self.si(7),
            ValType::Ref(_) => self.si(8),
            _ => self.si(3), // i32 scratch
        }
    }

    /// Emit an operand, promoting i32 constants to i64/f64 when the other
    /// operand in a binary expression is i64/f64.
    pub(super) fn emit_operand_coerced(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        op: &Operand,
        need_i64: bool,
        need_f64: bool,
    ) {
        match op {
            Operand::ConstI32(v) if need_i64 => {
                f.instruction(&Instruction::I64Const(*v as i64));
            }
            Operand::ConstI32(v) if need_f64 => {
                f.instruction(&Instruction::F64Const(*v as f64));
            }
            _ => {
                self.emit_operand(f, op);
            }
        }
    }

    pub(super) fn emit_binop(
        &self,
        f: &mut PeepholeWriter<'_>,
        op: BinOp,
        lhs_operand: Option<&Operand>,
    ) {
        // Determine operand type from LHS (not destination — comparisons return bool/i32)
        let is_f64 = lhs_operand.is_some_and(|o| self.is_f64_like_operand(o));
        let is_i64 = lhs_operand.is_some_and(|o| self.is_i64_like_operand(o));

        if is_f64 {
            match op {
                BinOp::Add => f.instruction(&Instruction::F64Add),
                BinOp::Sub => f.instruction(&Instruction::F64Sub),
                BinOp::Mul => f.instruction(&Instruction::F64Mul),
                BinOp::Div => f.instruction(&Instruction::F64Div),
                BinOp::Eq => f.instruction(&Instruction::F64Eq),
                BinOp::Ne => f.instruction(&Instruction::F64Ne),
                BinOp::Lt => f.instruction(&Instruction::F64Lt),
                BinOp::Le => f.instruction(&Instruction::F64Le),
                BinOp::Gt => f.instruction(&Instruction::F64Gt),
                BinOp::Ge => f.instruction(&Instruction::F64Ge),
                _ => f.instruction(&Instruction::F64Add),
            };
        } else if is_i64 {
            match op {
                BinOp::Add => f.instruction(&Instruction::I64Add),
                BinOp::Sub => f.instruction(&Instruction::I64Sub),
                BinOp::Mul => f.instruction(&Instruction::I64Mul),
                BinOp::Div => f.instruction(&Instruction::I64DivS),
                BinOp::Mod => f.instruction(&Instruction::I64RemS),
                BinOp::Eq => f.instruction(&Instruction::I64Eq),
                BinOp::Ne => f.instruction(&Instruction::I64Ne),
                BinOp::Lt => f.instruction(&Instruction::I64LtS),
                BinOp::Le => f.instruction(&Instruction::I64LeS),
                BinOp::Gt => f.instruction(&Instruction::I64GtS),
                BinOp::Ge => f.instruction(&Instruction::I64GeS),
                BinOp::And => f.instruction(&Instruction::I64And),
                BinOp::Or => f.instruction(&Instruction::I64Or),
                BinOp::BitAnd => f.instruction(&Instruction::I64And),
                BinOp::BitOr => f.instruction(&Instruction::I64Or),
                BinOp::BitXor => f.instruction(&Instruction::I64Xor),
                BinOp::Shl => f.instruction(&Instruction::I64Shl),
                BinOp::Shr => f.instruction(&Instruction::I64ShrS),
            };
        } else {
            match op {
                BinOp::Add => {
                    f.instruction(&Instruction::I32Add);
                }
                BinOp::Sub => {
                    f.instruction(&Instruction::I32Sub);
                }
                BinOp::Mul => {
                    f.instruction(&Instruction::I32Mul);
                }
                BinOp::Div => {
                    f.instruction(&Instruction::I32DivS);
                }
                BinOp::Mod => {
                    f.instruction(&Instruction::I32RemS);
                }
                BinOp::Eq => {
                    f.instruction(&Instruction::I32Eq);
                }
                BinOp::Ne => {
                    f.instruction(&Instruction::I32Ne);
                }
                BinOp::Lt => {
                    f.instruction(&Instruction::I32LtS);
                }
                BinOp::Le => {
                    f.instruction(&Instruction::I32LeS);
                }
                BinOp::Gt => {
                    f.instruction(&Instruction::I32GtS);
                }
                BinOp::Ge => {
                    f.instruction(&Instruction::I32GeS);
                }
                BinOp::And => {
                    f.instruction(&Instruction::I32And);
                }
                BinOp::Or => {
                    f.instruction(&Instruction::I32Or);
                }
                BinOp::BitAnd => {
                    f.instruction(&Instruction::I32And);
                }
                BinOp::BitOr => {
                    f.instruction(&Instruction::I32Or);
                }
                BinOp::BitXor => {
                    f.instruction(&Instruction::I32Xor);
                }
                BinOp::Shl => {
                    f.instruction(&Instruction::I32Shl);
                }
                BinOp::Shr => {
                    f.instruction(&Instruction::I32ShrS);
                }
            }
        }
    }

    pub(super) fn emit_unaryop(&self, f: &mut PeepholeWriter<'_>, op: UnaryOp, local_id: u32) {
        match op {
            UnaryOp::Neg => {
                if self.f64_locals.contains(&local_id) {
                    f.instruction(&Instruction::F64Neg);
                } else if self.i64_locals.contains(&local_id) {
                    f.instruction(&Instruction::I64Const(0));
                    f.instruction(&Instruction::I64Sub);
                } else {
                    // i32 neg: 0 - val (val is on stack)
                    // We need: push 0 then subtract, but val is already on stack
                    // So: store val, push 0, load val, sub
                    // Simpler approach: multiply by -1
                    f.instruction(&Instruction::I32Const(-1));
                    f.instruction(&Instruction::I32Mul);
                }
            }
            UnaryOp::Not => {
                f.instruction(&Instruction::I32Eqz);
            }
            UnaryOp::BitNot => {
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::I32Xor);
            }
            UnaryOp::SignExtend8 => {
                if self.i64_locals.contains(&local_id) {
                    f.instruction(&Instruction::I64Extend8S);
                } else {
                    f.instruction(&Instruction::I32Extend8S);
                }
            }
            UnaryOp::SignExtend16 => {
                if self.i64_locals.contains(&local_id) {
                    f.instruction(&Instruction::I64Extend16S);
                } else {
                    f.instruction(&Instruction::I32Extend16S);
                }
            }
            UnaryOp::SignExtend32 => {
                f.instruction(&Instruction::I64Extend32S);
            }
        }
    }

    pub(super) fn local_wasm_idx(&self, mir_local_id: u32) -> u32 {
        // In the bridge implementation, MIR local IDs map directly
        // to Wasm local indices (params first, then locals)
        mir_local_id
    }

    pub(super) fn enum_total_size(&self, enum_name: &str) -> u32 {
        let variants = match self.enum_defs.get(enum_name) {
            Some(v) => v,
            None => return 8,
        };
        let max_payload: u32 = variants
            .iter()
            .map(|(_, fields)| {
                fields
                    .iter()
                    .map(|t| match t.as_str() {
                        "f64" | "i64" => 8u32,
                        _ => 4u32,
                    })
                    .sum::<u32>()
            })
            .max()
            .unwrap_or(4);
        4 + max_payload.max(4)
    }
}
