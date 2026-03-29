//! Statement emission: emit_stmt, emit_call, emit_println, emit_print,
//! emit_eprintln, emit_fd_write.

use super::*;

impl EmitCtx {
    pub(super) fn emit_stmt(&mut self, f: &mut Function, stmt: &MirStmt) {
        match stmt {
            MirStmt::Assign(Place::Local(id), Rvalue::Use(op)) => {
                self.emit_operand(f, op);
                f.instruction(&Instruction::LocalSet(id.0));
            }
            MirStmt::Assign(Place::Field(base, field_name), Rvalue::Use(op)) => {
                // Field store: struct_ptr.field = value
                if let Place::Local(base_id) = base.as_ref() {
                    let struct_name = self.local_struct_names.get(&base_id.0).cloned();
                    let (offset, is_f64, is_i64) = if let Some(ref sname) = struct_name {
                        self.struct_field_info(sname, field_name)
                    } else {
                        (0, false, false)
                    };
                    // Push object pointer (base address)
                    f.instruction(&Instruction::LocalGet(base_id.0));
                    if offset > 0 {
                        f.instruction(&Instruction::I32Const(offset as i32));
                        f.instruction(&Instruction::I32Add);
                    }
                    // Push value to store
                    self.emit_operand(f, op);
                    // Store at the field location
                    if is_f64 {
                        f.instruction(&Instruction::F64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    } else if is_i64 {
                        f.instruction(&Instruction::I64Store(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    } else {
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                }
            }
            MirStmt::CallBuiltin {
                dest: Some(Place::Local(id)),
                name,
                args,
            } => {
                let call_op = Operand::Call(name.clone(), args.clone());
                self.emit_operand(f, &call_op);
                f.instruction(&Instruction::LocalSet(id.0));
            }
            MirStmt::CallBuiltin { name, args, .. } => {
                let original_name = name.as_str();
                let lookup_name = original_name.rsplit("::").next().unwrap_or(original_name);
                let name = normalize_intrinsic_name(lookup_name);
                match name {
                    "println" => self.emit_println(f, args),
                    "print" => self.emit_print(f, args),
                    "eprintln" => {
                        self.emit_eprintln(f, args);
                    }
                    "print_i32_ln" => {
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            self.call_fn(f, FN_PRINT_I32_LN);
                        }
                    }
                    "print_bool_ln" => {
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            self.call_fn(f, FN_PRINT_BOOL_LN);
                        }
                    }
                    "print_str_ln" => {
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            self.call_fn(f, FN_PRINT_STR_LN);
                        }
                    }
                    "i32_to_string" => {
                        // As statement, result is discarded
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            self.call_fn(f, FN_I32_TO_STR);
                        }
                    }
                    "push"
                    | "set"
                    | "sort_i32"
                    | "sort_String"
                    | "sort_i64"
                    | "sort_f64"
                    | "reverse_i32"
                    | "reverse_String"
                    | "remove_i32"
                    | "HashMap_i32_i32_insert" => {
                        // Void Vec operations — emit inline via Operand::Call path
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                    }
                    "panic" | "assert" | "assert_eq" | "assert_ne" | "assert_eq_str"
                    | "assert_eq_i64" => {
                        // void builtins — emit inline via Operand::Call path
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                    }
                    "push_char" => {
                        // push_char is mutating — emit call and update the local
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                        // push_char returns new str ptr; update the variable
                        if let Some(Operand::Place(Place::Local(lid))) = args.first() {
                            f.instruction(&Instruction::LocalSet(lid.0));
                        } else {
                            f.instruction(&Instruction::Drop);
                        }
                    }
                    "pop"
                    | "get"
                    | "Vec_new_i32"
                    | "Vec_new_String"
                    | "len"
                    | "get_unchecked"
                    | "fs_read_file"
                    | "fs_write_file"
                    | "any_i32"
                    | "find_i32"
                    | "clock_now"
                    | "random_i32"
                    | "HashMap_i32_i32_new"
                    | "HashMap_i32_i32_get"
                    | "HashMap_i32_i32_contains_key"
                    | "HashMap_i32_i32_len"
                    | "contains_i32"
                    | "contains_String" => {
                        // Value-returning Vec operations called as statement — emit and drop result
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                        f.instruction(&Instruction::Drop);
                    }
                    other if other.starts_with("Vec_new_") => {
                        // Dynamic Vec_new_<Type> — same as Vec_new_i32
                        let call_op = Operand::Call(other.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                        f.instruction(&Instruction::Drop);
                    }
                    other => {
                        // User function call — type-aware argument emission
                        let param_types = self
                            .fn_param_types
                            .get(other)
                            .or_else(|| self.fn_param_types.get(lookup_name))
                            .cloned();
                        for (i, arg) in args.iter().enumerate() {
                            let is_i64_param = param_types
                                .as_ref()
                                .and_then(|pts| pts.get(i))
                                .is_some_and(|t| {
                                    matches!(
                                        t,
                                        ark_typecheck::types::Type::I64
                                            | ark_typecheck::types::Type::U64
                                    )
                                });
                            let is_f64_param = param_types
                                .as_ref()
                                .and_then(|pts| pts.get(i))
                                .is_some_and(|t| matches!(t, ark_typecheck::types::Type::F64));
                            if is_i64_param {
                                self.emit_i64_operand(f, arg);
                            } else if is_f64_param {
                                self.emit_f64_operand(f, arg);
                            } else {
                                self.emit_operand(f, arg);
                            }
                        }
                        if let Some(idx) = self
                            .resolve_fn(other)
                            .or_else(|| self.resolve_fn(lookup_name))
                        {
                            f.instruction(&Instruction::Call(idx));
                            let returns_value = self
                                .fn_return_types
                                .get(other)
                                .or_else(|| self.fn_return_types.get(lookup_name))
                                .is_some_and(|t| !matches!(t, ark_typecheck::types::Type::Unit));
                            if returns_value {
                                f.instruction(&Instruction::Drop);
                            }
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
                // Track block depth for break/continue
                if let Some(d) = self.loop_depths.last_mut() {
                    *d += 1;
                }
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                if !else_body.is_empty() {
                    f.instruction(&Instruction::Else);
                    for s in else_body {
                        self.emit_stmt(f, s);
                    }
                }
                f.instruction(&Instruction::End);
                if let Some(d) = self.loop_depths.last_mut() {
                    *d -= 1;
                }
            }
            MirStmt::WhileStmt { cond, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // break target
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // continue target
                self.emit_operand(f, cond);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1)); // exit if cond is false
                self.loop_depths.push(0);
                for s in body {
                    self.emit_stmt(f, s);
                }
                self.loop_depths.pop();
                f.instruction(&Instruction::Br(0)); // loop back
                f.instruction(&Instruction::End); // end loop
                f.instruction(&Instruction::End); // end block
            }
            MirStmt::Break => {
                // break: jump to the outer block of the enclosing while
                // br(extra_depth + 1): +1 because loop label is between us and block
                let depth = self.loop_depths.last().copied().unwrap_or(0) + 1;
                f.instruction(&Instruction::Br(depth));
            }
            MirStmt::Continue => {
                // continue: jump to the loop label
                let depth = self.loop_depths.last().copied().unwrap_or(0);
                f.instruction(&Instruction::Br(depth));
            }
            MirStmt::Return(op) => {
                if let Some(val) = op {
                    self.emit_operand(f, val);
                }
                f.instruction(&Instruction::Return);
            }
            other => {
                eprintln!(
                    "ICE: unhandled statement in emit_stmt: {:?}",
                    std::mem::discriminant(other)
                );
            }
        }
    }

    pub(super) fn emit_println(&mut self, f: &mut Function, args: &[Operand]) {
        if let Some(arg) = args.first() {
            match arg {
                Operand::ConstString(s) => {
                    let msg = format!("{}\n", s);
                    let (offset, len) = self.alloc_string(&msg);
                    self.emit_fd_write(f, 1, offset, len);
                }
                Operand::Call(name, inner_args) => {
                    match normalize_intrinsic_name(name.as_str()) {
                        "i32_to_string" => {
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                self.call_fn(f, FN_PRINT_I32_LN);
                            }
                        }
                        "bool_to_string" => {
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                self.call_fn(f, FN_PRINT_BOOL_LN);
                            }
                        }
                        "f64_to_string" => {
                            // Convert f64 to string, then print as string
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                self.call_fn(f, FN_F64_TO_STR);
                                self.call_fn(f, FN_PRINT_STR_LN);
                            }
                        }
                        "i64_to_string" => {
                            // Convert i64 to string, then print as string
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                self.call_fn(f, FN_I64_TO_STR);
                                self.call_fn(f, FN_PRINT_STR_LN);
                            }
                        }
                        "concat" => {
                            // Concatenate strings, then print result
                            for a in inner_args {
                                self.emit_operand(f, a);
                            }
                            self.call_fn(f, FN_CONCAT);
                            self.call_fn(f, FN_PRINT_STR_LN);
                        }
                        "char_to_string" => {
                            if let Some(inner) = inner_args.first() {
                                let converted = Operand::Call(
                                    "char_to_string".to_string(),
                                    vec![inner.clone()],
                                );
                                self.emit_operand(f, &converted);
                                self.call_fn(f, FN_PRINT_STR_LN);
                            }
                        }
                        "String_from" => {
                            // String_from("literal") — just print the literal
                            if let Some(Operand::ConstString(s)) = inner_args.first() {
                                let msg = format!("{}\n", s);
                                let (offset, len) = self.alloc_string(&msg);
                                self.emit_fd_write(f, 1, offset, len);
                            } else if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                self.call_fn(f, FN_PRINT_STR_LN);
                            }
                        }
                        other => {
                            // Check if this is a builtin inline function (not a real fn index)
                            if self.resolve_fn(other).is_some() {
                                // Real user function call — emit args, call, then print result
                                for a in inner_args {
                                    self.emit_operand(f, a);
                                }
                                let idx = self.resolve_fn(other).unwrap();
                                f.instruction(&Instruction::Call(idx));
                                let is_str = self.fn_return_types.get(other).is_some_and(|t| {
                                    matches!(t, ark_typecheck::types::Type::String)
                                });
                                let args_suggest_str =
                                    inner_args.iter().any(|a| self.is_string_operand(a));
                                if is_str || args_suggest_str {
                                    self.call_fn(f, FN_PRINT_STR_LN);
                                } else {
                                    self.call_fn(f, FN_PRINT_I32_LN);
                                }
                            } else {
                                // Inline builtin (len, get, etc.) — emit full operand
                                self.emit_operand(f, arg);
                                if self.is_string_operand(arg) {
                                    self.call_fn(f, FN_PRINT_STR_LN);
                                } else {
                                    self.call_fn(f, FN_PRINT_I32_LN);
                                }
                            }
                        }
                    }
                }
                Operand::Place(Place::Local(id)) => {
                    f.instruction(&Instruction::LocalGet(id.0));
                    if self.string_locals.contains(&id.0) {
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else if self.i64_locals.contains(&id.0) {
                        self.call_fn(f, FN_I64_TO_STR);
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else if self.f64_locals.contains(&id.0) {
                        self.call_fn(f, FN_F64_TO_STR);
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else if self.bool_locals.contains(&id.0) {
                        self.call_fn(f, FN_PRINT_BOOL_LN);
                    } else if self.char_locals.contains(&id.0) {
                        let converted = Operand::Call(
                            "char_to_string".to_string(),
                            vec![Operand::Place(Place::Local(*id))],
                        );
                        f.instruction(&Instruction::Drop);
                        self.emit_operand(f, &converted);
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else {
                        self.call_fn(f, FN_PRINT_I32_LN);
                    }
                }
                _ => {
                    // Generic: emit operand and dispatch based on type
                    self.emit_operand(f, arg);
                    if self.is_string_operand(arg) {
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else if self.is_i64_operand(arg) {
                        self.call_fn(f, FN_I64_TO_STR);
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else if self.is_f64_operand(arg) {
                        self.call_fn(f, FN_F64_TO_STR);
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else if self.is_bool_operand(arg) {
                        self.call_fn(f, FN_PRINT_BOOL_LN);
                    } else if matches!(arg, Operand::ConstChar(_))
                        || matches!(arg, Operand::Place(Place::Local(id)) if self.char_locals.contains(&id.0))
                    {
                        let converted =
                            Operand::Call("char_to_string".to_string(), vec![arg.clone()]);
                        f.instruction(&Instruction::Drop);
                        self.emit_operand(f, &converted);
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else {
                        self.call_fn(f, FN_PRINT_I32_LN);
                    }
                }
            }
        }
    }

    pub(super) fn emit_eprintln(&mut self, f: &mut Function, args: &[Operand]) {
        // Write to stderr (fd=2)
        if let Some(arg) = args.first() {
            match arg {
                Operand::ConstString(s) => {
                    let msg = format!("{}\n", s);
                    let (offset, len) = self.alloc_string(&msg);
                    self.emit_fd_write(f, 2, offset, len);
                }
                _ => {
                    // For now, print as string literal to stderr
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::Drop);
                }
            }
        }
    }

    pub(super) fn emit_print(&mut self, f: &mut Function, args: &[Operand]) {
        if let Some(arg) = args.first() {
            match arg {
                Operand::ConstString(s) => {
                    let (offset, len) = self.alloc_string(s);
                    self.emit_fd_write(f, 1, offset, len);
                }
                Operand::Place(Place::Local(id)) if self.string_locals.contains(&id.0) => {
                    // String local: print via fd_write without newline
                    let ma2 = MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    };
                    // iov.base = ptr
                    f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                    f.instruction(&Instruction::LocalGet(id.0));
                    f.instruction(&Instruction::I32Store(ma2));
                    // iov.len = i32.load(ptr - 4)
                    f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                    f.instruction(&Instruction::LocalGet(id.0));
                    f.instruction(&Instruction::I32Const(4));
                    f.instruction(&Instruction::I32Sub);
                    f.instruction(&Instruction::I32Load(ma2));
                    f.instruction(&Instruction::I32Store(ma2));
                    // fd_write(1, iov, 1, nwritten)
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                    self.call_fn(f, FN_FD_WRITE);
                    f.instruction(&Instruction::Drop);
                }
                _ => {
                    self.emit_operand(f, arg);
                    if self.is_string_operand(arg) {
                        // String on stack: use print_str_ln (adds newline; best effort)
                        self.call_fn(f, FN_PRINT_STR_LN);
                    } else {
                        // Print i32 without newline
                        self.call_fn(f, FN_I32_TO_STR);
                        let ma2 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma2));
                        f.instruction(&Instruction::I32Store(ma2));
                        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                        f.instruction(&Instruction::I32Load(ma2));
                        f.instruction(&Instruction::I32Store(ma2));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        self.call_fn(f, FN_FD_WRITE);
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
        }
    }

    pub(super) fn emit_fd_write(
        &mut self,
        f: &mut Function,
        fd: u32,
        str_offset: u32,
        str_len: u32,
    ) {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(str_offset as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::I32Const(str_len as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const(fd as i32));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        self.call_fn(f, FN_FD_WRITE);
        f.instruction(&Instruction::Drop);
    }
}
