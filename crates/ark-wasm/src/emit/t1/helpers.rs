//! Helper function emission: i32_to_string, print_*, str_eq, concat,
//! f64_to_string, i64_to_string.

use super::*;
use wasm_encoder::MemArg;

impl EmitCtx {
    /// __i32_to_string(value: i32) -> void
    /// Writes decimal string to I32BUF, stores offset at SCRATCH, length at SCRATCH+4
    pub(super) fn build_i32_to_string(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let buf_end = I32BUF + 20; // write digits right-to-left
        // Locals: 0=value(param), 1=is_neg, 2=pos, 3=abs_val
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // pos = buf_end
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalSet(2));

        // Zero check
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(IOV_BASE as i32)); // not needed but let's go
            // Store '0' at buf_end-1
            f.instruction(&Instruction::I32Const((buf_end - 1) as i32));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Store8(ma));
            // SCRATCH = buf_end - 1
            f.instruction(&Instruction::I32Const(SCRATCH as i32));
            f.instruction(&Instruction::I32Const((buf_end - 1) as i32));
            f.instruction(&Instruction::I32Store(ma2));
            // SCRATCH+4 = 1
            f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Store(ma2));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // Check negative
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::LocalSet(1));

        // abs_val
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        // Digit extraction loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        // pos -= 1
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(2));
        // mem[pos] = (abs_val % 10) + '0'
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma));
        // abs_val /= 10
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Prepend '-' if negative
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma));
        f.instruction(&Instruction::End);

        // Store result: SCRATCH = pos (offset), SCRATCH+4 = buf_end - pos (len)
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Store(ma2));

        f.instruction(&Instruction::End);
        f
    }

    /// __print_i32_ln(value: i32) -> void
    /// Prints value as decimal string + newline to stdout
    pub(super) fn build_print_i32_ln(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![]);

        // Call __i32_to_string(value)
        f.instruction(&Instruction::LocalGet(0));
        self.call_fn(&mut f, FN_I32_TO_STR);

        // Set up iov for the number string
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::I32Store(ma2));

        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::I32Store(ma2));

        // fd_write(1, iov, 1, nwritten)
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        self.call_fn(&mut f, FN_FD_WRITE);
        f.instruction(&Instruction::Drop);

        // Print newline
        self.emit_static_print(&mut f, NEWLINE, 1);

        f.instruction(&Instruction::End);
        f
    }

    /// __print_bool_ln(value: i32) -> void
    /// Prints "true\n" or "false\n" to stdout
    pub(super) fn build_print_bool_ln(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // Locals: 0=value(param), 1=ptr, 2=len
        let mut f = Function::new(vec![(2, ValType::I32)]);

        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // true
            f.instruction(&Instruction::I32Const(BOOL_TRUE as i32));
            f.instruction(&Instruction::LocalSet(1));
            f.instruction(&Instruction::I32Const(4));
            f.instruction(&Instruction::LocalSet(2));
        }
        f.instruction(&Instruction::Else);
        {
            // false
            f.instruction(&Instruction::I32Const(BOOL_FALSE as i32));
            f.instruction(&Instruction::LocalSet(1));
            f.instruction(&Instruction::I32Const(5));
            f.instruction(&Instruction::LocalSet(2));
        }
        f.instruction(&Instruction::End);

        // Setup iov and write
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        self.call_fn(&mut f, FN_FD_WRITE);
        f.instruction(&Instruction::Drop);

        // Print newline
        self.emit_static_print(&mut f, NEWLINE, 1);

        f.instruction(&Instruction::End);
        f
    }

    /// __print_str_ln(ptr: i32) -> void
    /// ptr points to string data; length is at (ptr - 4) as i32.
    /// Prints the string + newline to stdout.
    pub(super) fn build_print_str_ln(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![]);

        // iov.base = ptr
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Store(ma2));

        // iov.len = i32.load(ptr - 4)
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::I32Store(ma2));

        // fd_write(1, iov, 1, nwritten)
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        self.call_fn(&mut f, FN_FD_WRITE);
        f.instruction(&Instruction::Drop);

        // Print newline
        self.emit_static_print(&mut f, NEWLINE, 1);

        f.instruction(&Instruction::End);
        f
    }

    /// Emit fd_write for a static data segment at known offset/length.
    pub(super) fn emit_static_print(&self, f: &mut Function, offset: u32, len: u32) {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(offset as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::I32Const(len as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        self.call_fn(f, FN_FD_WRITE);
        f.instruction(&Instruction::Drop);
    }

    /// __str_eq(ptr1: i32, ptr2: i32) -> i32 (0 or 1)
    /// Compares two length-prefixed strings for equality.
    /// Length is at ptr-4, data at ptr.
    pub(super) fn build_str_eq(&self) -> Function {
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
        // params: 0=ptr1, 1=ptr2; locals: 2=len1, 3=len2, 4=i
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // len1 = mem[ptr1 - 4]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(2));

        // len2 = mem[ptr2 - 4]
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(3));

        // if len1 != len2, return 0
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4));

        // loop: compare bytes
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len1, break (all equal)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if mem[ptr1+i] != mem[ptr2+i], return 0
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));

        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));

        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // i += 1
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));

        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // All bytes match
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::End);
        f
    }

    /// __concat(ptr1: i32, ptr2: i32) -> i32 (pointer to new length-prefixed string)
    /// Allocates new string = len1 + len2, copies data from both, returns data pointer.
    pub(super) fn build_concat(&self) -> Function {
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
        // params: 0=ptr1, 1=ptr2; locals: 2=len1, 3=len2, 4=total_len, 5=dst, 6=i
        let mut f = Function::new(vec![(5, ValType::I32)]);

        // len1 = mem[ptr1 - 4]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(2));

        // len2 = mem[ptr2 - 4]
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(3));

        // total_len = len1 + len2
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));

        // Write total_len at heap_ptr (length prefix)
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Store(ma2));

        // dst = heap_ptr + 4 (data start)
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(5));

        // Copy bytes from ptr1 (len1 bytes)
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= len1, break
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[dst + i] = mem[ptr1 + i]
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        // i += 1
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Copy bytes from ptr2 (len2 bytes)
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= len2, break
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[dst + len1 + i] = mem[ptr2 + i]
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        // i += 1
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Bump heap pointer: heap_ptr += 4 + total_len
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Return dst (data pointer)
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __f64_to_str(value: f64) -> i32 (string ptr)
    /// Converts f64 to a length-prefixed string on the heap.
    /// Uses I32BUF (scratch) area for building the string.
    /// Locals: 0=value(f64 param), 1=is_neg(i32), 2=int_part(i32), 3=frac_digits(i32),
    ///         4=pos(i32), 5=digit(i32), 6=len(i32), 7=dst(i32)
    pub(super) fn build_f64_to_string(&self) -> Function {
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
        let buf = I32BUF;
        // Locals: 7 i32 locals
        let mut f = Function::new(vec![(7, ValType::I32)]);

        // pos = buf (write position)
        f.instruction(&Instruction::I32Const(buf as i32));
        f.instruction(&Instruction::LocalSet(4));

        // is_neg = value < 0.0
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Const(0.0));
        f.instruction(&Instruction::F64Lt);
        f.instruction(&Instruction::LocalSet(1));

        // if negative, negate value
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Neg);
        f.instruction(&Instruction::LocalSet(0));
        // write '-' at pos
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(b'-' as i32));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::End);

        // int_part = trunc(value) as i32
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Floor);
        f.instruction(&Instruction::I32TruncF64S);
        f.instruction(&Instruction::LocalSet(2));

        // Convert integer part to digits (right to left into temp, then copy)
        // Handle 0 case
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Write '0' at pos
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::Else);
        {
            // Write integer digits: use SCRATCH area temporarily
            // digit_pos = SCRATCH+20 (write from right)
            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 20));
            f.instruction(&Instruction::LocalSet(5)); // reuse local 5 as digit_pos
            // digit_count = 0 in local 6
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(6));
            // loop: extract digits
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            // if int_part == 0, break
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Eqz);
            f.instruction(&Instruction::BrIf(1));
            // digit = int_part % 10
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalTee(5));
            // store digit char
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32RemU);
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // int_part /= 10
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32DivU);
            f.instruction(&Instruction::LocalSet(2));
            // digit_count += 1
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(6));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End); // end loop
            f.instruction(&Instruction::End); // end block
            // Copy digits from SCRATCH area to output buffer at pos
            // digit_pos points to first digit, digit_count has count
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(2)); // reuse as counter
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));
            // buf[pos + i] = scratch[digit_pos + i]
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Load8U(ma0));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);
            // pos += digit_count
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // Fractional part: frac = value - floor(value)
        // frac_digits = round(frac * 100000000) as i32  (8 decimal digits)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Floor);
        f.instruction(&Instruction::F64Sub);
        f.instruction(&Instruction::F64Const(100000000.0)); // 10^8
        f.instruction(&Instruction::F64Mul);
        f.instruction(&Instruction::F64Nearest);
        f.instruction(&Instruction::I32TruncF64S);
        f.instruction(&Instruction::LocalSet(3)); // frac_digits

        // If frac_digits > 0, add decimal point and digits
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32GtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Write '.'
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'.' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));

            // Write fractional digits (up to 8, removing trailing zeros)
            // Find how many trailing zeros to remove
            // Start with 8 digits: pad with leading zeros, remove trailing zeros
            // digit_count = 8
            f.instruction(&Instruction::I32Const(8));
            f.instruction(&Instruction::LocalSet(6));

            // Remove trailing zeros from frac_digits
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            // if frac_digits % 10 != 0, break
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32RemU);
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Ne);
            f.instruction(&Instruction::BrIf(1));
            // frac_digits /= 10, digit_count -= 1
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32DivU);
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(6));
            // if digit_count == 0, break
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Eqz);
            f.instruction(&Instruction::BrIf(1));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);

            // Now write digit_count digits from frac_digits (right to left)
            // Write digits at pos + digit_count - 1 .. pos
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::LocalSet(5)); // remaining = digit_count
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Eqz);
            f.instruction(&Instruction::BrIf(1));
            // buf[pos + remaining - 1] = '0' + frac_digits % 10
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32RemU);
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // frac_digits /= 10
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32DivU);
            f.instruction(&Instruction::LocalSet(3));
            // remaining -= 1
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(5));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);

            // pos += digit_count
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::Else);
        {
            // If frac_digits == 0, write ".0"
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'.' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalTee(4));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // Now build length-prefixed string on heap
        // len = pos - buf
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(buf as i32));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(6)); // len

        // Write length at heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma2));

        // Copy data from buf to heap_ptr+4
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(5)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // heap[heap_ptr + 4 + i] = buf[buf + i]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(buf as i32));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // dst = heap_ptr + 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(7));

        // Bump heap: heap_ptr += 4 + len
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Return dst
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::End);
        f
    }

    /// __i64_to_str(value: i64) -> i32 (string ptr)
    /// Similar to i32_to_string but handles 64-bit integers.
    /// Locals: 0=value(i64 param), 1=is_neg(i32), 2=pos(i32), 3=digit_count(i32),
    ///         4=digit(i32), 5=dst(i32), 6=len(i32)
    pub(super) fn build_i64_to_string(&self) -> Function {
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
        let buf = I32BUF;
        let buf_end = buf + 22; // max 20 digits for i64 + sign
        let mut f = Function::new(vec![(6, ValType::I32)]);

        // is_neg = value < 0
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::I64LtS);
        f.instruction(&Instruction::LocalSet(1));

        // if negative, negate value
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Sub);
        f.instruction(&Instruction::LocalSet(0));
        f.instruction(&Instruction::End);

        // pos = buf_end (write digits right-to-left)
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalSet(2));
        // digit_count = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3));

        // Handle 0 case
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalTee(2));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3));
        }
        f.instruction(&Instruction::Else);
        {
            // Loop: extract digits
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::I64Eqz);
            f.instruction(&Instruction::BrIf(1));
            // digit = (value % 10) as i32
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64RemU);
            f.instruction(&Instruction::I32WrapI64);
            f.instruction(&Instruction::LocalSet(4));
            // pos -= 1; store digit char
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalTee(2));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // value /= 10
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64DivU);
            f.instruction(&Instruction::LocalSet(0));
            // digit_count += 1
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End); // end loop
            f.instruction(&Instruction::End); // end block
        }
        f.instruction(&Instruction::End);

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalTee(2));
        f.instruction(&Instruction::I32Const(b'-' as i32));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        // len = digit_count (including sign)
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalSet(6));

        // Allocate length-prefixed string on heap: [len:i32][data:bytes]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma2));

        // Copy digits from buf area to heap+4
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // heap[heap_ptr + 4 + i] = buf[pos + i]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // dst = heap_ptr + 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(5));

        // Bump heap
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Return dst
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// Buffered stdin reader: returns next byte (0–255) or -1 on EOF.
    /// Reads 65536 bytes at a time from fd=0 into linear memory at STDIN_BUF.
    /// Uses STDIN_BUF_POS (mem[168]) and STDIN_BUF_LEN (mem[172]) as state.
    pub(super) fn build_get_byte(&self) -> Function {
        // Signature: () -> i32
        // local 0: result byte (default -1 = EOF)
        let mut f = Function::new(vec![(1, ValType::I32)]);

        let ma  = MemArg { offset: 0, align: 2, memory_index: 0 };
        let ma1 = MemArg { offset: 0, align: 0, memory_index: 0 };

        // default: result = -1 (EOF)
        f.instruction(&Instruction::I32Const(-1i32));
        f.instruction(&Instruction::LocalSet(0));

        // if STDIN_BUF_POS >= STDIN_BUF_LEN -> refill
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // $filled
        f.instruction(&Instruction::I32Const(STDIN_BUF_POS as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(STDIN_BUF_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32LtU);
        f.instruction(&Instruction::BrIf(0)); // buffer has data, skip refill

        // --- refill: call fd_read(fd=0, iov, 1, &nread) ---
        // IOV at address 0: iov_base=STDIN_BUF, iov_len=STDIN_BUF_SIZE
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(STDIN_BUF as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::I32Const(STDIN_BUF_SIZE as i32));
        f.instruction(&Instruction::I32Store(ma));

        // fd_read(0, IOV_BASE, 1, FS_NREAD)
        f.instruction(&Instruction::I32Const(0)); // fd=0 (stdin)
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
        self.call_fn(&mut f, FN_FD_READ);
        f.instruction(&Instruction::Drop);

        // STDIN_BUF_POS = 0
        f.instruction(&Instruction::I32Const(STDIN_BUF_POS as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // STDIN_BUF_LEN = mem[FS_NREAD]
        f.instruction(&Instruction::I32Const(STDIN_BUF_LEN as i32));
        f.instruction(&Instruction::I32Const(FS_NREAD as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::End); // end $filled

        // if STDIN_BUF_LEN > 0: read byte, increment POS
        f.instruction(&Instruction::I32Const(STDIN_BUF_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty)); // if EOF: skip
        f.instruction(&Instruction::Else);
        // byte = STDIN_BUF[STDIN_BUF_POS]
        f.instruction(&Instruction::I32Const(STDIN_BUF as i32));
        f.instruction(&Instruction::I32Const(STDIN_BUF_POS as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma1));
        f.instruction(&Instruction::LocalSet(0)); // result = byte
        // STDIN_BUF_POS += 1
        f.instruction(&Instruction::I32Const(STDIN_BUF_POS as i32));
        f.instruction(&Instruction::I32Const(STDIN_BUF_POS as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End); // end if

        // return result
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::End);
        f
    }


}