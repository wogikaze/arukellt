//! Operand/expression emission: emit_operand, binary ops, unary ops,
//! numeric conversions, type queries.

use super::*;
use wasm_encoder::MemArg;

impl EmitCtx {
    pub(super) fn emit_operand(&mut self, f: &mut Function, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::I32Const(*v));
            }
            Operand::ConstI64(v) => {
                f.instruction(&Instruction::I64Const(*v));
            }
            Operand::ConstF64(v) => {
                f.instruction(&Instruction::F64Const(*v));
            }
            Operand::ConstF32(v) => {
                f.instruction(&Instruction::F32Const(*v));
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
                // Allocate as length-prefixed string, return pointer
                let ptr = self.alloc_length_prefixed_string(s);
                f.instruction(&Instruction::I32Const(ptr as i32));
            }
            Operand::Place(Place::Local(id)) => {
                f.instruction(&Instruction::LocalGet(id.0));
            }
            Operand::BinOp(op, left, right) => {
                let is_f64 = self.is_f64_operand(left) || self.is_f64_operand(right);
                let is_i64 = !is_f64 && (self.is_i64_operand(left) || self.is_i64_operand(right));
                if is_f64 {
                    // Promote both operands to f64 if needed
                    self.emit_f64_operand(f, left);
                    self.emit_f64_operand(f, right);
                    self.emit_binop_f64(f, op);
                } else if is_i64 {
                    // Promote both operands to i64 if needed
                    self.emit_i64_operand(f, left);
                    self.emit_i64_operand(f, right);
                    self.emit_binop_i64(f, op);
                } else {
                    self.emit_operand(f, left);
                    self.emit_operand(f, right);
                    self.emit_binop(f, op);
                }
            }
            Operand::UnaryOp(op, inner) => {
                self.emit_unaryop(f, op, inner);
            }
            Operand::Call(name, args) => {
                let original_name = name.as_str();
                let lookup_name = original_name.rsplit("::").next().unwrap_or(original_name);
                // Resolve the canonical name. Full-path http entries must not be confused with
                // Vec::get (2 args) or other short-name builtins. Single-arg "get" / 3-arg
                // "request" that resolve to user functions are http wrappers – route them through
                // the http_get / http_request inline paths so the host call is emitted correctly.
                let name = match original_name {
                    "std::host::http::get" | "http::get" => "http_get",
                    "std::host::http::request" | "http::request" => "http_request",
                    _ => normalize_intrinsic_name(lookup_name),
                };
                match name {
                    "to_string" => {
                        // Polymorphic to_string: dispatch based on argument type
                        if let Some(arg) = args.first() {
                            if self.is_string_operand(arg) {
                                // Already a string — just emit it
                                self.emit_operand(f, arg);
                            } else if self.is_f64_operand(arg) {
                                let converted =
                                    Operand::Call("f64_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            } else if self.is_i64_operand(arg) {
                                let converted =
                                    Operand::Call("i64_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            } else if matches!(arg, Operand::ConstBool(_))
                                || self.is_bool_operand(arg)
                            {
                                let converted =
                                    Operand::Call("bool_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            } else if matches!(arg, Operand::ConstChar(_))
                                || matches!(arg, Operand::Place(Place::Local(id)) if self.char_locals.contains(&id.0))
                            {
                                let converted =
                                    Operand::Call("char_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            } else {
                                // Default: i32_to_string
                                let converted =
                                    Operand::Call("i32_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            }
                        }
                    }
                    "i32_to_string" => {
                        // Convert i32 to length-prefixed string on heap
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_I32_TO_STR);
                        // After __i32_to_string: SCRATCH has offset, SCRATCH+4 has len
                        // Allocate length-prefixed string on heap
                        let ma2 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Write length at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma2)); // len
                        f.instruction(&Instruction::I32Store(ma2));
                        // Copy digits from I32BUF area to heap_ptr+4
                        // Use a byte-by-byte copy loop
                        // local for loop counter - we don't have a dedicated local,
                        // so use NWRITTEN as temp storage
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma2)); // i = 0
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // i
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma2)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // mem[heap_ptr + 4 + i] = mem[scratch_offset + i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // i
                        f.instruction(&Instruction::I32Add); // dst = heap+4+i
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // offset
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // i
                        f.instruction(&Instruction::I32Add); // src = offset+i
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i += 1
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma2));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Result = heap_ptr + 4 (data pointer)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap: heap_ptr += 4 + len
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma2));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "bool_to_string" => {
                        // Convert bool (i32) to "true" or "false" string
                        // Pre-allocate both string literals
                        let true_ptr = self.alloc_length_prefixed_string("true");
                        let false_ptr = self.alloc_length_prefixed_string("false");
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            wasm_encoder::ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const(true_ptr as i32));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const(false_ptr as i32));
                        f.instruction(&Instruction::End);
                    }
                    "char_to_string" => {
                        // Convert char (i32 byte) to a length-prefixed string, return data ptr
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        let ma2 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma2));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        } else {
                            f.instruction(&Instruction::I32Const(0));
                        }
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(5));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "String_from" => {
                        // String_from("literal") → allocate length-prefixed string, return ptr
                        if let Some(Operand::ConstString(s)) = args.first() {
                            let ptr = self.alloc_length_prefixed_string(s);
                            f.instruction(&Instruction::I32Const(ptr as i32));
                        } else if let Some(a) = args.first() {
                            // Pass through the inner operand
                            self.emit_operand(f, a);
                        } else {
                            f.instruction(&Instruction::I32Const(0));
                        }
                    }
                    "eq" => {
                        // String equality: eq(a, b) -> bool (i32)
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_STR_EQ);
                    }
                    "concat" => {
                        // String concatenation: concat(a, b) -> new string ptr
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_CONCAT);
                    }
                    "f64_to_string" => {
                        // Convert f64 to length-prefixed string
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_F64_TO_STR);
                    }
                    "i64_to_string" => {
                        // Convert i64 to length-prefixed string
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_I64_TO_STR);
                    }
                    name if name == "Vec_new_i32_with_cap"
                        || name == "Vec_new_i64_with_cap"
                        || name == "Vec_new_f64_with_cap" =>
                    {
                        // Allocate Vec with exact capacity: {len:0, cap:args[0], data_ptr}
                        // No realloc needed if push count <= cap.
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let elem_size: i32 =
                            if name == "Vec_new_i64_with_cap" || name == "Vec_new_f64_with_cap" {
                                8
                            } else {
                                4
                            };

                        // Save start_ptr = current heap into SCRATCH (addr 16)
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));

                        // header[0] = len = 0
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));

                        // header[4] = cap = args[0]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Store(ma));

                        // bump heap past header (12 bytes)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);

                        // header[8] = data_ptr = current heap
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma)); // start_ptr
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));

                        // bump heap past data: heap += cap * elem_size
                        f.instruction(&Instruction::GlobalGet(0));
                        self.emit_operand(f, &args[0]); // cap
                        f.instruction(&Instruction::I32Const(elem_size));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);

                        // result = start_ptr (saved at SCRATCH)
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    name if name == "Vec_new_i32"
                        || name == "Vec_new_String"
                        || name.starts_with("Vec_new_") =>
                    {
                        // Allocate Vec header: {len:0, cap:8, data_ptr}
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Element size: 8 for i64/f64, 4 otherwise
                        let elem_size: i32 = if name == "Vec_new_i64" || name == "Vec_new_f64" {
                            8
                        } else {
                            4
                        };
                        let data_bytes: i32 = 8 * elem_size; // cap=8 * elem_size
                        let total_alloc: i32 = 12 + data_bytes;
                        // len = 0
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // cap = 8
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Store(ma));
                        // bump heap past header (12 bytes)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        // Store data_ptr at header+8
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0)); // data_ptr = current heap
                        f.instruction(&Instruction::I32Store(ma));
                        // bump heap past data
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(data_bytes));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        // result: header ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(total_alloc));
                        f.instruction(&Instruction::I32Sub);
                    }
                    "push" => {
                        // push(v, x): grow if needed, store x at data[len], increment len
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };

                        // Detect element size from the vector operand
                        let elem_size: i32 = args
                            .first()
                            .map(|v| self.vec_elem_size(v) as i32)
                            .unwrap_or(4);
                        let is_i64_elem = args.first().map(|v| self.is_vec_i64(v)).unwrap_or(false);
                        let is_f64_elem = args.first().map(|v| self.is_vec_f64(v)).unwrap_or(false);

                        // Check if len >= cap, if so grow
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // cap
                        f.instruction(&Instruction::I32GeU); // len >= cap?
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // Save new_data_ptr = heap_ptr to SCRATCH+8
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Store(ma));

                            // Bump heap: heap_ptr += old_cap * 2 * elem_size
                            f.instruction(&Instruction::GlobalGet(0));
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // old_cap
                            f.instruction(&Instruction::I32Const(2 * elem_size));
                            f.instruction(&Instruction::I32Mul);
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);

                            // Copy old data byte-by-byte: i=0..len*elem_size
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma)); // i = 0

                            // Pre-copy grow: ensure memory covers entire copy range
                            // needed_end = new_data_ptr + len * elem_size
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // new_data_ptr
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Load(ma)); // len
                            f.instruction(&Instruction::I32Const(elem_size));
                            f.instruction(&Instruction::I32Mul); // len * elem_size
                            f.instruction(&Instruction::I32Add); // new_data_ptr + len*elem_size
                            f.instruction(&Instruction::I32Const(16));
                            f.instruction(&Instruction::I32ShrU);
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::MemorySize(0));
                            f.instruction(&Instruction::I32GtU);
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(elem_size));
                            f.instruction(&Instruction::I32Mul);
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(16));
                            f.instruction(&Instruction::I32ShrU);
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::MemorySize(0));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::MemoryGrow(0));
                            f.instruction(&Instruction::Drop);
                            f.instruction(&Instruction::End);

                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            // if i >= len*elem_size, break
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // i
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Load(ma)); // len
                            f.instruction(&Instruction::I32Const(elem_size));
                            f.instruction(&Instruction::I32Mul); // len*elem_size
                            f.instruction(&Instruction::I32GeU);
                            f.instruction(&Instruction::BrIf(1));
                            // new_data[i] = old_data[i]
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // new_data
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // i
                            f.instruction(&Instruction::I32Add); // dst
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // old_data
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // i
                            f.instruction(&Instruction::I32Add); // src
                            f.instruction(&Instruction::I32Load8U(ma0));
                            f.instruction(&Instruction::I32Store8(ma0));
                            // i += 1
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0));
                            f.instruction(&Instruction::End); // end loop
                            f.instruction(&Instruction::End); // end block

                            // Update v.data_ptr = new_data
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));

                            // Update v.cap = old_cap * 2
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // old_cap
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::I32Mul); // new_cap
                            f.instruction(&Instruction::I32Store(ma));
                        }
                        f.instruction(&Instruction::End); // end if

                        // Now do the actual push: data[len] = value
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(elem_size));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add); // data_ptr + len*elem_size
                        if let Some(x) = args.get(1) {
                            if is_i64_elem {
                                self.emit_i64_operand(f, x);
                            } else if is_f64_elem {
                                self.emit_f64_operand(f, x);
                            } else {
                                self.emit_operand(f, x);
                            }
                        }
                        if is_i64_elem {
                            f.instruction(&Instruction::I64Store(ma));
                        } else if is_f64_elem {
                            f.instruction(&Instruction::F64Store(ma));
                        } else {
                            f.instruction(&Instruction::I32Store(ma));
                        }
                        // Increment len
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));

                        // Return the vec pointer for use as a value expression
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                    }
                    "len" => {
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(v) = args.first() {
                            if self.is_string_operand(v) {
                                // String len: load from ptr - 4
                                self.emit_operand(f, v);
                                f.instruction(&Instruction::I32Const(4));
                                f.instruction(&Instruction::I32Sub);
                                f.instruction(&Instruction::I32Load(ma));
                            } else {
                                // Vec len: load from v+0
                                self.emit_operand(f, v);
                                f.instruction(&Instruction::I32Load(ma));
                            }
                        }
                    }
                    "get_unchecked" => {
                        // get_unchecked(v, i): load data[i]
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let is_i64_elem = args.first().map(|v| self.is_vec_i64(v)).unwrap_or(false);
                        let is_f64_elem = args.first().map(|v| self.is_vec_f64(v)).unwrap_or(false);
                        let elem_size: i32 = if is_i64_elem || is_f64_elem { 8 } else { 4 };
                        // Load data_ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        // Add i*elem_size
                        if let Some(i) = args.get(1) {
                            self.emit_operand(f, i);
                        }
                        f.instruction(&Instruction::I32Const(elem_size));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        if is_i64_elem {
                            f.instruction(&Instruction::I64Load(ma));
                        } else if is_f64_elem {
                            f.instruction(&Instruction::F64Load(ma));
                        } else {
                            f.instruction(&Instruction::I32Load(ma));
                        }
                    }
                    "get" if args.len() >= 2 => {
                        // Vec::get(v, i) -> Option<T>: bounds check, return Some(data[i]) or None.
                        // Guard: only fire for 2-arg calls (Vec.get); single-arg "get" calls are
                        // user functions (e.g. http::get) and are handled by the `other` fallthrough.
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Check i < len
                        if let Some(i_arg) = args.get(1) {
                            self.emit_operand(f, i_arg);
                        }
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // In bounds: construct Some(data[i])
                        // Allocate enum: tag=0 (Some), payload=data[i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0)); // tag = Some
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Load data[i]
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(i_arg) = args.get(1) {
                            self.emit_operand(f, i_arg);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[i]
                        f.instruction(&Instruction::I32Store(ma)); // store payload
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        f.instruction(&Instruction::Else);
                        // Out of bounds: construct None
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1)); // tag = None
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "set" => {
                        // set(v, i, x): store x at data[i]
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let is_i64_elem = args.first().map(|v| self.is_vec_i64(v)).unwrap_or(false);
                        let is_f64_elem = args.first().map(|v| self.is_vec_f64(v)).unwrap_or(false);
                        let elem_size: i32 = if is_i64_elem || is_f64_elem { 8 } else { 4 };
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(i_arg) = args.get(1) {
                            self.emit_operand(f, i_arg);
                        }
                        f.instruction(&Instruction::I32Const(elem_size));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        if let Some(x) = args.get(2) {
                            if is_i64_elem {
                                self.emit_i64_operand(f, x);
                            } else if is_f64_elem {
                                self.emit_f64_operand(f, x);
                            } else {
                                self.emit_operand(f, x);
                            }
                        }
                        if is_i64_elem {
                            f.instruction(&Instruction::I64Store(ma));
                        } else if is_f64_elem {
                            f.instruction(&Instruction::F64Store(ma));
                        } else {
                            f.instruction(&Instruction::I32Store(ma));
                        }
                    }
                    "pop" => {
                        // pop(v) -> Option<i32>: if len > 0, decrement len, return Some(data[len-1]); else None
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Check len > 0
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // Decrement len
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // current len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma)); // store new len
                        // Construct Some(data[new_len])
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0)); // tag = Some
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Load data[new_len]: data_ptr + new_len * 4
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // new len
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[new_len]
                        f.instruction(&Instruction::I32Store(ma)); // store payload
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        f.instruction(&Instruction::Else);
                        // Empty: construct None
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1)); // tag = None
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "sort_i32" => {
                        // Bubble sort in-place
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // We need locals for the sort loop. Since we can't easily declare
                        // new locals mid-function in wasm-encoder, we'll use scratch memory.
                        // scratch[16..32]: sort temporaries
                        // SCRATCH+0 (16): outer i
                        // SCRATCH+4 (20): inner j
                        // SCRATCH+8 (24): n (len)
                        // SCRATCH+12 (28): data_ptr
                        // SCRATCH+16 (32): temp for swap
                        let s_i = SCRATCH;
                        let s_j = SCRATCH + 4;
                        let s_n = SCRATCH + 8;
                        let s_data = SCRATCH + 12;
                        let s_tmp = SCRATCH + 16;
                        // Load n = len(v)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Load data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Outer loop: while i < n-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // Check i < n-1
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1)); // break if i >= n-1
                        // j = 0
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Inner loop: while j < n-i-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // Check j < n-i-1
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1)); // break if j >= n-i-1
                        // Compare data[j] > data[j+1]
                        // addr_j = data_ptr + j*4
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[j]
                        // addr_j1 = data_ptr + (j+1)*4
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[j+1]
                        f.instruction(&Instruction::I32GtS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // Swap data[j] and data[j+1]
                        // tmp = data[j]
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j] = data[j+1]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        // data[j+1] value
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j+1] = tmp
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End); // end if
                        // j++
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue inner
                        f.instruction(&Instruction::End); // end inner loop
                        f.instruction(&Instruction::End); // end inner block
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue outer
                        f.instruction(&Instruction::End); // end outer loop
                        f.instruction(&Instruction::End); // end outer block
                    }
                    "sort_i64" => {
                        // Bubble sort in-place for Vec<i64> — 8-byte elements
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma8 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_j = SCRATCH + 4;
                        let s_n = SCRATCH + 8;
                        let s_data = SCRATCH + 12;
                        let s_tmp = SCRATCH + 16; // 8 bytes for i64
                        // Load n = len(v)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Load data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Outer loop: while i < n-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // j = 0
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Inner loop: while j < n-i-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Compare data[j] > data[j+1] using i64
                        // addr_j = data_ptr + j*8
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I64Load(ma8)); // data[j]
                        // addr_j1 = data_ptr + (j+1)*8
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I64Load(ma8)); // data[j+1]
                        f.instruction(&Instruction::I64GtS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // Swap data[j] and data[j+1]
                        // tmp = data[j]
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I64Load(ma8));
                        f.instruction(&Instruction::I64Store(ma8));
                        // data[j] = data[j+1]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I64Load(ma8));
                        f.instruction(&Instruction::I64Store(ma8));
                        // data[j+1] = tmp
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I64Load(ma8));
                        f.instruction(&Instruction::I64Store(ma8));
                        f.instruction(&Instruction::End); // end if
                        // j++
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue inner
                        f.instruction(&Instruction::End); // end inner loop
                        f.instruction(&Instruction::End); // end inner block
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue outer
                        f.instruction(&Instruction::End); // end outer loop
                        f.instruction(&Instruction::End); // end outer block
                    }
                    "sort_f64" => {
                        // Bubble sort in-place for Vec<f64> — 8-byte elements
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma8 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_j = SCRATCH + 4;
                        let s_n = SCRATCH + 8;
                        let s_data = SCRATCH + 12;
                        let s_tmp = SCRATCH + 16; // 8 bytes for f64
                        // Load n = len(v)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Load data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Outer loop: while i < n-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // j = 0
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Inner loop: while j < n-i-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Compare data[j] > data[j+1] using f64
                        // addr_j = data_ptr + j*8
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::F64Load(ma8)); // data[j]
                        // addr_j1 = data_ptr + (j+1)*8
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::F64Load(ma8)); // data[j+1]
                        f.instruction(&Instruction::F64Gt);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // Swap data[j] and data[j+1]
                        // tmp = data[j]
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::F64Load(ma8));
                        f.instruction(&Instruction::F64Store(ma8));
                        // data[j] = data[j+1]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::F64Load(ma8));
                        f.instruction(&Instruction::F64Store(ma8));
                        // data[j+1] = tmp
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::F64Load(ma8));
                        f.instruction(&Instruction::F64Store(ma8));
                        f.instruction(&Instruction::End); // end if
                        // j++
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue inner
                        f.instruction(&Instruction::End); // end inner loop
                        f.instruction(&Instruction::End); // end inner block
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue outer
                        f.instruction(&Instruction::End); // end outer loop
                        f.instruction(&Instruction::End); // end outer block
                    }
                    "string_len" => {
                        // string_len(s) -> i32: load length from length-prefixed string
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "string_eq" => {
                        // string_eq(a, b) -> bool
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_STR_EQ);
                    }
                    "String_new" => {
                        // Empty string: allocate [len=0] on heap, return data ptr
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "is_empty" => {
                        // is_empty(s): len(s) == 0
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eqz);
                    }
                    "slice" => {
                        // slice(s, start, end) -> String: copy bytes [start..end] to new string
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Compute new_len = end - start, save to SCRATCH+16
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        if let Some(end_arg) = args.get(2) {
                            self.emit_operand(f, end_arg);
                        }
                        if let Some(start_arg) = args.get(1) {
                            self.emit_operand(f, start_arg);
                        }
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // Pre-grow: ensure memory for 4 + new_len bytes
                        self.emit_pre_alloc_grow_from_scratch(f, (SCRATCH + 16) as i32);
                        // Write new_len at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy bytes: mem[heap+4+i] = mem[s+start+i] for i in 0..new_len
                        // Use NWRITTEN as loop counter
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= new_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma)); // new_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // dst = heap + 4 + i
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        // src = s + start + i
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        if let Some(start_arg) = args.get(1) {
                            self.emit_operand(f, start_arg);
                        }
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // result = heap + 4 (data ptr)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // bump heap past len + data
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma)); // new_len
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "starts_with" => {
                        // starts_with(s, prefix) -> bool
                        // Compare first prefix_len bytes of s with prefix
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Check s_len >= prefix_len
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // s_len
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // p_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // Compare byte by byte using NWRITTEN as counter
                        // Store prefix_len in SCRATCH
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // result = 1 (assume true)
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= prefix_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[i] != prefix[i], set result=0 and break
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(2)); // break to Block (exit loop)
                        f.instruction(&Instruction::End);
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Load result
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        // s_len < prefix_len → false
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "ends_with" => {
                        // ends_with(s, suffix) -> bool
                        // Compare last suffix_len bytes of s with suffix
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Check s_len >= suffix_len
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // s_len
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // suffix_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // offset = s_len - suffix_len
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma)); // scratch[0] = offset
                        // suffix_len
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma)); // scratch[4] = suffix_len
                        // i = 0, result = 1
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= suffix_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[offset+i] != suffix[i], set result=0 and break
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma)); // offset
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(2)); // break to Block (exit loop)
                        f.instruction(&Instruction::End);
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "join" => {
                        // join(parts: Vec<String>, sep: String) -> String
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_n = SCRATCH + 4;
                        let s_out_start = SCRATCH + 8;
                        let s_out_pos = SCRATCH + 12;
                        // n = len(parts)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // out_start = heap
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // out_pos = heap + 4  (leave room for length prefix)
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= n, break
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // If i > 0, copy separator
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // sep_len
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            // if j >= sep_len, break
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            if let Some(sep) = args.get(1) {
                                self.emit_operand(f, sep);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32GeU);
                            f.instruction(&Instruction::BrIf(1));
                            // out_pos[j] = sep[j]
                            f.instruction(&Instruction::I32Const(s_out_pos as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            if let Some(sep) = args.get(1) {
                                self.emit_operand(f, sep);
                            }
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load8U(ma0));
                            f.instruction(&Instruction::I32Store8(ma0));
                            // j++
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0));
                            f.instruction(&Instruction::End);
                            f.instruction(&Instruction::End);
                            // out_pos += sep_len
                            f.instruction(&Instruction::I32Const(s_out_pos as i32));
                            f.instruction(&Instruction::I32Const(s_out_pos as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            if let Some(sep) = args.get(1) {
                                self.emit_operand(f, sep);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                        }
                        f.instruction(&Instruction::End); // end if i > 0
                        // Copy current string: str_ptr = parts.data[i]
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if j >= str_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        // str_ptr = parts.data_ptr + i*4, deref to get string ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // str_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // out_pos[j] = str[j]
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        // str_ptr again
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // j++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // out_pos += str_len
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // str_len
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Write total length at out_start
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // Result: out_start + 4 (pointer to data, length-prefixed)
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "split" => {
                        // split(s: String, delim: String) -> Vec<String>
                        // Note: single-char delim only
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_slen = SCRATCH + 4;
                        let s_seg = SCRATCH + 8;
                        let s_vec = SCRATCH + 12;
                        // Create Vec header: len=0, cap=8, data_ptr=heap+12
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.len = 0
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.cap = 8
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.data_ptr = heap + 12
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // Bump heap past header + data (12 + 32 = 44)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(44));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        // Init: slen = len(s), i = 0, seg_start = 0
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Store delim char in I32BUF
                        f.instruction(&Instruction::I32Const(I32BUF as i32));
                        if let Some(d) = args.get(1) {
                            self.emit_operand(f, d);
                        }
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Scan loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= slen, break
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[i] == delim_char
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(I32BUF as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // Allocate substring [seg_start..i]
                            // Write length prefix at heap
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(s_i as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Store(ma)); // heap[0] = seg_len
                            // Copy bytes
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_i as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32GeU);
                            f.instruction(&Instruction::BrIf(1));
                            // heap[4 + j] = s[seg_start + j]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            if let Some(s) = args.first() {
                                self.emit_operand(f, s);
                            }
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load8U(ma0));
                            f.instruction(&Instruction::I32Store8(ma0));
                            // j++
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0));
                            f.instruction(&Instruction::End);
                            f.instruction(&Instruction::End);
                            // Push string ptr (heap+4) into vec
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // data_ptr
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Load(ma)); // vec.len
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Mul);
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // vec.len++
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past string (4 + seg_len)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Load(ma)); // seg_len
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // seg_start = i + 1
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Const(s_i as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                        }
                        f.instruction(&Instruction::End); // end if delim
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Push final segment [seg_start..slen]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma)); // heap[0] = final_seg_len
                        // Copy final segment bytes
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Push final string into vec
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.len++
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // Bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        // Result: vec_ptr
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "parse_i32" => {
                        // parse_i32(s: String) -> Result<i32, String>
                        // Returns enum ptr: tag=0 (Ok) + payload=value, or tag=1 (Err) + payload=err_string
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Save string ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Get string len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Parse: accumulate digits, handle optional leading '-'
                        // result = 0, i = 0, is_neg = 0, is_err = 0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // result = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 0

                        // Check for empty string → error
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);

                        // Check if first char is '-'
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // is_err
                        f.instruction(&Instruction::I32Eqz); // !is_err
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Load8U(ma0)); // first byte
                        f.instruction(&Instruction::I32Const(45)); // '-'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // i = 1
                        // Check that string isn't just "-"
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);

                        // Digit loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if is_err, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::BrIf(1));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte at str[i] and store to SCRATCH+32
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0)); // byte
                        f.instruction(&Instruction::I32Store(ma)); // mem[SCRATCH+32] = byte
                        // Check byte < '0' || byte > '9'
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(57));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::I32Or);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::Br(2)); // break outer block
                        f.instruction(&Instruction::End);
                        // result = result * 10 + (byte - '0')
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32)); // addr for store
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma)); // old_result
                        f.instruction(&Instruction::I32Const(10));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // byte
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub); // digit
                        f.instruction(&Instruction::I32Add); // result*10 + digit
                        f.instruction(&Instruction::I32Store(ma));
                        // i += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block

                        // Check is_err
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // Build Err Result: tag=1, payload=error string ptr
                            // First, create the error string "parse error: invalid integer"
                            let err_msg = b"parse error: invalid integer";
                            let err_len = err_msg.len() as i32;
                            // Allocate string on heap: [len:4][data:N]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma)); // store len at heap_ptr
                            // Copy bytes
                            for (j, &b) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + j as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(b as i32));
                                f.instruction(&Instruction::I32Store8(ma0));
                            }
                            // str_ptr = heap_ptr + 4 (points to data)
                            // Save str_ptr to SCRATCH+32
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past string
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // Build enum: [tag=1, payload=str_ptr]
                            f.instruction(&Instruction::GlobalGet(0)); // enum_base (save for result)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1)); // tag = Err
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // str_ptr
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past enum (8 bytes)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // enum_base is on stack — this will be left for the outer if/else
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Apply negation if is_neg
                            f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // is_neg
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Store(ma)); // result = -result
                            f.instruction(&Instruction::End);
                            // Build Result::Ok(value) enum on heap
                            f.instruction(&Instruction::GlobalGet(0)); // save base
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0)); // tag = Ok
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // result value
                            f.instruction(&Instruction::I32Store(ma));
                            // bump heap
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // enum_base on stack
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "parse_i64" => {
                        // parse_i64(s: String) -> Result<i64, String>
                        // Returns enum ptr: tag=0 (Ok) + i64 payload, or tag=1 (Err) + string payload
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        let ma8 = MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        };
                        // Save string ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Get string len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Initialize: result=0i64, i=0, is_neg=0, is_err=0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I64Const(0));
                        f.instruction(&Instruction::I64Store(ma8)); // result = 0i64
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 0

                        // Check for empty string → error
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);

                        // Check if first char is '-'
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // is_err
                        f.instruction(&Instruction::I32Eqz); // !is_err
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Load8U(ma0)); // first byte
                        f.instruction(&Instruction::I32Const(45)); // '-'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // i = 1
                        // Check that string isn't just "-"
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);

                        // Digit loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if is_err, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::BrIf(1));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte at str[i] and store to SCRATCH+32
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0)); // byte
                        f.instruction(&Instruction::I32Store(ma)); // mem[SCRATCH+32] = byte
                        // Check byte < '0' || byte > '9'
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(57));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::I32Or);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::Br(2)); // break outer block
                        f.instruction(&Instruction::End);
                        // result = result * 10i64 + i64.extend_i32_u(byte - '0')
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32)); // addr for store
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I64Load(ma8)); // old_result
                        f.instruction(&Instruction::I64Const(10));
                        f.instruction(&Instruction::I64Mul);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // byte
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub); // digit (i32)
                        f.instruction(&Instruction::I64ExtendI32U); // digit as i64
                        f.instruction(&Instruction::I64Add); // result*10 + digit
                        f.instruction(&Instruction::I64Store(ma8));
                        // i += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block

                        // Build Result enum on heap
                        // Layout: [tag:4][payload:8] = 12 bytes
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // Build Err Result: tag=1, payload=error string ptr
                            let err_msg = b"parse error: invalid integer";
                            let err_len = err_msg.len() as i32;
                            // Allocate string on heap: [len:4][data:N]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma)); // store len at heap_ptr
                            for (j, &b) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + j as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(b as i32));
                                f.instruction(&Instruction::I32Store8(ma0));
                            }
                            // str_ptr = heap_ptr + 4
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past string
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // Build enum: [tag=1][str_ptr:i32 at offset 4]
                            f.instruction(&Instruction::GlobalGet(0)); // enum_base (result on stack)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1)); // tag = Err
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // str_ptr
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past enum (12 bytes for i64 payload alignment)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(12));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Apply negation if is_neg
                            f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // is_neg
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I64Const(0));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I64Load(ma8));
                            f.instruction(&Instruction::I64Sub);
                            f.instruction(&Instruction::I64Store(ma8)); // result = -result
                            f.instruction(&Instruction::End);
                            // Build Result::Ok(value) enum on heap: [tag=0][i64 payload at offset 4]
                            f.instruction(&Instruction::GlobalGet(0)); // save base (result on stack)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0)); // tag = Ok
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I64Load(ma8)); // i64 result
                            f.instruction(&Instruction::I64Store(ma8));
                            // Bump heap past enum (12 bytes)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(12));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "clock_now" => {
                        // clock_time_get(clock_id=0 (realtime), precision=0, result_ptr=SCRATCH)
                        f.instruction(&Instruction::I32Const(0)); // clock_id = REALTIME
                        f.instruction(&Instruction::I64Const(0)); // precision
                        f.instruction(&Instruction::I32Const(SCRATCH as i32)); // result buffer
                        self.call_fn(f, FN_CLOCK_TIME_GET);
                        f.instruction(&Instruction::Drop); // drop errno
                        // Load i64 result from SCRATCH
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I64Load(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }
                    "random_i32" => {
                        // random_get(buf_ptr=SCRATCH, buf_len=4)
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Const(4));
                        self.call_fn(f, FN_RANDOM_GET);
                        f.instruction(&Instruction::Drop); // drop errno
                        // Load i32 result from SCRATCH
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                    "arg_count" => {
                        self.call_fn(f, FN_ARG_COUNT);
                    }
                    "args" => {
                        self.call_fn(f, FN_ARGS_VEC);
                    }
                    "env_var" => {
                        // env_var(name: String) -> Option<String>
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_ENV_VAR);
                    }
                    "parse_f64" => {
                        // parse_f64(s: String) -> Result<f64, String>
                        // Returns enum ptr: tag=0 (Ok) + f64 payload, or tag=1 (Err) + string payload
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        let ma_f64 = MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        };
                        // Save string ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Get string len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Initialize
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::F64Const(0.0));
                        f.instruction(&Instruction::F64Store(ma_f64)); // result = 0.0
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // saw_dot = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // decimal_count = 0

                        // Check for empty string → error
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);

                        // Check if first char is '-'
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // is_err
                        f.instruction(&Instruction::I32Eqz); // !is_err
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Load8U(ma0)); // first byte
                        f.instruction(&Instruction::I32Const(45)); // '-'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // i = 1
                        // Check that string isn't just "-"
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);

                        // Digit loop (handles digits and '.')
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if is_err, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::BrIf(1));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte at str[i] and store to SCRATCH+32
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0)); // byte
                        f.instruction(&Instruction::I32Store(ma)); // mem[SCRATCH+32] = byte
                        // Check if byte == '.' (46)
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(46)); // '.'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // Check if we already saw a dot → error
                            f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                            f.instruction(&Instruction::Br(3)); // break outer block
                            f.instruction(&Instruction::End);
                            // Set saw_dot = 1
                            f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            // i += 1
                            f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(1)); // continue loop
                        }
                        f.instruction(&Instruction::End); // end dot check
                        // Check byte < '0' || byte > '9'
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(57));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::I32Or);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::Br(2)); // break outer block
                        f.instruction(&Instruction::End);
                        // result = result * 10.0 + f64.convert_i32_u(byte - '0')
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32)); // addr for store
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::F64Load(ma_f64)); // old_result
                        f.instruction(&Instruction::F64Const(10.0));
                        f.instruction(&Instruction::F64Mul);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // byte
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub); // digit (i32)
                        f.instruction(&Instruction::F64ConvertI32U); // digit as f64
                        f.instruction(&Instruction::F64Add); // result * 10.0 + digit
                        f.instruction(&Instruction::F64Store(ma_f64));
                        // if saw_dot: decimal_count += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // saw_dot
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma)); // decimal_count++
                        f.instruction(&Instruction::End);
                        // i += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block

                        // Build Result enum on heap
                        // Layout: [tag:4][payload:8] = 12 bytes
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // Build Err Result: tag=1, payload=error string ptr
                            let err_msg = b"parse error: invalid float";
                            let err_len = err_msg.len() as i32;
                            // Allocate string on heap: [len:4][data:N]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma)); // store len at heap_ptr
                            for (j, &b) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + j as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(b as i32));
                                f.instruction(&Instruction::I32Store8(ma0));
                            }
                            // str_ptr = heap_ptr + 4
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past string
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // Build enum: [tag=1][str_ptr:i32 at offset 4]
                            f.instruction(&Instruction::GlobalGet(0)); // enum_base (result on stack)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1)); // tag = Err
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // str_ptr
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past enum (12 bytes)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(12));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Compute divisor = 10^decimal_count via loop
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::F64Const(1.0));
                            f.instruction(&Instruction::F64Store(ma_f64)); // divisor = 1.0
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // decimal_count
                            f.instruction(&Instruction::I32Eqz);
                            f.instruction(&Instruction::BrIf(1)); // break if 0
                            // divisor *= 10.0
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::F64Const(10.0));
                            f.instruction(&Instruction::F64Mul);
                            f.instruction(&Instruction::F64Store(ma_f64));
                            // decimal_count -= 1
                            f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                            f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0)); // continue
                            f.instruction(&Instruction::End); // end loop
                            f.instruction(&Instruction::End); // end block

                            // Divide result by divisor (only if saw_dot)
                            f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // saw_dot
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::F64Div);
                            f.instruction(&Instruction::F64Store(ma_f64));
                            f.instruction(&Instruction::End);

                            // Apply negation if is_neg
                            f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // is_neg
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::F64Neg);
                            f.instruction(&Instruction::F64Store(ma_f64));
                            f.instruction(&Instruction::End);

                            // Build Result::Ok(value) enum on heap: [tag=0][f64 payload at offset 4]
                            f.instruction(&Instruction::GlobalGet(0)); // save base (result on stack)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0)); // tag = Ok
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::F64Load(ma_f64)); // f64 result
                            f.instruction(&Instruction::F64Store(ma_f64));
                            // Bump heap past enum (12 bytes)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(12));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "fs_read_file" => {
                        // fs_read_file(path: String) -> Result<String, String>
                        // path is a length-prefixed string pointer
                        // Uses WASI path_open + fd_read + fd_close
                        // Returns heap-allocated enum: tag=0(Ok)+str_ptr or tag=1(Err)+err_str_ptr
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };

                        // Save path_ptr to FS_SCRATCH+8
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Get path_len from length prefix (ptr - 4)
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // path_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // path_len
                        f.instruction(&Instruction::I32Store(ma));

                        // Call path_open(dirfd=3, dirflags=0, path, path_len, oflags=0,
                        //   fs_rights_base=FD_READ(2), fs_rights_inheriting=0, fdflags=0, &opened_fd)
                        f.instruction(&Instruction::I32Const(3)); // dirfd = 3 (first preopened dir)
                        f.instruction(&Instruction::I32Const(0)); // dirflags
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // path_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // path_len
                        f.instruction(&Instruction::I32Const(0)); // oflags = 0
                        f.instruction(&Instruction::I64Const(2)); // fs_rights_base = FD_READ
                        f.instruction(&Instruction::I64Const(0)); // fs_rights_inheriting
                        f.instruction(&Instruction::I32Const(0)); // fdflags
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32)); // &opened_fd
                        self.call_fn(f, FN_PATH_OPEN);

                        // Check errno: 0 = success
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // path_open failed → return Err("file open error")
                            let err_msg = b"file open error";
                            let err_len = err_msg.len() as i32;
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma));
                            for (i, &byte) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + i as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(byte as i32));
                                f.instruction(&Instruction::I32Store8(MemArg {
                                    offset: 0,
                                    align: 0,
                                    memory_index: 0,
                                }));
                            }
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // Build Err enum: [tag=1][payload=str_ptr]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // path_open succeeded — read file contents
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Store(ma)); // save buf_start
                            // Skip 4 bytes for length prefix
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);

                            // Read loop
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            {
                                // Set up iovec: [ptr=global(0), len=FS_BUF_SIZE]
                                f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Store(ma));
                                f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                                f.instruction(&Instruction::I32Const(FS_BUF_SIZE as i32));
                                f.instruction(&Instruction::I32Store(ma));

                                // fd_read(fd, &iov, 1, &nread)
                                f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                                f.instruction(&Instruction::I32Load(ma));
                                f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                                f.instruction(&Instruction::I32Const(1));
                                f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                                self.call_fn(f, FN_FD_READ);
                                f.instruction(&Instruction::Drop);

                                // if nread == 0, break
                                f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                                f.instruction(&Instruction::I32Load(ma));
                                f.instruction(&Instruction::I32Eqz);
                                f.instruction(&Instruction::BrIf(1)); // break outer block

                                // Bump heap by nread
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                                f.instruction(&Instruction::I32Load(ma));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::GlobalSet(0));
                                self.emit_heap_grow_check(f);

                                f.instruction(&Instruction::Br(0)); // continue loop
                            }
                            f.instruction(&Instruction::End); // end loop
                            f.instruction(&Instruction::End); // end block

                            // Store total_len at buf_start
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // buf_start
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Sub); // total_len = end - (start+4)
                            f.instruction(&Instruction::I32Store(ma));

                            // fd_close(fd)
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            self.call_fn(f, FN_FD_CLOSE);
                            f.instruction(&Instruction::Drop);

                            // str_ptr = buf_start + 4
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));

                            // Build Ok enum: [tag=0][payload=str_ptr]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "fs_write_file" => {
                        // fs_write_file(path: String, content: String) -> Result<(), String>
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };

                        // Save path_ptr to FS_SCRATCH+8
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Save content_ptr to FS_SCRATCH+28
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Get path_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));

                        // Call path_open(dirfd=3, dirflags=0, path, path_len,
                        //   oflags=O_CREAT|O_TRUNC=9, rights=FD_WRITE(64), inheriting=0, fdflags=0, &opened_fd)
                        f.instruction(&Instruction::I32Const(3));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(9)); // O_CREAT(1) | O_TRUNC(8)
                        f.instruction(&Instruction::I64Const(64)); // FD_WRITE
                        f.instruction(&Instruction::I64Const(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        self.call_fn(f, FN_PATH_OPEN);

                        // Check errno
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // Error → Err("file write error")
                            let err_msg = b"file write error";
                            let err_len = err_msg.len() as i32;
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma));
                            for (i, &byte) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + i as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(byte as i32));
                                f.instruction(&Instruction::I32Store8(MemArg {
                                    offset: 0,
                                    align: 0,
                                    memory_index: 0,
                                }));
                            }
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // Err enum
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Write content using fd_write
                            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                            // content_len = mem[content_ptr - 4]
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            // fd_write(fd, &iov, 1, &nwritten)
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            self.call_fn(f, FN_FD_WRITE);
                            f.instruction(&Instruction::Drop);
                            // fd_close
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            self.call_fn(f, FN_FD_CLOSE);
                            f.instruction(&Instruction::Drop);
                            // Build Ok(()) enum: [tag=0][payload=0]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::End);
                    }
                    "fs_write_bytes" => {
                        // fs_write_bytes(path: String, bytes: Vec<i32>) -> Result<(), String>
                        // Vec<i32> layout: [len: i32, cap: i32, data_ptr: i32] at bytes_ptr
                        // Element i is at data_ptr + i*4
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma8 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };

                        // Save path_ptr to FS_SCRATCH+8
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Get path_len from length prefix (ptr - 4)
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));

                        // Save bytes_vec_ptr to FS_SCRATCH+16
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Save byte_count = mem[bytes_ptr + 0] (Vec len) to FS_SCRATCH+24
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));

                        // Save data_ptr = mem[bytes_ptr + 8] to FS_SCRATCH+28
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));

                        // Save buf_start = GlobalGet(0) to FS_SCRATCH+20 (no heap advance yet)
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));

                        // Init loop counter i = 0 at FS_SCRATCH+32
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));

                        // Copy loop: while i < byte_count: mem8[buf_start + i] = mem32[data_ptr + i*4]
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        {
                            // if i >= byte_count: break
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32GeU);
                            f.instruction(&Instruction::BrIf(1));

                            // dst = buf_start + i
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            // value = mem32[data_ptr + i*4] (truncated to byte)
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Mul);
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store8(ma8));

                            // i++
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));

                            f.instruction(&Instruction::Br(0)); // continue loop
                        }
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block

                        // Call path_open(dirfd=3, dirflags=0, path, path_len, oflags=9, rights=64, inheriting=0, fdflags=0, &opened_fd)
                        f.instruction(&Instruction::I32Const(3));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(9)); // O_CREAT|O_TRUNC
                        f.instruction(&Instruction::I64Const(64)); // FD_WRITE
                        f.instruction(&Instruction::I64Const(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        self.call_fn(f, FN_PATH_OPEN);

                        // Check errno: 0 = success
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // Error → Err("file write error")
                            let err_msg = b"file write error";
                            let err_len = err_msg.len() as i32;
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma));
                            for (i, &byte) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + i as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(byte as i32));
                                f.instruction(&Instruction::I32Store8(MemArg {
                                    offset: 0,
                                    align: 0,
                                    memory_index: 0,
                                }));
                            }
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 36) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                            // Err enum: [tag=1][payload=str_ptr]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 36) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Advance heap by byte_count (align to 4)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(3));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(-4i32));
                            f.instruction(&Instruction::I32And);
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);

                            // Write content: IOV = [ptr=buf_start, len=byte_count]
                            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            // fd_write(fd, &iov, 1, &nwritten)
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            self.call_fn(f, FN_FD_WRITE);
                            f.instruction(&Instruction::Drop);
                            // fd_close
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            self.call_fn(f, FN_FD_CLOSE);
                            f.instruction(&Instruction::Drop);
                            // Build Ok(()) enum: [tag=0][payload=0]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            self.emit_heap_grow_check(f);
                        }
                        f.instruction(&Instruction::End);
                    }
                    "http_get" => {
                        // http_get(url: String) -> Result<String, String>
                        // T1 strings: ptr→data bytes, length at (ptr-4).
                        // Scratch layout (reuses FS_SCRATCH area; http+fs are mutually exclusive):
                        //   FS_SCRATCH+0  (160): url_ptr
                        //   FS_SCRATCH+4  (164): url_len
                        //   FS_SCRATCH+8  (168): resp_str_ptr (heap_top + 4 before host call)
                        //   FS_SCRATCH+12 (172): ret_i32 from host
                        //   FS_SCRATCH+16 (176): abs_len
                        //   FS_SCRATCH+20 (180): enum_ptr
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };

                        // 1. Save url_ptr to scratch[0]
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Store(ma));

                        // 2. Save url_len = *(url_ptr - 4) to scratch[4]
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));

                        // 3. Allocate 4 bytes for length prefix; resp_str_ptr = heap_top + 4
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));

                        // Pre-grow: ensure ≥65536 bytes headroom for response body
                        {
                            let headroom: i32 = 65540;
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(headroom));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(16));
                            f.instruction(&Instruction::I32ShrU);
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::MemorySize(0));
                            f.instruction(&Instruction::I32GtU);
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(headroom));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(16));
                            f.instruction(&Instruction::I32ShrU);
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::MemorySize(0));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::MemoryGrow(0));
                            f.instruction(&Instruction::Drop);
                            f.instruction(&Instruction::End);
                        }

                        // 4. Call http_get(url_ptr, url_len, resp_str_ptr) → ret_i32; save to scratch[12]
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32)); // dest addr
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma)); // url_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // url_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // resp_str_ptr
                        self.call_fn(f, FN_HTTP_GET);
                        f.instruction(&Instruction::I32Store(ma)); // scratch[12] = ret_i32

                        // 5. Compute abs_len = select(-ret, ret, ret < 0); save to scratch[16]
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub); // -ret (val if err)
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // ret (val if ok)
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS); // ret < 0?
                        f.instruction(&Instruction::Select); // abs_len
                        f.instruction(&Instruction::I32Store(ma)); // scratch[16] = abs_len

                        // 6. Write length prefix: mem[resp_str_ptr - 4] = abs_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub); // resp_len_ptr = resp_str_ptr - 4
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // abs_len
                        f.instruction(&Instruction::I32Store(ma));

                        // 7. Bump heap by abs_len
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);

                        // 8. Build Result enum on heap: [tag: i32][str_ptr: i32]
                        //    Save enum_ptr = heap_top to scratch[20]
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));

                        // Write tag at enum_ptr: tag = select(1, 0, ret < 0)
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // addr = enum_ptr
                        f.instruction(&Instruction::I32Const(1)); // val if err
                        f.instruction(&Instruction::I32Const(0)); // val if ok
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS); // ret < 0?
                        f.instruction(&Instruction::Select); // tag
                        f.instruction(&Instruction::I32Store(ma));

                        // Write str_ptr at enum_ptr+4
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add); // enum_ptr + 4
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // resp_str_ptr
                        f.instruction(&Instruction::I32Store(ma));

                        // Bump heap by 8 for enum
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);

                        // Return enum_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "http_request" => {
                        // http_request(method: String, url: String, body: String) -> Result<String, String>
                        // T1 strings: ptr→data bytes, length at (ptr-4).
                        // Scratch layout (reuses FS_SCRATCH area):
                        //   FS_SCRATCH+0  (160): method_ptr
                        //   FS_SCRATCH+4  (164): method_len
                        //   FS_SCRATCH+8  (168): url_ptr
                        //   FS_SCRATCH+12 (172): url_len
                        //   FS_SCRATCH+16 (176): body_ptr
                        //   FS_SCRATCH+20 (180): body_len
                        //   FS_SCRATCH+24 (184): resp_str_ptr
                        //   FS_SCRATCH+28 (188): ret_i32
                        //   FS_SCRATCH+32 (192): abs_len
                        //   FS_SCRATCH+36 (196): enum_ptr
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };

                        // 1. Save method_ptr/len, url_ptr/len, body_ptr/len to scratch
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        self.emit_operand(f, &args[0]); // method_ptr
                        f.instruction(&Instruction::I32Store(ma));

                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // method_len
                        f.instruction(&Instruction::I32Store(ma));

                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        self.emit_operand(f, &args[1]); // url_ptr
                        f.instruction(&Instruction::I32Store(ma));

                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // url_len
                        f.instruction(&Instruction::I32Store(ma));

                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        self.emit_operand(f, &args[2]); // body_ptr
                        f.instruction(&Instruction::I32Store(ma));

                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // body_len
                        f.instruction(&Instruction::I32Store(ma));

                        // 2. Allocate 4 bytes for length prefix; resp_str_ptr = heap_top + 4
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));

                        // Pre-grow: ensure ≥65536 bytes headroom for response body
                        {
                            let headroom: i32 = 65540;
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(headroom));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(16));
                            f.instruction(&Instruction::I32ShrU);
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::MemorySize(0));
                            f.instruction(&Instruction::I32GtU);
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(headroom));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(16));
                            f.instruction(&Instruction::I32ShrU);
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::MemorySize(0));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::MemoryGrow(0));
                            f.instruction(&Instruction::Drop);
                            f.instruction(&Instruction::End);
                        }

                        // 3. Call http_request(m_ptr,m_len,u_ptr,u_len,b_ptr,b_len,resp_ptr)→ret; save to scratch[28]
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32)); // dest addr
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma)); // method_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // method_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // url_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // url_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // body_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // body_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // resp_str_ptr
                        self.call_fn(f, FN_HTTP_REQUEST);
                        f.instruction(&Instruction::I32Store(ma)); // scratch[28] = ret_i32

                        // 4. Compute abs_len; save to scratch[32]
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub); // -ret
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // ret
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS);
                        f.instruction(&Instruction::Select);
                        f.instruction(&Instruction::I32Store(ma)); // scratch[32] = abs_len

                        // 5. Write length prefix: mem[resp_str_ptr - 4] = abs_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));

                        // 6. Bump heap by abs_len
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);

                        // 7. Build Result enum on heap: [tag: i32][str_ptr: i32]
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 36) as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma)); // scratch[36] = enum_ptr

                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 36) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // enum_ptr (addr)
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS);
                        f.instruction(&Instruction::Select); // tag
                        f.instruction(&Instruction::I32Store(ma));

                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 36) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add); // enum_ptr + 4
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // resp_str_ptr
                        f.instruction(&Instruction::I32Store(ma));

                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);

                        // Return enum_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 36) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "map_i32_i32" | "map_String_String" => {
                        // map(vec, fn) -> call __map_i32 helper (String is i32 ptr at Wasm level)
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_MAP_I32);
                    }
                    "filter_i32" | "filter_String" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_FILTER_I32);
                    }
                    "fold_i32_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_FOLD_I32);
                    }
                    "map_i64_i64" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_MAP_I64);
                    }
                    "filter_i64" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_FILTER_I64);
                    }
                    "fold_i64_i64" => {
                        // fold_i64_i64(vec, init: i64, fn): init must be i64
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        if let Some(init) = args.get(1) {
                            self.emit_i64_operand(f, init);
                        }
                        if let Some(fn_arg) = args.get(2) {
                            self.emit_operand(f, fn_arg);
                        }
                        self.call_fn(f, FN_FOLD_I64);
                    }
                    "map_f64_f64" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_MAP_F64);
                    }
                    "filter_f64" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_FILTER_F64);
                    }
                    "map_option_i32_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_MAP_OPT_I32);
                    }
                    "any_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_ANY_I32);
                    }
                    "find_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_FIND_I32);
                    }
                    "HashMap_i32_i32_new" => {
                        self.call_fn(f, FN_HASHMAP_I32_NEW);
                    }
                    "HashMap_i32_i32_insert" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_HASHMAP_I32_INSERT);
                    }
                    "HashMap_i32_i32_get" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_HASHMAP_I32_GET);
                    }
                    "HashMap_i32_i32_contains_key" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_HASHMAP_I32_CONTAINS);
                    }
                    "HashMap_i32_i32_len" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        self.call_fn(f, FN_HASHMAP_I32_LEN);
                    }
                    "Box_new" => {
                        // Box_new(value): allocate sizeof(enum) on heap, copy value, return pointer
                        // For enum payloads, the value is already a pointer to tag+payloads
                        // Box just stores that pointer
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store the value (an enum pointer) at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0)); // heap_ptr
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Return current heap_ptr, then bump by 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "unbox" => {
                        // unbox(box_ptr): load the stored value (enum pointer) from box
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "unwrap" => {
                        // unwrap(opt): Option/Result → payload at offset 4
                        // Assumes tag 0 = Some/Ok (has payload)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "unwrap_or" => {
                        // unwrap_or(opt, default): if Some/Ok return payload, else default
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store opt ptr to scratch
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Check tag
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma)); // load tag
                        f.instruction(&Instruction::I32Eqz); // tag == 0 means Some/Ok
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        // Some: load payload
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        // None: return default
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::End);
                    }
                    "is_some" => {
                        // is_some(opt): tag == 0 (Some)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // load tag
                        f.instruction(&Instruction::I32Eqz); // tag == 0 → true
                    }
                    "is_none" => {
                        // is_none(opt): tag != 0 (None)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // load tag
                        // tag != 0 → tag itself is truthy (None=1 → true)
                    }
                    "sqrt" => {
                        // sqrt(x: f64) -> f64: Wasm native f64.sqrt
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::F64Sqrt);
                    }
                    "abs" => {
                        // abs(x: i32) -> i32: if x < 0 then -x else x
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store x to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // if x < 0 then 0 - x else x
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                    }
                    "min" => {
                        // min(a, b) -> i32: if a <= b then a else b
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32LeS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                    }
                    "max" => {
                        // max(a, b) -> i32: if a >= b then a else b
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                    }
                    "f64_bits_lo" => {
                        // f64_bits_lo(val: f64) -> i32: low 32 bits of IEEE 754
                        let ma8 = MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        };
                        let ma4 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store f64 to scratch
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::F64Store(ma8));
                        // Load low 4 bytes as i32
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma4));
                    }
                    "f64_bits_hi" => {
                        // f64_bits_hi(val: f64) -> i32: high 32 bits of IEEE 754
                        let ma8 = MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        };
                        let ma4 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store f64 to scratch
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::F64Store(ma8));
                        // Load high 4 bytes as i32
                        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Load(ma4));
                    }
                    "panic" => {
                        // panic(msg: String) -> !: print to stderr, then unreachable
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Print "panic: " prefix to stderr
                        let prefix_ptr = self.alloc_length_prefixed_string("panic: ");
                        // Write prefix to stderr (fd=2)
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(prefix_ptr as i32));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32 + 4));
                        f.instruction(&Instruction::I32Const(7)); // "panic: " len
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(2)); // fd=stderr
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        self.call_fn(f, FN_FD_WRITE);
                        f.instruction(&Instruction::Drop);
                        // Write user message to stderr
                        // Store msg ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // iov_base = msg_ptr, iov_len = msg_len
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // msg_ptr
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32 + 4));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // msg_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // msg_len
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(2)); // fd=stderr
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        self.call_fn(f, FN_FD_WRITE);
                        f.instruction(&Instruction::Drop);
                        // Write newline to stderr
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(NEWLINE as i32));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32 + 4));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(2)); // fd=stderr
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        self.call_fn(f, FN_FD_WRITE);
                        f.instruction(&Instruction::Drop);
                        f.instruction(&Instruction::Unreachable);
                    }
                    "assert" => {
                        // assert(cond: bool): if !cond, panic
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed");
                        f.instruction(&Instruction::End);
                    }
                    "assert_eq" => {
                        // assert_eq(a: i32, b: i32): if a != b, panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_eq");
                        f.instruction(&Instruction::End);
                    }
                    "assert_ne" => {
                        // assert_ne(a: i32, b: i32): if a == b, panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_ne");
                        f.instruction(&Instruction::End);
                    }
                    "assert_eq_str" => {
                        // assert_eq_str(a: String, b: String): if !str_eq(a, b), panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        self.call_fn(f, FN_STR_EQ);
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_eq_str");
                        f.instruction(&Instruction::End);
                    }
                    "assert_eq_i64" => {
                        // assert_eq_i64(a: i64, b: i64): if a != b, panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I64Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_eq_i64");
                        f.instruction(&Instruction::End);
                    }
                    "clone" => {
                        // clone(s: String) -> String: deep copy a length-prefixed string
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Save src ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Load src len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Pre-grow: ensure memory can hold heap_ptr + 4 + len
                        self.emit_pre_alloc_grow_from_scratch(f, (SCRATCH + 12) as i32);
                        // Write len at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy bytes: heap_ptr+4+i = src+i for i in 0..len
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // dst[i] = src[i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add); // dst = heap+4+i
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add); // src = src_ptr+i
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Result = heap_ptr + 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "push_char" => {
                        // push_char(s: String, c: char) -> (): append byte to string
                        // String layout: [len:4][data:N] — s points to data (len is at s-4)
                        // We need to copy the string to heap with the char appended
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Store s ptr and char value
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Load old len
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // old_len
                        f.instruction(&Instruction::I32Store(ma));
                        // Pre-grow: ensure memory for old_len + 5 (4 + old_len + 1)
                        self.emit_pre_alloc_grow_from_scratch(f, (SCRATCH + 16) as i32);
                        // Write new_len = old_len + 1 at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy old data bytes
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // heap[4+i] = src[i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Write char byte at heap[4+old_len]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // old_len
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // char value
                        f.instruction(&Instruction::I32Store8(ma0));
                        // Result ptr = heap_ptr + 4
                        // (push_char is void, but we need to update the local var's pointer)
                        // For now push_char returns the new string ptr on stack
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap past [len:4][data:old_len+1]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "to_lower" => {
                        // to_lower(s: String) -> String: ASCII lowercase
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Save src ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Load len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Write len at heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy with lowercase: for each byte, if 'A'<=b<='Z' then b+32
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store(ma));
                        // If 'A' (65) <= byte <= 'Z' (90), add 32
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(65));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(90));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::I32And);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End);
                        // Store byte
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Result = heap_ptr + 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "to_upper" => {
                        // to_upper(s: String) -> String: ASCII uppercase
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store(ma));
                        // If 'a' (97) <= byte <= 'z' (122), subtract 32
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(97));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(122));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::I32And);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "char_at" => {
                        // char_at(s: String, i: i32) -> i32
                        // Returns the byte value at index i (0-based) within the string data
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        if let Some(i) = args.get(1) {
                            self.emit_operand(f, i);
                        }
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                    }
                    "substring" => {
                        // substring(s: String, start: i32, end: i32) -> String
                        // Same as slice — copy bytes [start..end] to new heap allocation
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Compute new_len = end - start, save to SCRATCH+16
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        if let Some(end_arg) = args.get(2) {
                            self.emit_operand(f, end_arg);
                        }
                        if let Some(start_arg) = args.get(1) {
                            self.emit_operand(f, start_arg);
                        }
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // Pre-grow
                        self.emit_pre_alloc_grow_from_scratch(f, (SCRATCH + 16) as i32);
                        // Write new_len at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Use NWRITTEN as loop counter i = 0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= new_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma)); // new_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // dst = heap + 4 + i
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        // src = s + start + i
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        if let Some(start_arg) = args.get(1) {
                            self.emit_operand(f, start_arg);
                        }
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // result = heap + 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // bump heap past len + data
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma)); // new_len
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "trim" => {
                        // trim(s: String) -> String
                        // Strip leading and trailing ASCII whitespace (byte <= 32)
                        // SCRATCH+8  = s_ptr
                        // SCRATCH+12 = s_len
                        // SCRATCH+16 = start (first non-ws index)
                        // SCRATCH+20 = end   (one past last non-ws index)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Save s_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // s_len = mem[s_ptr - 4]
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // start = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Forward scan: while start < s_len && s[start] <= 32: start++
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if start >= s_len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[start] > 32, break (non-whitespace)
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::BrIf(1));
                        // start++
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // end = s_len
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Backward scan: while end > start && s[end-1] <= 32: end--
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if end <= start, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[end-1] > 32, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::BrIf(1));
                        // end--
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // new_len = end - start (may be 0 if all whitespace)
                        // NWRITTEN = new_len
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // Write len at heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy bytes: i = 0..new_len, dst[i] = src[start + i]
                        // Reuse SCRATCH+24 as copy loop counter
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= new_len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // dst = heap + 4 + i
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        // src byte = s[start + i]
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // result = heap + 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // bump heap by 4 + new_len
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                    }
                    "contains" => {
                        // contains(s: String, sub: String) -> bool
                        // Naive O(n*m) substring search
                        // SCRATCH+8  = s_ptr
                        // SCRATCH+12 = s_len
                        // SCRATCH+16 = sub_ptr
                        // SCRATCH+20 = sub_len
                        // SCRATCH+24 = i (outer position in s)
                        // SCRATCH+28 = j (inner position in sub)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Save s_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // s_len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // sub_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        if let Some(sub) = args.get(1) {
                            self.emit_operand(f, sub);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // sub_len
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // result starts as 0 (not found); store at NWRITTEN
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // outer block/loop: for i in 0..=(s_len - sub_len)
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block2 (found-exit)
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block1 (outer loop exit)
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop0 (outer)
                        // if i + sub_len > s_len, exit outer
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::BrIf(1)); // exit block1
                        // j = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // inner block/loop: for j in 0..sub_len
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block_inner_exit
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop_inner
                        // if j >= sub_len, match found
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // found: set result=1, break to block2
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(5)); // exit block2 (found)
                        f.instruction(&Instruction::End);
                        // if s[i+j] != sub[j], break inner
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::BrIf(1)); // exit block_inner_exit
                        // j++
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue loop_inner
                        f.instruction(&Instruction::End); // end loop_inner
                        f.instruction(&Instruction::End); // end block_inner_exit
                        // i++
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue loop0
                        f.instruction(&Instruction::End); // end loop0
                        f.instruction(&Instruction::End); // end block1
                        f.instruction(&Instruction::End); // end block2
                        // return result
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "index_of" => {
                        // index_of(s: String, sub: String) -> i32
                        // Returns byte position of first occurrence of sub in s, or -1 if not found.
                        // Memory layout identical to `contains`:
                        //   SCRATCH+8  = s_ptr
                        //   SCRATCH+12 = s_len
                        //   SCRATCH+16 = sub_ptr
                        //   SCRATCH+20 = sub_len
                        //   SCRATCH+24 = i (outer position in s)
                        //   SCRATCH+28 = j (inner position in sub)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Save s_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // s_len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // sub_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        if let Some(sub) = args.get(1) {
                            self.emit_operand(f, sub);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // sub_len
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // result starts as -1 (not found); store at NWRITTEN
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(-1i32));
                        f.instruction(&Instruction::I32Store(ma));
                        // outer block/loop: for i in 0..=(s_len - sub_len)
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block2 (found-exit)
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block1 (outer loop exit)
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop0 (outer)
                        // if i + sub_len > s_len, exit outer
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::BrIf(1)); // exit block1
                        // j = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // inner block/loop: for j in 0..sub_len
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block_inner_exit
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop_inner
                        // if j >= sub_len, match found
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // found: store i into result, break to block2
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(5)); // exit block2 (found)
                        f.instruction(&Instruction::End);
                        // if s[i+j] != sub[j], break inner
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::BrIf(1)); // exit block_inner_exit
                        // j++
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue loop_inner
                        f.instruction(&Instruction::End); // end loop_inner
                        f.instruction(&Instruction::End); // end block_inner_exit
                        // i++
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue loop0
                        f.instruction(&Instruction::End); // end loop0
                        f.instruction(&Instruction::End); // end block1
                        f.instruction(&Instruction::End); // end block2
                        // return result (-1 or position i)
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "replace" => {
                        // replace(s: String, from: String, to: String) -> String
                        // Replace all non-overlapping occurrences of `from` in `s` with `to`
                        // Memory layout:
                        //   SCRATCH+8  = s_ptr
                        //   SCRATCH+12 = s_len
                        //   SCRATCH+16 = from_ptr
                        //   SCRATCH+20 = from_len
                        //   SCRATCH+24 = to_ptr
                        //   SCRATCH+28 = to_len  (byte 44 — last slot before I32BUF at 48)
                        //   NWRITTEN=8 = i (position in s)
                        //   FS_SCRATCH=160 = buf_start (data start, after 4-byte len prefix)
                        //   FS_NREAD=164   = j (inner match/copy loop counter)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Save inputs
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        if let Some(from) = args.get(1) {
                            self.emit_operand(f, from);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        if let Some(to) = args.get(2) {
                            self.emit_operand(f, to);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // buf_start = global[0] + 4  (reserve 4 bytes for length prefix)
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        // i = 0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Main loop: while i < s_len
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block_outer_exit
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop_outer
                        // if i >= s_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Check if from_len == 0: if so, just copy s[i] and advance
                        // (to avoid infinite loop; also handle zero-length from)
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // from_len==0: copy s[i] to output and i++
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(1)); // continue loop_outer
                        f.instruction(&Instruction::End); // end if from_len==0
                        // Match check: does s[i..i+from_len] == from?
                        // FS_NREAD = j (inner counter)
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // inner block/loop for match
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block_match_exit (no-match)
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop_match
                        // if j >= from_len: full match
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // Match found: copy `to` bytes to heap
                        // FS_NREAD = k (copy counter)
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block_copy_exit
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop_copy
                        // if k >= to_len, break copy
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1)); // exit block_copy_exit
                        // mem[heap] = to[k]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 24) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        // k++
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue loop_copy
                        f.instruction(&Instruction::End); // end loop_copy
                        f.instruction(&Instruction::End); // end block_copy_exit
                        // i += from_len
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(3)); // continue loop_outer (If=0, loop_match=1, block_match_exit=2, loop_outer=3)
                        f.instruction(&Instruction::End); // end if j>=from_len (match found)
                        // if i+j >= s_len or s[i+j] != from[j]: break (no match)
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1)); // exit block_match_exit
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::BrIf(1)); // exit block_match_exit (mismatch)
                        // j++
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue loop_match
                        f.instruction(&Instruction::End); // end loop_match
                        f.instruction(&Instruction::End); // end block_match_exit (no match)
                        // No match at i: copy s[i] to output and i++
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue loop_outer
                        f.instruction(&Instruction::End); // end loop_outer
                        f.instruction(&Instruction::End); // end block_outer_exit
                        // Store length prefix: mem[buf_start - 4] = global[0] - buf_start
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // return buf_start
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "read_int" => {
                        // read_int() -> i32: buffered stdin, no heap allocation.
                        // FN_GET_BYTE reads 65KB at a time from fd=0 into STDIN_BUF (4096).
                        // Scratch: SCRATCH(16)=sign, SCRATCH+4(20)=result, SCRATCH+8(24)=byte.
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };

                        // sign = 1, result = 0
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));

                        // Phase 1: skip whitespace, detect '-' or first digit
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // $exit_p1
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // $loop_p1
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        self.call_fn(f, FN_GET_BYTE);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(-1i32));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::BrIf(1)); // EOF -> exit
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::BrIf(0)); // whitespace -> loop
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(45));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Const(-1i32));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(2)); // '-' -> sign=-1, exit
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma)); // result = digit
                        f.instruction(&Instruction::Br(1)); // exit p1
                        f.instruction(&Instruction::End); // end $loop_p1
                        f.instruction(&Instruction::End); // end $exit_p1

                        // Phase 2: accumulate remaining digits
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // $exit_p2
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // $loop_p2
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        self.call_fn(f, FN_GET_BYTE);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(-1i32));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::BrIf(1)); // EOF
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::BrIf(1)); // < '0'
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(57));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::BrIf(1)); // > '9'
                        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(10));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end $loop_p2
                        f.instruction(&Instruction::End); // end $exit_p2

                        // return result * sign
                        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Mul);
                    }
                    "read_line" => {
                        // read_line() -> String
                        // Read from stdin (fd=0) one byte at a time until '\n' or EOF
                        // IOV_BASE (0): iov.buf (updated each iteration)
                        // IOV_BASE+4 (4): iov.len = 1
                        // FS_NREAD (164): nread result from fd_read
                        // FS_SCRATCH (160): buf_start (heap pointer after length slot)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Set iov.len = 1 (constant for all iterations)
                        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        // buf_start = global[0] + 4 (reserve 4 bytes for length prefix)
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // advance heap by 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        // Read loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // block_exit
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // loop_read
                        // iov.buf = global[0] (current write position)
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // fd_read(fd=0, iov_ptr=0, iov_count=1, nread_ptr=FS_NREAD)
                        f.instruction(&Instruction::I32Const(0)); // fd = stdin
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        self.call_fn(f, FN_FD_READ);
                        f.instruction(&Instruction::Drop); // drop errno
                        // if nread == 0: EOF, break
                        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::BrIf(1)); // break block_exit
                        // Load the byte just read (at global[0])
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store(ma));
                        // if byte == '\n' (10): break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(10));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::BrIf(1)); // break block_exit
                        // if byte == '\r' (13): skip (don't advance heap)
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(13));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Br(1)); // continue loop_read
                        f.instruction(&Instruction::End);
                        // Valid byte: advance heap by 1
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        self.emit_heap_grow_check(f);
                        f.instruction(&Instruction::Br(0)); // continue loop_read
                        f.instruction(&Instruction::End); // end loop_read
                        f.instruction(&Instruction::End); // end block_exit
                        // Store length: mem[buf_start - 4] = global[0] - buf_start
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // return buf_start
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "sort_String" => {
                        // sort_String(v: Vec<String>): bubble sort comparing strings lexicographically
                        // Uses __str_eq helper and byte comparison for ordering
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_j = SCRATCH + 4;
                        let s_n = SCRATCH + 8;
                        let s_data = SCRATCH + 12;
                        let s_tmp = SCRATCH + 16;
                        // n = len(v)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Outer loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // j = 0
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Inner loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Compare strings: data[j] > data[j+1] lexicographically
                        // Load ptr_a = data[j], ptr_b = data[j+1]
                        // Use scratch area at SCRATCH+20..SCRATCH+40
                        let s_ptr_a = SCRATCH + 20;
                        let s_ptr_b = SCRATCH + 24;
                        let s_len_a = SCRATCH + 28;
                        let s_len_b = SCRATCH + 32;
                        let s_k = SCRATCH + 36;
                        let s_min_len = SCRATCH + 40;
                        let s_swap = SCRATCH + 44;
                        // ptr_a = *(data_ptr + j*4)
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // ptr_b = *(data_ptr + (j+1)*4)
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // len_a, len_b (from length-prefix at ptr-4)
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // min_len = if len_a < len_b then len_a else len_b
                        f.instruction(&Instruction::I32Const(s_min_len as i32));
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::I32Store(ma));
                        // swap = 0; k = 0
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Compare loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_min_len as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // byte_a = ptr_a[k], byte_b = ptr_b[k]
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(2)); // break compare loop
                        f.instruction(&Instruction::End);
                        // if byte_a < byte_b → no swap, break
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::BrIf(1)); // break compare loop
                        // k++
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end compare loop
                        f.instruction(&Instruction::End); // end compare block
                        // If bytes were equal, compare lengths: swap if len_a > len_b
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // If swap needed, swap data[j] and data[j+1]
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // tmp = data[j]
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j] = data[j+1]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j+1] = tmp
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End); // end if swap
                        // j++
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end inner loop
                        f.instruction(&Instruction::End); // end inner block
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end outer loop
                        f.instruction(&Instruction::End); // end outer block
                    }
                    "contains_i32" => {
                        // contains_i32(v: Vec<i32>, x: i32) -> bool
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_n = SCRATCH + 4;
                        let s_data = SCRATCH + 8;
                        let s_target = SCRATCH + 12;
                        // n = len(v)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // target value
                        f.instruction(&Instruction::I32Const(s_target as i32));
                        if let Some(x) = args.get(1) {
                            self.emit_operand(f, x);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Search loop with block returning i32
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Result(
                            wasm_encoder::ValType::I32,
                        )));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= n, break (not found)
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if data[i] == target, found
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_target as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::Br(3)); // break outer block with 1
                        f.instruction(&Instruction::End);
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end inner block
                        f.instruction(&Instruction::I32Const(0)); // not found
                        f.instruction(&Instruction::End); // end outer block
                    }
                    "contains_String" => {
                        // contains_String(v: Vec<String>, x: String) -> bool
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_n = SCRATCH + 4;
                        let s_data = SCRATCH + 8;
                        let s_target = SCRATCH + 12;
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_target as i32));
                        if let Some(x) = args.get(1) {
                            self.emit_operand(f, x);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Result(
                            wasm_encoder::ValType::I32,
                        )));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // str_eq(data[i], target)
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_target as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        self.call_fn(f, FN_STR_EQ);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::Br(3));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::End);
                    }
                    "reverse_i32" => {
                        // reverse_i32(v: Vec<i32>): in-place front/back swap
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let s_lo = SCRATCH;
                        let s_hi = SCRATCH + 4;
                        let s_data = SCRATCH + 8;
                        let s_tmp = SCRATCH + 12;
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeS);
                        f.instruction(&Instruction::BrIf(1));
                        // tmp = data[lo]
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[lo] = data[hi]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[hi] = tmp
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // lo++, hi--
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                    }
                    "reverse_String" => {
                        // reverse_String: same layout as i32 (4-byte string ptrs)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let s_lo = SCRATCH;
                        let s_hi = SCRATCH + 4;
                        let s_data = SCRATCH + 8;
                        let s_tmp = SCRATCH + 12;
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeS);
                        f.instruction(&Instruction::BrIf(1));
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Const(s_lo as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Const(s_hi as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                    }
                    "remove_i32" => {
                        // remove_i32(v: Vec<i32>, index: i32): shift left, decrement len
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let s_i = SCRATCH;
                        let s_n = SCRATCH + 4;
                        let s_data = SCRATCH + 8;
                        let s_vec = SCRATCH + 12;
                        // vec ptr
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // n = len - 1
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = index
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        if let Some(idx) = args.get(1) {
                            self.emit_operand(f, idx);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // shift loop: while i < n
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // data[i] = data[i+1]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // v.len = n
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                    }
                    "f32_to_string" => {
                        // f32_to_string(x: f32) -> String: promote to f64 then use f64_to_string
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::F64PromoteF32);
                        self.call_fn(f, FN_F64_TO_STR);
                    }
                    // --- Scalar type conversion functions (issue #040) ---
                    // Narrow types (u8/u16/i8/i16) are all stored as i32 in Wasm.
                    // Widening to i32 is a no-op; narrowing applies masking/sign-extension.
                    "u8_to_i32" | "u16_to_i32" | "i8_to_i32" | "i16_to_i32" => {
                        // All these types are already i32 in Wasm — identity conversion.
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                    }
                    "i32_to_u8" => {
                        // Mask to 8-bit unsigned: x & 0xFF
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Const(0xFF));
                        f.instruction(&Instruction::I32And);
                    }
                    "i32_to_u16" => {
                        // Mask to 16-bit unsigned: x & 0xFFFF
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Const(0xFFFF));
                        f.instruction(&Instruction::I32And);
                    }
                    "i32_to_i8" => {
                        // Sign-extend from 8 bits: (x << 24) >> 24
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Extend8S);
                    }
                    "i32_to_i16" => {
                        // Sign-extend from 16 bits: (x << 16) >> 16
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Extend16S);
                    }
                    "u32_to_u64" => {
                        // Zero-extend i32 to i64 (unsigned): i64.extend_i32_u
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I64ExtendI32U);
                    }
                    "i32_to_i64" => {
                        // Sign-extend i32 to i64: i64.extend_i32_s
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I64ExtendI32S);
                    }
                    "u64_to_u32" | "i64_to_i32" => {
                        // Truncate i64 to i32: i32.wrap_i64
                        if let Some(a) = args.first() {
                            self.emit_i64_operand(f, a);
                        }
                        f.instruction(&Instruction::I32WrapI64);
                    }
                    "f32_to_f64" => {
                        // Promote f32 to f64: f64.promote_f32
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::F64PromoteF32);
                    }
                    "f64_to_f32" => {
                        // Demote f64 to f32: f32.demote_f64
                        if let Some(a) = args.first() {
                            self.emit_f64_operand(f, a);
                        }
                        f.instruction(&Instruction::F32DemoteF64);
                    }
                    "exit" | "proc_exit" => {
                        // process::exit(code: i32) -> !: call WASI proc_exit
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        } else {
                            f.instruction(&Instruction::I32Const(0));
                        }
                        self.call_fn(f, FN_PROC_EXIT);
                        f.instruction(&Instruction::Unreachable);
                    }
                    other => {
                        // Type-aware argument emission for user functions
                        let lookup_name = other.rsplit("::").next().unwrap_or(other);
                        let param_types = self
                            .fn_param_types
                            .get(other)
                            .or_else(|| self.fn_param_types.get(lookup_name))
                            .cloned();
                        for (i, a) in args.iter().enumerate() {
                            let is_i64_param = param_types
                                .as_ref()
                                .and_then(|pts| pts.get(i))
                                .is_some_and(|t| matches!(t, ark_typecheck::types::Type::I64));
                            let is_f64_param = param_types
                                .as_ref()
                                .and_then(|pts| pts.get(i))
                                .is_some_and(|t| matches!(t, ark_typecheck::types::Type::F64));
                            if is_i64_param {
                                self.emit_i64_operand(f, a);
                            } else if is_f64_param {
                                self.emit_f64_operand(f, a);
                            } else {
                                self.emit_operand(f, a);
                            }
                        }
                        if let Some(idx) = self
                            .resolve_fn(other)
                            .or_else(|| self.resolve_fn(lookup_name))
                        {
                            f.instruction(&Instruction::Call(idx));
                        } else {
                            f.instruction(&Instruction::I32Const(0));
                        }
                    }
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
                self.emit_operand(f, cond);
                // Determine the block result type from the then branch result
                let block_valtype = if let Some(r) = then_result {
                    if self.is_i64_operand(r) {
                        ValType::I64
                    } else if self.is_f64_operand(r) {
                        ValType::F64
                    } else {
                        ValType::I32
                    }
                } else {
                    ValType::I32
                };
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    block_valtype,
                )));
                // Track depth for break/continue
                if let Some(d) = self.loop_depths.last_mut() {
                    *d += 1;
                }
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                if let Some(r) = then_result {
                    if block_valtype == ValType::I64 {
                        self.emit_i64_operand(f, r);
                    } else if block_valtype == ValType::F64 {
                        self.emit_f64_operand(f, r);
                    } else {
                        self.emit_operand(f, r);
                    }
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                f.instruction(&Instruction::Else);
                for s in else_body {
                    self.emit_stmt(f, s);
                }
                if let Some(r) = else_result {
                    if matches!(r.as_ref(), Operand::Unit) {
                        // Dead branch in exhaustive match — unreachable satisfies any type
                        f.instruction(&Instruction::Unreachable);
                    } else if block_valtype == ValType::I64 {
                        self.emit_i64_operand(f, r);
                    } else if block_valtype == ValType::F64 {
                        self.emit_f64_operand(f, r);
                    } else {
                        self.emit_operand(f, r);
                    }
                } else if block_valtype == ValType::I64 {
                    f.instruction(&Instruction::I64Const(0));
                } else if block_valtype == ValType::F64 {
                    f.instruction(&Instruction::F64Const(0.0));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                f.instruction(&Instruction::End);
                if let Some(d) = self.loop_depths.last_mut() {
                    *d -= 1;
                }
            }
            Operand::StructInit { name, fields } => {
                let ma_i32 = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                let ma_f64 = MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                };
                let total_size = self.struct_total_size(name).max(fields.len() as u32 * 4);
                let layout_fields = self.struct_layouts.get(name).cloned().unwrap_or_default();
                let save_local = self.struct_init_local_base + self.struct_init_depth;
                self.struct_init_depth += 1;
                // Save base = current heap_ptr to a Wasm local (not scratch memory)
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::LocalSet(save_local));
                // Bump heap for this struct
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(total_size as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                self.emit_heap_grow_check(f);
                // Store each field at saved_base + field_offset
                let mut offset = 0u32;
                for (i, (_fname, fval)) in fields.iter().enumerate() {
                    let (fsize, is_f64, is_i64) = layout_fields
                        .get(i)
                        .map(|(_, ftype)| Self::field_type_info(ftype))
                        .unwrap_or((4, false, false));
                    // Load saved base from Wasm local
                    f.instruction(&Instruction::LocalGet(save_local));
                    if offset > 0 {
                        f.instruction(&Instruction::I32Const(offset as i32));
                        f.instruction(&Instruction::I32Add);
                    }
                    self.emit_operand(f, fval);
                    if is_f64 {
                        f.instruction(&Instruction::F64Store(ma_f64));
                    } else if is_i64 {
                        f.instruction(&Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    } else {
                        f.instruction(&Instruction::I32Store(ma_i32));
                    }
                    offset += fsize;
                }
                self.struct_init_depth -= 1;
                // Push saved base as result
                f.instruction(&Instruction::LocalGet(save_local));
            }
            Operand::FieldAccess {
                object,
                struct_name,
                field,
            } => {
                let ma_i32 = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                let ma_f64 = MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                };
                let (field_offset, is_f64, is_i64) = self.struct_field_info(struct_name, field);
                self.emit_operand(f, object);
                if field_offset > 0 {
                    f.instruction(&Instruction::I32Const(field_offset as i32));
                    f.instruction(&Instruction::I32Add);
                }
                if is_f64 {
                    f.instruction(&Instruction::F64Load(ma_f64));
                } else if is_i64 {
                    f.instruction(&Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                } else {
                    f.instruction(&Instruction::I32Load(ma_i32));
                }
            }
            Operand::EnumInit {
                enum_name,
                variant,
                tag,
                payload,
            } => {
                let ma_i32 = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                let ma_f64 = MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                };
                let total_size = self.enum_variant_total_size(enum_name, variant);
                let depth = self.enum_init_depth;
                self.enum_init_depth += 1;
                let save_local = self.enum_init_local_base + depth;
                // Pre-allocate: bump heap pointer and save base to Wasm local
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::LocalSet(save_local));
                // Bump heap pointer past this enum's allocation
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(total_size as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                self.emit_heap_grow_check(f);
                // Store tag at base + 0
                f.instruction(&Instruction::LocalGet(save_local));
                f.instruction(&Instruction::I32Const(*tag));
                f.instruction(&Instruction::I32Store(ma_i32));
                // Store each payload value with proper type
                for (i, pval) in payload.iter().enumerate() {
                    let (offset, is_f64, is_i64) = self.enum_payload_info(enum_name, variant, i);
                    // Load saved base pointer from Wasm local
                    f.instruction(&Instruction::LocalGet(save_local));
                    f.instruction(&Instruction::I32Const(offset as i32));
                    f.instruction(&Instruction::I32Add);
                    self.emit_operand(f, pval);
                    if is_f64 {
                        f.instruction(&Instruction::F64Store(ma_f64));
                    } else if is_i64 {
                        f.instruction(&Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    } else {
                        f.instruction(&Instruction::I32Store(ma_i32));
                    }
                }
                // Push base pointer as result
                f.instruction(&Instruction::LocalGet(save_local));
                self.enum_init_depth -= 1;
            }
            Operand::EnumTag(inner) => {
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // Load tag (i32) from enum pointer at offset 0
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Load(ma));
            }
            Operand::EnumPayload {
                object,
                index,
                enum_name,
                variant_name,
            } => {
                let (offset, is_f64, is_i64) =
                    self.enum_payload_info(enum_name, variant_name, *index as usize);
                self.emit_operand(f, object);
                f.instruction(&Instruction::I32Const(offset as i32));
                f.instruction(&Instruction::I32Add);
                if is_f64 {
                    f.instruction(&Instruction::F64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                } else if is_i64 {
                    f.instruction(&Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                } else {
                    f.instruction(&Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }));
                }
            }
            Operand::LoopExpr {
                init: _,
                body,
                result,
            } => {
                for stmt in body {
                    self.emit_stmt(f, stmt);
                }
                self.emit_operand(f, result);
            }
            Operand::TryExpr { expr, from_fn } => {
                // expr? on Result<T, E>:
                // 1. Evaluate expr → Result ptr
                // 2. Store to SCRATCH+24
                // 3. Load tag: if Err (tag=1), optionally convert via From, then return
                // 4. If Ok (tag=0), load payload at offset 4
                f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                self.emit_operand(f, expr);
                f.instruction(&Instruction::I32Store(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));

                // Check tag
                f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                // Stack: [tag]
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                {
                    if let Some(from_fn_name) = from_fn {
                        if let Some(from_idx) = self.resolve_fn(from_fn_name) {
                            let ma = MemArg {
                                offset: 0,
                                align: 2,
                                memory_index: 0,
                            };
                            let ma4 = MemArg {
                                offset: 4,
                                align: 2,
                                memory_index: 0,
                            };

                            // Step 1: Allocate new Result (8 bytes: [tag][payload])
                            // Push dest addr FIRST, then produce value, then store
                            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 32));
                            self.emit_bump_alloc(f, 8);
                            // stack: [SCRATCH+32, result_ptr]
                            f.instruction(&Instruction::I32Store(ma));
                            // result_ptr saved at SCRATCH+32. Stack: []

                            // Step 2: Store tag=1 at result_ptr+0
                            // Load result_ptr (address), push 1 (value), store
                            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            // Stack: []

                            // Step 3: Call From::from(err_payload) and store at result_ptr+4
                            // Push result_ptr (address for final store)
                            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 32));
                            f.instruction(&Instruction::I32Load(ma));
                            // Load err payload from original Result ptr + 4
                            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Load(ma4));
                            // Call From::from(err) → converted_err
                            f.instruction(&Instruction::Call(from_idx));
                            // stack: [result_ptr, converted_err]
                            f.instruction(&Instruction::I32Store(ma4));
                            // stored converted_err at result_ptr+4. Stack: []

                            // Step 4: Return new Result ptr
                            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::Return);
                        } else {
                            // From fn not found at emission time — fall through to simple return
                            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                            f.instruction(&Instruction::I32Load(MemArg {
                                offset: 0,
                                align: 2,
                                memory_index: 0,
                            }));
                            f.instruction(&Instruction::Return);
                        }
                    } else {
                        // No From conversion — return original Result ptr
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                        f.instruction(&Instruction::Return);
                    }
                }
                f.instruction(&Instruction::End);
                // tag == 0 (Ok) — extract payload at offset 4
                f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 4,
                    align: 2,
                    memory_index: 0,
                }));
            }
            Operand::FnRef(name) => {
                // Push the function's table index (== function index)
                if let Some(idx) = self
                    .resolve_fn(name)
                    .or_else(|| self.resolve_fn(name.rsplit("::").next().unwrap_or(name)))
                {
                    f.instruction(&Instruction::I32Const(idx as i32));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            Operand::CallIndirect { callee, args } => {
                // Push arguments first
                for arg in args {
                    self.emit_operand(f, arg);
                }
                // Push the function table index (callee)
                self.emit_operand(f, callee);
                // Determine the type signature: (i32 × n_args) -> i32
                let params: Vec<ValType> = args.iter().map(|_| ValType::I32).collect();
                let results = vec![ValType::I32];
                let type_idx = self.lookup_or_register_indirect_type(params, results);
                f.instruction(&Instruction::CallIndirect {
                    type_index: type_idx,
                    table_index: 0,
                });
            }
            Operand::ArrayInit { elements } => {
                // Allocate [element0][element1]...[elementN-1] in linear memory.
                // Each element is 4 bytes (i32). Returns pointer to element 0.
                let n = elements.len() as i32;
                let ma = wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // Save current bump pointer (= array base) to SCRATCH
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Store(ma));
                // Advance bump pointer by n*4
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(n * 4));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                self.emit_heap_grow_check(f);
                // Store each element: mem[base + i*4] = elem
                for (i, elem) in elements.iter().enumerate() {
                    f.instruction(&Instruction::I32Const(SCRATCH as i32));
                    f.instruction(&Instruction::I32Load(ma));
                    f.instruction(&Instruction::I32Const((i as i32) * 4));
                    f.instruction(&Instruction::I32Add);
                    self.emit_operand(f, elem);
                    f.instruction(&Instruction::I32Store(ma));
                }
                // Leave base pointer on stack as the array value
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Load(ma));
            }
            Operand::IndexAccess { object, index } => {
                let ma = wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // array_ptr + index * 4
                self.emit_operand(f, object);
                self.emit_operand(f, index);
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Mul);
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(ma));
            }
            Operand::Unit => { /* nothing to push */ }
            other => {
                eprintln!(
                    "ICE: unhandled operand in emit_operand: {:?}",
                    std::mem::discriminant(other)
                );
                f.instruction(&Instruction::I32Const(0));
            }
        }
    }

    pub(super) fn emit_binop(&mut self, f: &mut Function, op: &BinOp) {
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
        };
    }

    pub(super) fn emit_binop_i64(&mut self, f: &mut Function, op: &BinOp) {
        match op {
            BinOp::Add => {
                f.instruction(&Instruction::I64Add);
            }
            BinOp::Sub => {
                f.instruction(&Instruction::I64Sub);
            }
            BinOp::Mul => {
                f.instruction(&Instruction::I64Mul);
            }
            BinOp::Div => {
                f.instruction(&Instruction::I64DivS);
            }
            BinOp::Mod => {
                f.instruction(&Instruction::I64RemS);
            }
            BinOp::Eq => {
                f.instruction(&Instruction::I64Eq);
            }
            BinOp::Ne => {
                f.instruction(&Instruction::I64Ne);
            }
            BinOp::Lt => {
                f.instruction(&Instruction::I64LtS);
            }
            BinOp::Le => {
                f.instruction(&Instruction::I64LeS);
            }
            BinOp::Gt => {
                f.instruction(&Instruction::I64GtS);
            }
            BinOp::Ge => {
                f.instruction(&Instruction::I64GeS);
            }
            BinOp::And => {
                f.instruction(&Instruction::I64And);
            }
            BinOp::Or => {
                f.instruction(&Instruction::I64Or);
            }
            BinOp::BitAnd => {
                f.instruction(&Instruction::I64And);
            }
            BinOp::BitOr => {
                f.instruction(&Instruction::I64Or);
            }
            BinOp::BitXor => {
                f.instruction(&Instruction::I64Xor);
            }
            BinOp::Shl => {
                f.instruction(&Instruction::I64Shl);
            }
            BinOp::Shr => {
                f.instruction(&Instruction::I64ShrS);
            }
        };
    }

    pub(super) fn emit_binop_f64(&mut self, f: &mut Function, op: &BinOp) {
        match op {
            BinOp::Add => {
                f.instruction(&Instruction::F64Add);
            }
            BinOp::Sub => {
                f.instruction(&Instruction::F64Sub);
            }
            BinOp::Mul => {
                f.instruction(&Instruction::F64Mul);
            }
            BinOp::Div => {
                f.instruction(&Instruction::F64Div);
            }
            BinOp::Eq => {
                f.instruction(&Instruction::F64Eq);
            }
            BinOp::Ne => {
                f.instruction(&Instruction::F64Ne);
            }
            BinOp::Lt => {
                f.instruction(&Instruction::F64Lt);
            }
            BinOp::Le => {
                f.instruction(&Instruction::F64Le);
            }
            BinOp::Gt => {
                f.instruction(&Instruction::F64Gt);
            }
            BinOp::Ge => {
                f.instruction(&Instruction::F64Ge);
            }
            // Mod and bitwise ops don't apply to f64, fall back to i32
            _ => {
                self.emit_binop(f, op);
            }
        };
    }

    pub(super) fn is_f64_operand(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstF64(_) | Operand::ConstF32(_) => true,
            Operand::Place(Place::Local(id)) => self.f64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_f64_operand(l) || self.is_f64_operand(r),
            Operand::UnaryOp(_, inner) => self.is_f64_operand(inner),
            Operand::Call(name, _) => {
                let normalized = normalize_intrinsic_name(name.as_str());
                let lookup_name = name.rsplit("::").next().unwrap_or(name.as_str());
                if matches!(normalized, "sqrt") {
                    return true;
                }
                self.fn_return_types
                    .get(normalized)
                    .or_else(|| self.fn_return_types.get(lookup_name))
                    .is_some_and(|t| matches!(t, ark_typecheck::types::Type::F64))
            }
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => {
                then_result.as_ref().is_some_and(|r| self.is_f64_operand(r))
                    || else_result.as_ref().is_some_and(|r| self.is_f64_operand(r))
            }
            _ => false,
        }
    }

    pub(super) fn is_i64_operand(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstI64(_) | Operand::ConstU64(_) => true,
            Operand::Place(Place::Local(id)) => self.i64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_i64_operand(l) || self.is_i64_operand(r),
            Operand::UnaryOp(_, inner) => self.is_i64_operand(inner),
            Operand::Call(name, _) => {
                let normalized = normalize_intrinsic_name(name.as_str());
                if matches!(normalized, "clock_now") {
                    return true;
                }
                // Check fn_return_types for user-defined functions returning i64
                self.fn_return_types.get(normalized).is_some_and(|t| {
                    matches!(
                        t,
                        ark_typecheck::types::Type::I64 | ark_typecheck::types::Type::U64
                    )
                })
            }
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => {
                then_result.as_ref().is_some_and(|r| self.is_i64_operand(r))
                    || else_result.as_ref().is_some_and(|r| self.is_i64_operand(r))
            }
            _ => false,
        }
    }

    /// Returns the element byte size for a vector operand: 8 for Vec<i64>/Vec<f64>, 4 otherwise.
    pub(super) fn vec_elem_size(&self, vec_op: &Operand) -> u32 {
        if let Operand::Place(Place::Local(id)) = vec_op
            && (self.vec_i64_locals.contains(&id.0) || self.vec_f64_locals.contains(&id.0))
        {
            return 8;
        }
        // Also handle struct field access
        if let Operand::FieldAccess {
            struct_name, field, ..
        } = vec_op
        {
            let key = (struct_name.clone(), field.clone());
            if self.struct_vec_i64_fields.contains(&key)
                || self.struct_vec_f64_fields.contains(&key)
            {
                return 8;
            }
        }
        4
    }

    /// Check if a vector operand holds Vec<i64> elements.
    pub(super) fn is_vec_i64(&self, vec_op: &Operand) -> bool {
        if let Operand::Place(Place::Local(id)) = vec_op {
            return self.vec_i64_locals.contains(&id.0);
        }
        if let Operand::FieldAccess {
            struct_name, field, ..
        } = vec_op
        {
            return self
                .struct_vec_i64_fields
                .contains(&(struct_name.clone(), field.clone()));
        }
        false
    }

    /// Check if a vector operand holds Vec<f64> elements.
    pub(super) fn is_vec_f64(&self, vec_op: &Operand) -> bool {
        if let Operand::Place(Place::Local(id)) = vec_op {
            return self.vec_f64_locals.contains(&id.0);
        }
        if let Operand::FieldAccess {
            struct_name, field, ..
        } = vec_op
        {
            return self
                .struct_vec_f64_fields
                .contains(&(struct_name.clone(), field.clone()));
        }
        false
    }

    pub(super) fn is_bool_operand(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstBool(_) => true,
            Operand::Place(Place::Local(id)) => self.bool_locals.contains(&id.0),
            Operand::BinOp(op, _, _) => {
                matches!(
                    op,
                    BinOp::Eq
                        | BinOp::Ne
                        | BinOp::Lt
                        | BinOp::Le
                        | BinOp::Gt
                        | BinOp::Ge
                        | BinOp::And
                        | BinOp::Or
                )
            }
            Operand::UnaryOp(op, _) => matches!(op, UnaryOp::Not),
            _ => false,
        }
    }

    /// Emit an operand as f64, promoting i32/i64 constants if needed
    pub(super) fn emit_f64_operand(&mut self, f: &mut Function, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::F64Const(*v as f64));
            }
            Operand::ConstI64(v) => {
                f.instruction(&Instruction::F64Const(*v as f64));
            }
            _ => self.emit_operand(f, op),
        }
    }

    /// Emit an operand as i64, promoting i32 constants if needed
    pub(super) fn emit_i64_operand(&mut self, f: &mut Function, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::I64Const(*v as i64));
            }
            Operand::UnaryOp(UnaryOp::Neg, inner) => {
                // Emit negation as i64: 0 - inner
                f.instruction(&Instruction::I64Const(0));
                self.emit_i64_operand(f, inner);
                f.instruction(&Instruction::I64Sub);
            }
            Operand::BinOp(op, left, right) => {
                self.emit_i64_operand(f, left);
                self.emit_i64_operand(f, right);
                self.emit_binop_i64(f, op);
            }
            _ => self.emit_operand(f, op),
        }
    }

    pub(super) fn emit_unaryop(&mut self, f: &mut Function, op: &UnaryOp, inner: &Operand) {
        match op {
            UnaryOp::Neg => {
                let is_f64 = matches!(inner, Operand::ConstF64(_))
                    || matches!(inner, Operand::Place(Place::Local(id)) if self.f64_locals.contains(&id.0));
                if is_f64 {
                    self.emit_operand(f, inner);
                    f.instruction(&Instruction::F64Neg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    self.emit_operand(f, inner);
                    f.instruction(&Instruction::I32Sub);
                }
            }
            UnaryOp::Not => {
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Eqz);
            }
            UnaryOp::BitNot => {
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::I32Xor);
            }
            UnaryOp::SignExtend8 => {
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Extend8S);
            }
            UnaryOp::SignExtend16 => {
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Extend16S);
            }
            UnaryOp::SignExtend32 => {
                self.emit_operand(f, inner);
                // i64.extend32_s — only meaningful for i64 values
                f.instruction(&Instruction::I64Extend32S);
            }
        }
    }
}
