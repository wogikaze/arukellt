//! Helper function emission for the T3 Wasm GC emitter.
//!
//! Emits built-in helper functions (print, i32_to_str, parse_i32, etc.)
//! and the main user function emission logic.

use ark_mir::mir::*;
use std::collections::{HashMap, HashSet};
use ark_typecheck::types::Type;
use wasm_encoder::{
    CodeSection, Function, HeapType, Instruction, MemArg, RefType as WasmRefType, ValType,
};

use super::{nominalize_generic_type_name, normalize_intrinsic, ref_nullable, Ctx};
use super::{
    SCRATCH, I32BUF, P2_RETPTR,
};
use super::peephole::PeepholeWriter;

impl Ctx {
    // ── Helper function bodies ───────────────────────────────────

    pub(super) fn emit_wasi_p2_get_stdout_shim(&self, codes: &mut CodeSection) {
        let mut f = Function::new([]);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_wasi_p2_write_and_flush_shim(&self, codes: &mut CodeSection) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // Params: 0=stream handle, 1=ptr, 2=len, 3=retptr scratch.
        let mut f = Function::new([]);

        // Reuse the caller-provided retptr area as a single preview1 iovec plus nwritten slot.
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::Call(self.wasi_fd_write));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_wasi_p2_drop_output_stream_shim(&self, codes: &mut CodeSection) {
        let mut f = Function::new([]);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_str_helper(&self, codes: &mut CodeSection) {
        let ma1 = MemArg { offset: 0, align: 0, memory_index: 0 };
        // Param 0 = (ref null $string). Locals 1=len, 2=i, 3=handle.
        let mut f = Function::new([(3, ValType::I32)]);

        f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma1));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_i32_helper(&self, codes: &mut CodeSection) {
        let ma0 = MemArg { offset: 0, align: 0, memory_index: 0 };
        // param 0 = i32 value. Locals: 1=is_neg, 2=digit_count, 3=abs_val, 4=temp, 5=handle.
        let mut f = Function::new([
            (1, ValType::I32),
            (1, ValType::I32),
            (1, ValType::I32),
            (1, ValType::I32),
            (1, ValType::I32),
        ]);
        let buf_base = I32BUF;
        let buf_end = buf_base + 11;

        f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        f.instruction(&Instruction::LocalSet(5));

        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(buf_base as i32));
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(buf_base as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(45));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_bool_helper(
        &self,
        codes: &mut CodeSection,
        true_off: u32,
        false_off: u32,
    ) {
        // param 0 = i32 (0=false, 1=true). Local 1 = stdout handle.
        let mut f = Function::new([(1, ValType::I32)]);

        f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        f.instruction(&Instruction::LocalSet(1));

        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(true_off as i32));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(false_off as i32));
        f.instruction(&Instruction::I32Const(5));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_str_ln_helper(&self, codes: &mut CodeSection, newline_off: u32) {
        let ma1 = MemArg { offset: 0, align: 0, memory_index: 0 };
        // Param 0 = (ref null $string). Locals 1=len, 2=i, 3=handle.
        let mut f = Function::new([(3, ValType::I32)]);

        f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma1));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_i32_ln_helper(&self, codes: &mut CodeSection, newline_off: u32) {
        let ma0 = MemArg { offset: 0, align: 0, memory_index: 0 };
        // param 0 = i32 value. Locals: 1=is_neg, 2=digit_count, 3=abs_val, 4=temp, 5=handle.
        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::I32), // local 2: digit_count
            (1, ValType::I32), // local 3: abs_val
            (1, ValType::I32), // local 4: temp
            (1, ValType::I32), // local 5: stdout handle
        ]);
        let buf_base = I32BUF;
        let buf_end = buf_base + 11;

        // Get stdout handle → local 5
        f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        f.instruction(&Instruction::LocalSet(5));

        // Handle negative
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        // Handle zero: write "0\n", drop handle, return
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(buf_base as i32));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(buf_base as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // Extract digits right-to-left into buf
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        // write digits: ptr = buf_end - digit_count + 1, len = digit_count
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_bool_ln_helper(
        &self,
        codes: &mut CodeSection,
        true_off: u32,
        false_off: u32,
        newline_off: u32,
    ) {
        // param 0 = i32 (0=false, 1=true). Local 1 = stdout handle.
        let mut f = Function::new([(1, ValType::I32)]);

        // Get stdout handle → local 1
        f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        f.instruction(&Instruction::LocalSet(1));

        // Write "true" or "false"
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(true_off as i32));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(false_off as i32));
        f.instruction(&Instruction::I32Const(5));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_i32_to_str_helper(&mut self, codes: &mut CodeSection) {
        // GC-native: converts i32 → (ref $string) as a GC byte array.
        // Algorithm: extract digits to scratch memory, then build GC array.
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let str_ref = ref_nullable(self.string_ty);
        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::I32), // local 2: abs_val
            (1, ValType::I32), // local 3: digit_count (total string length)
            (1, ValType::I32), // local 4: temp/loop counter
            (1, str_ref),      // local 5: result GC string
        ]);

        // Determine sign and absolute value
        f.instruction(&Instruction::LocalGet(0)); // value
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1)); // is_neg = 1
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(2)); // abs_val = -value
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(2)); // abs_val = value
        f.instruction(&Instruction::End);

        // Handle zero specially
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(str_ref)));
        // Create "0" as a 1-byte GC array: array.new $string fill=48('0') len=1
        f.instruction(&Instruction::I32Const(48)); // fill value = '0'
        f.instruction(&Instruction::I32Const(1)); // length = 1
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::Else);
        // dummy ref for type consistency (never used)
        f.instruction(&Instruction::I32Const(0)); // fill value
        f.instruction(&Instruction::I32Const(0)); // length
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::Drop); // drop the else-branch dummy

        // Extract digits to scratch memory in reverse order
        // scratch[0..11] holds digits (max 11 for "-2147483648")
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3)); // digit_count = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // break if abs_val == 0
        // scratch[11 - digit_count] = (abs_val % 10) + '0'
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 11));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub); // addr = SCRATCH+11-digit_count
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add); // digit char
        f.instruction(&Instruction::I32Store8(ma0));
        // abs_val /= 10
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(2));
        // digit_count++
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0)); // continue
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3)); // digit_count++ for '-'
        // scratch[12 - digit_count] = '-'
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 12));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        // Create GC string from scratch bytes
        // The string starts at scratch[12 - digit_count] and has digit_count bytes
        f.instruction(&Instruction::I32Const(0)); // fill value (will be overwritten)
        f.instruction(&Instruction::LocalGet(3)); // length
        f.instruction(&Instruction::ArrayNew(self.string_ty)); // array.new $string fill=0 len=digit_count
        f.instruction(&Instruction::LocalSet(5)); // store GC string

        // Copy bytes from scratch to GC array
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // break if i >= digit_count
        // array.set $string result[i] = mem[SCRATCH + 12 - digit_count + i]
        f.instruction(&Instruction::LocalGet(5)); // ref $string
        f.instruction(&Instruction::LocalGet(4)); // i
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 12));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add); // addr = SCRATCH+12-digit_count+i
        f.instruction(&Instruction::I32Load8U(ma0)); // byte from scratch
        f.instruction(&Instruction::ArraySet(self.string_ty));
        // i++
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0)); // continue
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Return the GC string
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_newline_helper(&self, codes: &mut CodeSection, newline_off: u32) {
        // No params. Local 0 = stdout handle.
        let mut f = Function::new([(1, ValType::I32)]);
        f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        f.instruction(&Instruction::LocalSet(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
        f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_i64_to_str_helper(&mut self, codes: &mut CodeSection) {
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let str_ref = ref_nullable(self.string_ty);
        // Use scratch area at SCRATCH offset. Max i64 string = 20 chars ("-9223372036854775808")
        // We'll use scratch[SCRATCH..SCRATCH+24] as digit buffer
        let buf_end = SCRATCH as i32 + 23; // rightmost digit position

        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::I64), // local 2: abs_val (i64)
            (1, ValType::I32), // local 3: digit_count
            (1, ValType::I32), // local 4: loop counter i
            (1, str_ref),      // local 5: result GC string
        ]);

        // Determine sign and absolute value
        f.instruction(&Instruction::LocalGet(0)); // i64 value
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::I64LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1)); // is_neg = 1
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Sub);
        f.instruction(&Instruction::LocalSet(2)); // abs_val = -value
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(2)); // abs_val = value
        f.instruction(&Instruction::End);

        // Handle zero specially
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(str_ref)));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::Drop);

        // Extract digits to scratch memory in reverse order
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3)); // digit_count = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::BrIf(1)); // break if abs_val == 0
        // scratch[buf_end - digit_count] = (abs_val % 10) + '0'
        f.instruction(&Instruction::I32Const(buf_end));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub); // addr
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Const(10));
        f.instruction(&Instruction::I64RemU);
        f.instruction(&Instruction::I32WrapI64); // digit as i32
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add); // digit char
        f.instruction(&Instruction::I32Store8(ma0));
        // abs_val /= 10
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Const(10));
        f.instruction(&Instruction::I64DivU);
        f.instruction(&Instruction::LocalSet(2));
        // digit_count++
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::I32Const(buf_end + 1));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        // Create GC string and copy bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(5));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(5)); // ref $string
        f.instruction(&Instruction::LocalGet(4)); // i
        f.instruction(&Instruction::I32Const(buf_end + 1));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add); // addr = buf_end+1-digit_count+i
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::ArraySet(self.string_ty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __f64_to_str(val: f64) -> ref $string
    /// Converts an f64 to decimal string with up to 15 significant digits.
    /// Uses integer + fractional parts separately.
    pub(super) fn emit_f64_to_str_helper(&mut self, codes: &mut CodeSection) {
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let str_ref = ref_nullable(self.string_ty);
        // Strategy: write characters to scratch memory, then copy to GC array.
        // Max f64 printed length: sign + 20 int digits + '.' + 15 frac digits ≈ 40
        // Use scratch area at offset 64..128
        let buf_base = 64_i32;
        let _buf_size = 64_i32;

        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::F64), // local 2: abs_val
            (1, ValType::I32), // local 3: write_pos (cursor into scratch)
            (1, ValType::I64), // local 4: int_part (as i64)
            (1, ValType::F64), // local 5: frac_part
            (1, ValType::I32), // local 6: temp / digit_count for int part
            (1, ValType::I32), // local 7: int_start position in scratch
            (1, ValType::I32), // local 8: loop counter
            (1, str_ref),      // local 9: result GC string
            (1, ValType::I32), // local 10: total_len
            (1, ValType::I64), // local 11: temp i64 for int digit extraction
            (1, ValType::I32), // local 12: frac_digits count
        ]);

        // Determine sign
        f.instruction(&Instruction::LocalGet(0)); // f64 value
        f.instruction(&Instruction::F64Const(0.0));
        f.instruction(&Instruction::F64Lt);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1)); // is_neg = 1
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Neg);
        f.instruction(&Instruction::LocalSet(2)); // abs_val = -value
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(2)); // abs_val = value
        f.instruction(&Instruction::End);

        // int_part = trunc(abs_val) as i64
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64TruncSatF64U);
        f.instruction(&Instruction::LocalSet(4)); // int_part

        // frac_part = abs_val - f64(int_part)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::F64ConvertI64U);
        f.instruction(&Instruction::F64Sub);
        f.instruction(&Instruction::LocalSet(5)); // frac_part

        // Write position starts after potential '-' sign
        f.instruction(&Instruction::I32Const(buf_base));
        f.instruction(&Instruction::LocalSet(3)); // write_pos = buf_base

        // If negative, write '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        // Convert integer part to digits.
        // Handle zero case
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Write '0'
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Else);
        // Non-zero: extract digits in reverse to a temp area, then copy forward
        // Use scratch at buf_base+40..buf_base+60 as temp digit buffer
        {
            let temp_end = buf_base + 59; // rightmost temp digit position
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(6)); // digit_count = 0
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalSet(11)); // temp_i64 = int_part
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(11));
            f.instruction(&Instruction::I64Eqz);
            f.instruction(&Instruction::BrIf(1));
            // temp[temp_end - digit_count] = (temp_i64 % 10) + '0'
            f.instruction(&Instruction::I32Const(temp_end));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalGet(11));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64RemU);
            f.instruction(&Instruction::I32WrapI64);
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // temp_i64 /= 10
            f.instruction(&Instruction::LocalGet(11));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64DivU);
            f.instruction(&Instruction::LocalSet(11));
            // digit_count++
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(6));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End); // end loop
            f.instruction(&Instruction::End); // end block

            // Copy digits from temp to write_pos
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(8)); // i = 0
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));
            // scratch[write_pos + i] = scratch[temp_end+1-digit_count+i]
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::I32Add); // dest addr
            f.instruction(&Instruction::I32Const(temp_end + 1));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::I32Add); // src addr
            f.instruction(&Instruction::I32Load8U(ma0));
            f.instruction(&Instruction::I32Store8(ma0));
            // i++
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(8));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);

            // write_pos += digit_count
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(3));
        }
        f.instruction(&Instruction::End); // end if/else for zero check

        // Write '.'
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(46)); // '.'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));

        // Write fractional digits
        // Multiply frac by 10 repeatedly, extract digit, up to 15 digits max
        // But trim trailing zeros (keep at least 1 digit)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(12)); // frac_digits = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if frac_digits >= 15, break
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(15));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // frac_part *= 10
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::F64Const(10.0));
        f.instruction(&Instruction::F64Mul);
        f.instruction(&Instruction::LocalSet(5));
        // digit = trunc(frac_part) as i32
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32TruncSatF64S);
        f.instruction(&Instruction::LocalSet(6)); // temp digit
        // frac_part -= f64(digit)
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::F64ConvertI32S);
        f.instruction(&Instruction::F64Sub);
        f.instruction(&Instruction::LocalSet(5));
        // write digit + '0'
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        // frac_digits++
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(12));
        // if frac_part is effectively zero AND frac_digits >= 1, break
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::F64Const(1e-14));
        f.instruction(&Instruction::F64Lt);
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::I32And);
        f.instruction(&Instruction::BrIf(1)); // break if frac ~= 0 and at least 1 digit
        f.instruction(&Instruction::Br(0)); // continue
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Now trim trailing zeros (but keep at least 1 fractional digit)
        // write_pos currently points past the last digit written
        // Scan backwards while scratch[write_pos-1] == '0' AND more than 1 frac digit
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if frac_digits <= 1, stop trimming
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32LeU);
        f.instruction(&Instruction::BrIf(1));
        // if scratch[write_pos - 1] != '0', stop
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::BrIf(1));
        // write_pos--
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(3));
        // frac_digits--
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(12));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // total_len = write_pos - buf_base
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(buf_base));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(10)); // total_len

        // Create GC string
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(9));

        // Copy bytes from scratch to GC array
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(8)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Const(buf_base));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::ArraySet(self.string_ty));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(8));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __parse_i32(s: ref $string) -> ref $Result
    /// Parses a decimal integer from a GC string, returns Ok(i32) or Err(String)
    pub(super) fn emit_parse_i32_helper(&self, codes: &mut CodeSection) {
        let result_base = *self.enum_base_types.get("Result").unwrap();
        let result_ok = *self
            .enum_variant_types
            .get("Result")
            .unwrap()
            .get("Ok")
            .unwrap();
        let result_err = *self
            .enum_variant_types
            .get("Result")
            .unwrap()
            .get("Err")
            .unwrap();
        let _result_ref = ref_nullable(result_base);

        // locals: s(param0), len(1), i(2), neg(3), result(4), ch(5)
        let mut f = Function::new(vec![
            (1, ValType::I32), // len
            (1, ValType::I32), // i
            (1, ValType::I32), // neg
            (1, ValType::I32), // result
            (1, ValType::I32), // ch
        ]);

        // len = array.len(s)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        // if len == 0: return Err("invalid number")
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            let seg = self.find_or_make_err_string();
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(28));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: seg,
            });
            f.instruction(&Instruction::StructNew(result_err));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // Check for leading '-'
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3)); // neg = 1
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(2)); // i = 1
            // if len == 1 (just "-"): return Err
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
        }
        f.instruction(&Instruction::End);

        // Parse loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        {
            // if i >= len: break
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1)); // break outer block

            // ch = array.get_u(s, i)
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::ArrayGetU(self.string_ty));
            f.instruction(&Instruction::LocalSet(5));

            // if ch < '0' || ch > '9': return Err
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32LtU);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(57));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::I32Or);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);

            // result = result * 10 + (ch - '0')
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32Mul);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));

            // i += 1
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));

            f.instruction(&Instruction::Br(0)); // continue loop
        }
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // if neg: result = -result
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // return Ok(result)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::StructNew(result_ok));

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __parse_i64(s: ref $string) -> ref $Result_i64_String
    pub(super) fn emit_parse_i64_helper(&self, codes: &mut CodeSection) {
        let _result_base = *self.enum_base_types.get("Result_i64_String").unwrap();
        let result_ok = *self
            .enum_variant_types
            .get("Result_i64_String")
            .unwrap()
            .get("Ok")
            .unwrap();
        let result_err = *self
            .enum_variant_types
            .get("Result_i64_String")
            .unwrap()
            .get("Err")
            .unwrap();

        // locals: s(param0), len(1), i(2), neg(3), result_lo(4):i32, result_hi(5):i64, ch(6)
        let mut f = Function::new(vec![
            (1, ValType::I32), // len
            (1, ValType::I32), // i
            (1, ValType::I32), // neg
            (1, ValType::I64), // result
            (1, ValType::I32), // ch
        ]);

        // len = array.len(s)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        // if len == 0: return Err
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            let seg = self.find_or_make_err_string();
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(28));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: seg,
            });
            f.instruction(&Instruction::StructNew(result_err));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // Check for '-'
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Const(45));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
        }
        f.instruction(&Instruction::End);

        // Parse loop (i64 accumulation)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));

            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::ArrayGetU(self.string_ty));
            f.instruction(&Instruction::LocalSet(5));

            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32LtU);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(57));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::I32Or);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);

            // result = result * 10 + (ch - 48) as i64
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64Mul);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::I64ExtendI32U);
            f.instruction(&Instruction::I64Add);
            f.instruction(&Instruction::LocalSet(4));

            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));

            f.instruction(&Instruction::Br(0));
        }
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // if neg: result = -result
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I64Const(0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I64Sub);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // return Ok(result)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::StructNew(result_ok));

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __parse_f64(s: ref $string) -> ref $Result_f64_String
    /// Simplified: parse integer part + optional decimal part
    pub(super) fn emit_parse_f64_helper(&self, codes: &mut CodeSection) {
        let _result_base = *self.enum_base_types.get("Result_f64_String").unwrap();
        let result_ok = *self
            .enum_variant_types
            .get("Result_f64_String")
            .unwrap()
            .get("Ok")
            .unwrap();
        let result_err = *self
            .enum_variant_types
            .get("Result_f64_String")
            .unwrap()
            .get("Err")
            .unwrap();

        // locals: s(0), len(1), i(2), neg(3), int_part(4):f64, frac_part(5):f64, divisor(6):f64, ch(7), has_dot(8)
        let mut f = Function::new(vec![
            (1, ValType::I32), // len
            (1, ValType::I32), // i
            (1, ValType::I32), // neg
            (1, ValType::F64), // int_part
            (1, ValType::F64), // frac_part
            (1, ValType::F64), // divisor
            (1, ValType::I32), // ch
            (1, ValType::I32), // has_dot
        ]);

        // len = array.len(s)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        // if len == 0: return Err
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            let seg = self.find_or_make_err_float_string();
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(26));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: seg,
            });
            f.instruction(&Instruction::StructNew(result_err));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // divisor = 1.0
        f.instruction(&Instruction::F64Const(1.0));
        f.instruction(&Instruction::LocalSet(6));

        // Check for '-'
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Const(45));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_float_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(26));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
        }
        f.instruction(&Instruction::End);

        // Parse loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));

            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::ArrayGetU(self.string_ty));
            f.instruction(&Instruction::LocalSet(7));

            // Check for '.'
            f.instruction(&Instruction::LocalGet(7));
            f.instruction(&Instruction::I32Const(46)); // '.'
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                f.instruction(&Instruction::I32Const(1));
                f.instruction(&Instruction::LocalSet(8)); // has_dot = 1
                f.instruction(&Instruction::LocalGet(2));
                f.instruction(&Instruction::I32Const(1));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::LocalSet(2));
                f.instruction(&Instruction::Br(1)); // continue loop
            }
            f.instruction(&Instruction::End);

            // Check digit range
            f.instruction(&Instruction::LocalGet(7));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32LtU);
            f.instruction(&Instruction::LocalGet(7));
            f.instruction(&Instruction::I32Const(57));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::I32Or);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_float_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(26));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);

            // digit = (ch - 48) as f64
            f.instruction(&Instruction::LocalGet(8)); // has_dot
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                // frac_part = frac_part * 10 + digit; divisor *= 10
                f.instruction(&Instruction::LocalGet(5));
                f.instruction(&Instruction::F64Const(10.0));
                f.instruction(&Instruction::F64Mul);
                f.instruction(&Instruction::LocalGet(7));
                f.instruction(&Instruction::I32Const(48));
                f.instruction(&Instruction::I32Sub);
                f.instruction(&Instruction::F64ConvertI32U);
                f.instruction(&Instruction::F64Add);
                f.instruction(&Instruction::LocalSet(5));
                f.instruction(&Instruction::LocalGet(6));
                f.instruction(&Instruction::F64Const(10.0));
                f.instruction(&Instruction::F64Mul);
                f.instruction(&Instruction::LocalSet(6));
            }
            f.instruction(&Instruction::Else);
            {
                // int_part = int_part * 10 + digit
                f.instruction(&Instruction::LocalGet(4));
                f.instruction(&Instruction::F64Const(10.0));
                f.instruction(&Instruction::F64Mul);
                f.instruction(&Instruction::LocalGet(7));
                f.instruction(&Instruction::I32Const(48));
                f.instruction(&Instruction::I32Sub);
                f.instruction(&Instruction::F64ConvertI32U);
                f.instruction(&Instruction::F64Add);
                f.instruction(&Instruction::LocalSet(4));
            }
            f.instruction(&Instruction::End);

            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));

            f.instruction(&Instruction::Br(0));
        }
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // result = int_part + frac_part / divisor → store in int_part (local 4)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::F64Div);
        f.instruction(&Instruction::F64Add);
        f.instruction(&Instruction::LocalSet(4)); // result in local 4

        // if neg: result = -result
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::F64Neg);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // return Ok(result)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::StructNew(result_ok));

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Find or allocate the "invalid number" error string data segment
    pub(super) fn find_or_make_err_string(&self) -> u32 {
        self.data_segs.len() as u32 + self.err_string_seg.unwrap_or(0)
    }

    pub(super) fn find_or_make_err_float_string(&self) -> u32 {
        self.data_segs.len() as u32 + self.err_float_string_seg.unwrap_or(0)
    }

    // ── User function emission ───────────────────────────────────

    /// Emit a stub body for a builtin function (inlined at call sites).
    /// The body is `unreachable` — it should never be called at runtime.
    pub(super) fn emit_builtin_stub(&self, codes: &mut CodeSection, _func: &MirFunction) {
        let f_locals: Vec<(u32, ValType)> = vec![];
        let mut f = Function::new(f_locals);
        f.instruction(&Instruction::Unreachable);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_function(&mut self, codes: &mut CodeSection, func: &MirFunction) {
        // Track whether this is the _start/main function
        self.is_start_fn = func.name == "main" || func.name == "_start";
        // Reset per-function state
        self.string_locals.clear();
        self.f64_locals.clear();
        self.i64_locals.clear();
        self.bool_locals.clear();
        self.any_locals.clear();
        self.f64_vec_locals.clear();
        self.i64_vec_locals.clear();
        self.i32_vec_locals.clear();
        self.string_vec_locals.clear();
        self.struct_vec_locals.clear();
        self.local_struct.clear();
        self.local_enum.clear();
        self.current_fn_type_params = func.type_params.clone();
        self.current_fn_return_ty = func.return_ty.clone();
        self.current_fn_ret_type_name = self.fn_ret_type_names.get(&func.name).cloned();

        // ── Type propagation: scan ALL assignments (including nested) to infer ref types ──
        let mut extra_enum: HashMap<u32, String> = HashMap::new();
        let mut extra_struct: HashMap<u32, String> = HashMap::new();
        let mut extra_vec_i32: HashSet<u32> = HashSet::new();
        let mut extra_vec_i64: HashSet<u32> = HashSet::new();
        let mut extra_vec_f64: HashSet<u32> = HashSet::new();
        let mut extra_vec_string: HashSet<u32> = HashSet::new();
        let mut extra_string: HashSet<u32> = HashSet::new();

        // Collect ALL statements recursively (flattens IfStmt/WhileStmt bodies)
        fn collect_stmts_recursive<'a>(stmts: &'a [MirStmt], out: &mut Vec<&'a MirStmt>) {
            for stmt in stmts {
                out.push(stmt);
                match stmt {
                    MirStmt::IfStmt {
                        then_body,
                        else_body,
                        ..
                    } => {
                        collect_stmts_recursive(then_body, out);
                        collect_stmts_recursive(else_body, out);
                    }
                    MirStmt::WhileStmt { body, .. } => {
                        collect_stmts_recursive(body, out);
                    }
                    _ => {}
                }
            }
        }
        // Also collect stmts embedded in Operand trees (e.g. IfExpr.then_body inside Return)
        fn collect_stmts_from_operand<'a>(op: &'a Operand, out: &mut Vec<&'a MirStmt>) {
            match op {
                Operand::IfExpr {
                    cond,
                    then_body,
                    then_result,
                    else_body,
                    else_result,
                } => {
                    collect_stmts_from_operand(cond, out);
                    collect_stmts_recursive(then_body, out);
                    collect_stmts_recursive(else_body, out);
                    if let Some(tr) = then_result {
                        collect_stmts_from_operand(tr, out);
                    }
                    if let Some(er) = else_result {
                        collect_stmts_from_operand(er, out);
                    }
                }
                Operand::LoopExpr { body, result, .. } => {
                    collect_stmts_recursive(body, out);
                    collect_stmts_from_operand(result, out);
                }
                Operand::BinOp(_, a, b) => {
                    collect_stmts_from_operand(a, out);
                    collect_stmts_from_operand(b, out);
                }
                Operand::UnaryOp(_, a) => {
                    collect_stmts_from_operand(a, out);
                }
                Operand::TryExpr { expr, .. } => {
                    collect_stmts_from_operand(expr, out);
                }
                _ => {}
            }
        }
        let mut all_stmts: Vec<&MirStmt> = Vec::new();
        for block in &func.blocks {
            collect_stmts_recursive(&block.stmts, &mut all_stmts);
            // Also scan operands inside the terminator (especially Return)
            match &block.terminator {
                Terminator::Return(Some(op)) => collect_stmts_from_operand(op, &mut all_stmts),
                Terminator::If { cond, .. } => collect_stmts_from_operand(cond, &mut all_stmts),
                Terminator::Switch { scrutinee, .. } => {
                    collect_stmts_from_operand(scrutinee, &mut all_stmts)
                }
                _ => {}
            }
        }

        // Multi-pass to propagate through chains (e.g., a = b = c where c is typed)
        for _pass in 0..3 {
            for stmt in &all_stmts {
                match stmt {
                    MirStmt::Assign(Place::Local(dst), Rvalue::Use(op)) => {
                        match op {
                            Operand::Place(Place::Local(src)) => {
                                let ename_opt = func
                                    .enum_typed_locals
                                    .get(&src.0)
                                    .or_else(|| extra_enum.get(&src.0))
                                    .cloned();
                                if let Some(ename) = ename_opt {
                                    extra_enum.entry(dst.0).or_insert(ename);
                                }
                                let sname_opt = func
                                    .struct_typed_locals
                                    .get(&src.0)
                                    .or_else(|| extra_struct.get(&src.0))
                                    .cloned();
                                if let Some(sname) = sname_opt {
                                    extra_struct.entry(dst.0).or_insert(sname);
                                }
                                if extra_vec_i32.contains(&src.0) {
                                    extra_vec_i32.insert(dst.0);
                                }
                                if extra_vec_i64.contains(&src.0) {
                                    extra_vec_i64.insert(dst.0);
                                }
                                if extra_vec_f64.contains(&src.0) {
                                    extra_vec_f64.insert(dst.0);
                                }
                                if extra_vec_string.contains(&src.0) {
                                    extra_vec_string.insert(dst.0);
                                }
                                if extra_string.contains(&src.0) {
                                    extra_string.insert(dst.0);
                                }
                                // Propagate struct-vec
                                if let Some(svn) = self.struct_vec_locals.get(&src.0).cloned() {
                                    self.struct_vec_locals.entry(dst.0).or_insert(svn);
                                }
                            }
                            Operand::EnumInit { enum_name, .. } => {
                                extra_enum.entry(dst.0).or_insert_with(|| enum_name.clone());
                            }
                            Operand::EnumPayload {
                                object,
                                enum_name,
                                variant_name,
                                index,
                                ..
                            } => {
                                // The payload type comes from enum_variant_field_types
                                let effective_enum_name =
                                    if matches!(enum_name.as_str(), "Result" | "Option") {
                                        if let Operand::Place(Place::Local(src)) = &**object {
                                            func.enum_typed_locals
                                                .get(&src.0)
                                                .or_else(|| extra_enum.get(&src.0))
                                                .cloned()
                                                .unwrap_or_else(|| enum_name.clone())
                                        } else {
                                            enum_name.clone()
                                        }
                                    } else {
                                        enum_name.clone()
                                    };
                                let key = (effective_enum_name, variant_name.clone());
                                if let Some(field_types) = self.enum_variant_field_types.get(&key) {
                                    if let Some(ft) = field_types.get(*index as usize) {
                                        if ft == "String" {
                                            extra_string.insert(dst.0);
                                        }
                                        if self.enum_base_types.contains_key(ft.as_str()) {
                                            extra_enum.entry(dst.0).or_insert_with(|| ft.clone());
                                        }
                                        if self.struct_gc_types.contains_key(ft.as_str()) {
                                            extra_struct.entry(dst.0).or_insert_with(|| ft.clone());
                                        }
                                    }
                                }
                            }
                            Operand::TryExpr { expr, .. } => {
                                let ok_enum_name = match &**expr {
                                    Operand::Call(name, _) => self
                                        .fn_ret_type_names
                                        .get(name)
                                        .and_then(|ret_name| self.result_enum_name_for_type_name(ret_name)),
                                    Operand::Place(Place::Local(src)) => func
                                        .enum_typed_locals
                                        .get(&src.0)
                                        .or_else(|| extra_enum.get(&src.0))
                                        .cloned(),
                                    _ => None,
                                };
                                if let Some(enum_name) = ok_enum_name {
                                    let key = (enum_name, "Ok".to_string());
                                    if let Some(field_types) = self.enum_variant_field_types.get(&key) {
                                        if let Some(ft) = field_types.first() {
                                            if ft == "String" {
                                                extra_string.insert(dst.0);
                                            }
                                            if self.enum_base_types.contains_key(ft.as_str()) {
                                                extra_enum.entry(dst.0).or_insert_with(|| ft.clone());
                                            }
                                            if self.struct_gc_types.contains_key(ft.as_str()) {
                                                extra_struct.entry(dst.0).or_insert_with(|| ft.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            Operand::StructInit { name, .. } => {
                                extra_struct.entry(dst.0).or_insert_with(|| name.clone());
                            }
                            Operand::Call(name, _) => {
                                let canonical = normalize_intrinsic(name);
                                if let Some(sname) = canonical.strip_prefix("Vec_new_") {
                                    match sname {
                                        "i32" => {
                                            extra_vec_i32.insert(dst.0);
                                        }
                                        "i64" => {
                                            extra_vec_i64.insert(dst.0);
                                        }
                                        "f64" => {
                                            extra_vec_f64.insert(dst.0);
                                        }
                                        "String" => {
                                            extra_vec_string.insert(dst.0);
                                        }
                                        _ => {
                                            if self.custom_vec_types.contains_key(sname) {
                                                self.struct_vec_locals
                                                    .insert(dst.0, sname.to_string());
                                            }
                                        }
                                    }
                                }
                                // Propagate vec type from filter/map calls
                                match canonical {
                                    "filter_i32" => {
                                        extra_vec_i32.insert(dst.0);
                                    }
                                    "filter_i64" => {
                                        extra_vec_i64.insert(dst.0);
                                    }
                                    "filter_f64" => {
                                        extra_vec_f64.insert(dst.0);
                                    }
                                    "filter_String" => {
                                        extra_vec_string.insert(dst.0);
                                    }
                                    "map_i32_i32" => {
                                        extra_vec_i32.insert(dst.0);
                                    }
                                    "map_i64_i64" => {
                                        extra_vec_i64.insert(dst.0);
                                    }
                                    "map_f64_f64" => {
                                        extra_vec_f64.insert(dst.0);
                                    }
                                    "map_i32_String" | "map_i64_String" | "map_f64_String"
                                    | "map_String_String" => {
                                        extra_vec_string.insert(dst.0);
                                    }
                                    "parse_i32" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result".to_string());
                                    }
                                    "parse_i64" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result_i64_String".to_string());
                                    }
                                    "parse_f64" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result_f64_String".to_string());
                                    }
                                    "fs_read_file" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result_String_String".to_string());
                                    }
                                    "fs_write_file" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result".to_string());
                                    }
                                    "find_i32" | "find_String" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Option".to_string());
                                    }
                                    "HashMap_i32_i32_new" => {
                                        extra_struct
                                            .entry(dst.0)
                                            .or_insert_with(|| "__hashmap_i32_i32".to_string());
                                    }
                                    "HashMap_i32_i32_get" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Option".to_string());
                                    }
                                    _ => {}
                                }
                                // Also check fn_ret_type_names for enum return types
                                if let Some(ret_name) = self.fn_ret_type_names.get(canonical) {
                                    if self.enum_base_types.contains_key(ret_name.as_str()) {
                                        extra_enum.entry(dst.0).or_insert_with(|| ret_name.clone());
                                    } else if let Some(specialized_name) =
                                        nominalize_generic_type_name(ret_name)
                                    {
                                        if self
                                            .enum_base_types
                                            .contains_key(specialized_name.as_str())
                                        {
                                            extra_enum
                                                .entry(dst.0)
                                                .or_insert_with(|| specialized_name);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    MirStmt::CallBuiltin {
                        dest: Some(Place::Local(dst)),
                        name,
                        ..
                    } => {
                        let canonical = normalize_intrinsic(name);
                        if let Some(sname) = canonical.strip_prefix("Vec_new_") {
                            match sname {
                                "i32" => {
                                    extra_vec_i32.insert(dst.0);
                                }
                                "i64" => {
                                    extra_vec_i64.insert(dst.0);
                                }
                                "f64" => {
                                    extra_vec_f64.insert(dst.0);
                                }
                                "String" => {
                                    extra_vec_string.insert(dst.0);
                                }
                                _ => {
                                    if self.custom_vec_types.contains_key(sname) {
                                        self.struct_vec_locals.insert(dst.0, sname.to_string());
                                    }
                                }
                            }
                        }
                        match canonical {
                            "filter_i32" => {
                                extra_vec_i32.insert(dst.0);
                            }
                            "filter_i64" => {
                                extra_vec_i64.insert(dst.0);
                            }
                            "filter_f64" => {
                                extra_vec_f64.insert(dst.0);
                            }
                            "filter_String" => {
                                extra_vec_string.insert(dst.0);
                            }
                            "map_i32_i32" => {
                                extra_vec_i32.insert(dst.0);
                            }
                            "map_i64_i64" => {
                                extra_vec_i64.insert(dst.0);
                            }
                            "map_f64_f64" => {
                                extra_vec_f64.insert(dst.0);
                            }
                            "map_i32_String" | "map_i64_String" | "map_f64_String"
                            | "map_String_String" => {
                                extra_vec_string.insert(dst.0);
                            }
                            "parse_i32" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result".to_string());
                            }
                            "parse_i64" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result_i64_String".to_string());
                            }
                            "parse_f64" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result_f64_String".to_string());
                            }
                            "fs_read_file" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result_String_String".to_string());
                            }
                            "fs_write_file" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result".to_string());
                            }
                            "find_i32" | "find_String" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Option".to_string());
                            }
                            "HashMap_i32_i32_new" => {
                                extra_struct
                                    .entry(dst.0)
                                    .or_insert_with(|| "__hashmap_i32_i32".to_string());
                            }
                            "HashMap_i32_i32_get" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Option".to_string());
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            // Also propagate from MIR-declared vec types
            for local in func.params.iter().chain(func.locals.iter()) {
                if let Type::Vec(elem) = &local.ty {
                    match elem.as_ref() {
                        Type::I32 | Type::Bool => {
                            extra_vec_i32.insert(local.id.0);
                        }
                        Type::I64 => {
                            extra_vec_i64.insert(local.id.0);
                        }
                        Type::F64 => {
                            extra_vec_f64.insert(local.id.0);
                        }
                        Type::String => {
                            extra_vec_string.insert(local.id.0);
                        }
                        _ => {
                            extra_vec_i32.insert(local.id.0);
                        }
                    }
                }
            }
        }

        // Merge extra maps into the function's typed_locals for local_val_type
        let mut merged_struct = func.struct_typed_locals.clone();
        for (k, v) in &extra_struct {
            merged_struct.entry(*k).or_insert_with(|| v.clone());
        }
        let mut merged_enum = func.enum_typed_locals.clone();
        for (k, v) in &extra_enum {
            // Specialized names (e.g. Result_i64_String) override generic (Result)
            let should_override = v.starts_with("Result_") || v.starts_with("Option_");
            if should_override {
                merged_enum.insert(*k, v.clone());
            } else {
                merged_enum.entry(*k).or_insert_with(|| v.clone());
            }
        }

        // Merge vec sets: propagated from assignment scan + type scan
        let vec_sets = Some((
            &extra_vec_i32,
            &extra_vec_i64,
            &extra_vec_f64,
            &extra_vec_string,
        ));

        // Collect local types (skip params — they are already in the func signature)
        let num_params = func.params.len();
        let mut local_types: Vec<(u32, ValType)> = Vec::new();
        for local in func.locals.iter().skip(num_params) {
            let vt = if extra_string.contains(&local.id.0) {
                ref_nullable(self.string_ty)
            } else {
                self.local_val_type(local, &merged_struct, &merged_enum, vec_sets)
            };
            local_types.push((1, vt));
        }
        // Track type metadata for all locals (including params)
        for local in func.params.iter().chain(func.locals.iter()) {
            match &local.ty {
                Type::String => {
                    self.string_locals.insert(local.id.0);
                }
                Type::F64 => {
                    self.f64_locals.insert(local.id.0);
                }
                Type::I64 | Type::U64 => {
                    self.i64_locals.insert(local.id.0);
                }
                Type::Bool => {
                    self.bool_locals.insert(local.id.0);
                }
                Type::Vec(elem) => match elem.as_ref() {
                    Type::F64 => {
                        self.f64_vec_locals.insert(local.id.0);
                    }
                    Type::I64 => {
                        self.i64_vec_locals.insert(local.id.0);
                    }
                    Type::String => {
                        self.string_vec_locals.insert(local.id.0);
                    }
                    _ => {
                        self.i32_vec_locals.insert(local.id.0);
                    }
                },
                Type::Any => {
                    self.any_locals.insert(local.id.0);
                }
                _ => {}
            }
        }
        for lid in &extra_string {
            self.string_locals.insert(*lid);
        }
        // Populate struct/enum local maps from side-channel + propagated types
        for (lid, sname) in &merged_struct {
            self.local_struct
                .entry(*lid)
                .or_insert_with(|| sname.clone());
        }
        for (lid, ename) in &merged_enum {
            self.local_enum.entry(*lid).or_insert_with(|| ename.clone());
        }
        // Merge propagated vec types into runtime sets
        for lid in &extra_vec_i32 {
            self.i32_vec_locals.insert(*lid);
        }
        for lid in &extra_vec_i64 {
            self.i64_vec_locals.insert(*lid);
        }
        for lid in &extra_vec_f64 {
            self.f64_vec_locals.insert(*lid);
        }
        for lid in &extra_vec_string {
            self.string_vec_locals.insert(*lid);
        }
        // Track generic function params as enum/vec locals based on fn_param_type_names
        if !func.type_params.is_empty() {
            if let Some(param_names) = self.fn_param_type_names.get(&func.name).cloned() {
                for (i, pname) in param_names.iter().enumerate() {
                    if let Some(p) = func.params.get(i) {
                        if pname.starts_with("Option") {
                            self.local_enum.insert(p.id.0, "Option".to_string());
                        } else if pname.starts_with("Result") {
                            let rname = if pname.contains("i64") {
                                "Result_i64_String"
                            } else if pname.contains("f64") {
                                "Result_f64_String"
                            } else if pname.contains("String, String")
                                || pname.contains("String,String")
                            {
                                "Result_String_String"
                            } else {
                                "Result"
                            };
                            self.local_enum.insert(p.id.0, rname.to_string());
                        } else if pname.starts_with("Vec<") {
                            let inner = &pname[4..pname.len().saturating_sub(1)];
                            match inner {
                                "i64" => {
                                    self.i64_vec_locals.insert(p.id.0);
                                }
                                "f64" => {
                                    self.f64_vec_locals.insert(p.id.0);
                                }
                                "String" => {
                                    self.string_vec_locals.insert(p.id.0);
                                }
                                _ => {
                                    self.i32_vec_locals.insert(p.id.0);
                                }
                            }
                        }
                    }
                }
            }
        }
        // Add scratch locals for GC string operations:
        // +0: i32 (counter/index), +1: i32, +2: i32, +3: i32,
        // +4: ref $string, +5: ref $string
        // +6: i64, +7: f64, +8: ref $string (extra for vec ops)
        // +9: i32 (extra scratch for nested loops)
        // +10: anyref (for TryExpr ? operator)
        self.scratch_base = func.locals.len() as u32;
        let str_ref = ref_nullable(self.string_ty);
        local_types.push((1, ValType::I32));
        local_types.push((1, ValType::I32));
        local_types.push((1, ValType::I32));
        local_types.push((1, ValType::I32));
        local_types.push((1, str_ref));
        local_types.push((1, str_ref));
        local_types.push((1, ValType::I64)); // si(6): i64 scratch
        local_types.push((1, ValType::F64)); // si(7): f64 scratch
        local_types.push((1, str_ref)); // si(8): extra ref scratch
        local_types.push((1, ValType::I32)); // si(9): extra i32 scratch
        // si(10): anyref scratch for TryExpr (? operator)
        let anyref_ty = ValType::Ref(WasmRefType {
            nullable: true,
            heap_type: HeapType::Abstract {
                shared: false,
                ty: wasm_encoder::AbstractHeapType::Any,
            },
        });
        local_types.push((1, anyref_ty));
        // si(11): anyref scratch #2 for HOF operations
        local_types.push((1, anyref_ty));

        let mut f = Function::new(local_types);

        // Wrap in PeepholeWriter for local.set/get → local.tee optimization
        let tee_count = {
            let mut w = PeepholeWriter::new(&mut f, self.opt_level);

            // Emit statements from entry block
            if let Some(block) = func.blocks.first() {
                for stmt in &block.stmts {
                    self.emit_stmt(&mut w, stmt);
                }
                // Handle terminator
                match &block.terminator {
                    Terminator::Return(Some(op)) => {
                        if func.name == "main" || func.name == "_start" {
                            // WASI _start must be () -> (); emit for side effects but discard result
                            if !matches!(op, Operand::Unit) {
                                self.emit_operand(&mut w, op);
                                if self.operand_produces_value(op) {
                                    w.instruction(&Instruction::Drop);
                                }
                            }
                        } else {
                            self.emit_operand(&mut w, op);
                            // Box value types when returning from generic function with anyref return
                            if self.current_fn_return_ty == Type::Any {
                                let op_vt = self.infer_operand_type(op);
                                if op_vt == ValType::I32 {
                                    w.instruction(&Instruction::RefI31);
                                }
                            }
                        }
                        w.instruction(&Instruction::Return);
                    }
                    Terminator::Return(None) => {
                        w.instruction(&Instruction::Return);
                    }
                    _ => {}
                }
            }
            w.instruction(&Instruction::End);
            w.flush();
            w.tee_count()
        };
        if tee_count > 0 {
            // Peephole applied: `tee_count` local.set/get pairs → local.tee
            let _ = tee_count;
        }
        codes.function(&f);
    }
}
