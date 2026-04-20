//! Helper function emission for the T3 Wasm GC emitter.
//!
//! Emits built-in helper functions (print, i32_to_str, parse_i32, etc.)
//! and the main user function emission logic.

use ark_mir::mir::*;
use ark_typecheck::types::Type;
use std::collections::{HashMap, HashSet};
use wasm_encoder::{
    CodeSection, Function, HeapType, Instruction, MemArg, RefType as WasmRefType, ValType,
};

use super::i31ref as i31;
use super::peephole::PeepholeWriter;
use super::{Ctx, nominalize_generic_type_name, normalize_intrinsic, ref_nullable};
use super::{I32BUF, P2_RETPTR, SCRATCH};

impl Ctx {
    // ── Helper function bodies ───────────────────────────────────

    pub(super) fn emit_wasi_p2_get_stdout_shim(&self, codes: &mut CodeSection) {
        let mut f = Function::new([]);
        // Call the P2 get_stdout import which returns i32 (stream handle)
        f.instruction(&Instruction::Call(self.wasi_p2_import_get_stdout));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_wasi_p2_get_stderr_shim(&self, codes: &mut CodeSection) {
        let mut f = Function::new([]);
        f.instruction(&Instruction::I32Const(2)); // fd 2 = stderr
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit eprint_str_ln: write GC string + newline to stderr (fd 2)
    pub(super) fn emit_eprint_str_ln_helper(
        &self,
        codes: &mut CodeSection,
        newline_off: u32,
        get_stderr_idx: u32,
    ) {
        let ma1 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        // Param 0 = (ref null $string). Locals 1=len, 2=i, 3=handle.
        let mut f = Function::new([(3, ValType::I32)]);

        f.instruction(&Instruction::Call(get_stderr_idx));
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

    pub(super) fn emit_wasi_p2_write_and_flush_shim(&self, codes: &mut CodeSection) {
        // Params: 0=stream handle, 1=ptr, 2=len, 3=retptr (unused in P2).
        // P2 signature: (stream, ptr, len) -> result (i32 error code)
        let mut f = Function::new([(0, ValType::I32), (1, ValType::I32), (2, ValType::I32), (3, ValType::I32)]);

        // Call P2 import directly with 3 params
        f.instruction(&Instruction::LocalGet(0)); // stream
        f.instruction(&Instruction::LocalGet(1)); // ptr
        f.instruction(&Instruction::LocalGet(2)); // len
        f.instruction(&Instruction::Call(self.wasi_p2_import_write_and_flush));
        // Drop the result (error code) to match registered signature
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_wasi_p2_drop_output_stream_shim(&self, codes: &mut CodeSection) {
        // P2 signature: (stream: i32) -> ()
        // Params: 0=stream handle
        let mut f = Function::new([(0, ValType::I32)]);
        // Call the P2 drop import
        f.instruction(&Instruction::LocalGet(0)); // stream
        f.instruction(&Instruction::Call(self.wasi_p2_import_drop_output_stream));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_str_helper(&self, codes: &mut CodeSection) {
        let ma1 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        // Param 0 = (ref null $string). Locals 1=len, 2=i, 3=handle.
        let mut f = Function::new([(3, ValType::I32)]);

        // Get stdout handle
        if self.wasi_version == ark_target::WasiVersion::P2 {
            f.instruction(&Instruction::Call(self.wasi_p2_import_get_stdout));
        } else {
            f.instruction(&Instruction::Call(self.wasi_p2_get_stdout));
        }
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

        // Write string
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(1));
        if self.wasi_version == ark_target::WasiVersion::P2 {
            // P2: call blocking_write_and_flush directly (stream, ptr, len) -> result
            f.instruction(&Instruction::Call(self.wasi_p2_import_write_and_flush));
            f.instruction(&Instruction::Drop); // drop error code
        } else {
            // P1: call shim with (stream, ptr, len, retptr)
            f.instruction(&Instruction::I32Const(P2_RETPTR as i32));
            f.instruction(&Instruction::Call(self.wasi_p2_write_and_flush));
        }
        f.instruction(&Instruction::LocalGet(3));
        if self.wasi_version == ark_target::WasiVersion::P2 {
            f.instruction(&Instruction::Call(self.wasi_p2_import_drop_output_stream));
        } else {
            f.instruction(&Instruction::Call(self.wasi_p2_drop_output_stream));
        }
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    pub(super) fn emit_print_i32_helper(&self, codes: &mut CodeSection) {
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
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
        let ma1 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
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
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
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
        self.char_locals.clear();
        self.any_locals.clear();
        self.f64_vec_locals.clear();
        self.i64_vec_locals.clear();
        self.i32_vec_locals.clear();
        self.string_vec_locals.clear();
        self.struct_vec_locals.clear();
        self.local_struct.clear();
        self.local_enum.clear();
        self.fn_ref_locals.clear();
        self.current_fn_type_params = func.type_params.clone();
        self.current_fn_return_ty = func.return_ty.clone();
        self.current_fn_ret_type_name = self.fn_ret_type_names.get(&func.name).cloned();
        self.current_emit_fn_idx = self
            .fn_map
            .get(&func.name)
            .copied()
            .expect("emit_function: missing wasm function index for MIR function");

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
                                if let Some(field_types) = self.enum_variant_field_types.get(&key)
                                    && let Some(ft) = field_types.get(*index as usize)
                                {
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
                            Operand::TryExpr { expr, .. } => {
                                let ok_enum_name = match &**expr {
                                    Operand::Call(name, _) => {
                                        self.fn_ret_type_names.get(name).and_then(|ret_name| {
                                            self.result_enum_name_for_type_name(ret_name)
                                        })
                                    }
                                    Operand::Place(Place::Local(src)) => func
                                        .enum_typed_locals
                                        .get(&src.0)
                                        .or_else(|| extra_enum.get(&src.0))
                                        .cloned(),
                                    _ => None,
                                };
                                if let Some(enum_name) = ok_enum_name {
                                    let key = (enum_name, "Ok".to_string());
                                    if let Some(field_types) =
                                        self.enum_variant_field_types.get(&key)
                                        && let Some(ft) = field_types.first()
                                    {
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
                            Operand::StructInit { name, .. } => {
                                extra_struct.entry(dst.0).or_insert_with(|| name.clone());
                            }
                            Operand::Call(name, args) => {
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
                                    "env_var" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Option_String".to_string());
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
                                    "HashMap_new_String_i32" => {
                                        extra_struct
                                            .entry(dst.0)
                                            .or_insert_with(|| "__hashmap_str_i32".to_string());
                                    }
                                    "HashMap_String_i32_get" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Option".to_string());
                                    }
                                    // get_unchecked on Vec<Struct> → result is a struct
                                    "get_unchecked" | "get" => {
                                        if let Some(first_arg) = args.first()
                                            && let Operand::Place(Place::Local(src)) = first_arg
                                        {
                                            if let Some(sname) =
                                                self.struct_vec_locals.get(&src.0).cloned()
                                            {
                                                extra_struct.entry(dst.0).or_insert(sname);
                                            } else if canonical == "get"
                                                && (extra_vec_string.contains(&src.0)
                                                    || self.string_vec_locals.contains(&src.0))
                                                && self
                                                    .enum_base_types
                                                    .contains_key("Option_String")
                                            {
                                                extra_enum
                                                    .entry(dst.0)
                                                    .or_insert_with(|| "Option_String".to_string());
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                                // Also check fn_ret_type_names for enum return types
                                if let Some(ret_name) = self.fn_ret_type_names.get(canonical) {
                                    if self.enum_base_types.contains_key(ret_name.as_str()) {
                                        extra_enum.entry(dst.0).or_insert_with(|| ret_name.clone());
                                    } else if let Some(specialized_name) =
                                        nominalize_generic_type_name(ret_name)
                                        && self
                                            .enum_base_types
                                            .contains_key(specialized_name.as_str())
                                    {
                                        extra_enum.entry(dst.0).or_insert_with(|| specialized_name);
                                    } else if self.struct_gc_types.contains_key(ret_name.as_str()) {
                                        // Function returns a struct type
                                        extra_struct
                                            .entry(dst.0)
                                            .or_insert_with(|| ret_name.clone());
                                    }
                                }
                            }
                            Operand::FnRef(name) => {
                                // Record that this local holds a named function reference,
                                // to enable return_call_ref emission in tail position.
                                self.fn_ref_locals.insert(dst.0, name.clone());
                            }
                            _ => {}
                        }
                    }
                    MirStmt::CallBuiltin {
                        dest: Some(Place::Local(dst)),
                        name,
                        args,
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
                            "http_get" | "http_request" => {
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
                            "HashMap_new_String_i32" => {
                                extra_struct
                                    .entry(dst.0)
                                    .or_insert_with(|| "__hashmap_str_i32".to_string());
                            }
                            // get_unchecked on Vec<Struct> → result is a struct; get on Vec<String> → Option_String
                            "get_unchecked" | "get" => {
                                if let Some(first_arg) = args.first()
                                    && let Operand::Place(Place::Local(src)) = first_arg
                                {
                                    if let Some(sname) = self.struct_vec_locals.get(&src.0).cloned()
                                    {
                                        extra_struct.entry(dst.0).or_insert(sname);
                                    } else if canonical == "get"
                                        && (extra_vec_string.contains(&src.0)
                                            || self.string_vec_locals.contains(&src.0))
                                        && self.enum_base_types.contains_key("Option_String")
                                    {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Option_String".to_string());
                                    }
                                }
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
                Type::Char => {
                    self.char_locals.insert(local.id.0);
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
        if !func.type_params.is_empty()
            && let Some(param_names) = self.fn_param_type_names.get(&func.name).cloned()
        {
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
        // si(12): ref null $vec_string_ty scratch for emit_args_builtin
        let vec_string_ref = ref_nullable(self.vec_string_ty);
        local_types.push((1, vec_string_ref));

        let mut f = Function::new(local_types);

        // Build the set of Wasm local indices that hold GC references.
        // The local.tee peephole optimization is suppressed for these to avoid
        // a wasmtime DRC GC issue where the stack copy left by local.tee is not
        // registered in the VMGcRefActivationsTable.
        let gc_ref_locals: std::collections::HashSet<u32> = self
            .string_locals
            .iter()
            .copied()
            .chain(self.local_struct.keys().copied())
            .chain(self.local_enum.keys().copied())
            .chain(self.any_locals.iter().copied())
            .chain(self.i32_vec_locals.iter().copied())
            .chain(self.i64_vec_locals.iter().copied())
            .chain(self.f64_vec_locals.iter().copied())
            .chain(self.string_vec_locals.iter().copied())
            .chain(self.struct_vec_locals.keys().copied())
            // Scratch string/anyref locals
            .chain([
                self.scratch_base + 4,
                self.scratch_base + 5,
                self.scratch_base + 8,
                self.scratch_base + 10,
                self.scratch_base + 11,
                self.scratch_base + 12,
            ])
            .collect();

        // Wrap in PeepholeWriter for local.set/get → local.tee optimization
        let tee_count = {
            let mut w = PeepholeWriter::with_gc_ref_locals(&mut f, self.opt_level, gc_ref_locals);

            // Emit statements from entry block
            if let Some(block) = func.blocks.first() {
                let is_main_fn = func.name == "main" || func.name == "_start";
                let n_stmts = block.stmts.len();

                // ── Opportunistic TCO: let-call-return detection ──
                // Detect `let x = user_call(args…); return x` patterns where:
                //   • the last MIR statement binds a user function call to a local, AND
                //   • the block terminator returns that exact local.
                //
                // This covers non-desugared tail calls that the MIR-level
                // `detect_tail_calls` pass misses because it only rewrites
                // `Terminator::Return(Some(Operand::Call(…)))`, not the
                // statement-form binding `MirStmt::Call { dest: Local(id), … }` /
                // `MirStmt::Assign(Local(id), Rvalue::Use(Operand::Call(…)))` followed
                // by `Terminator::Return(Some(Operand::Place(Local(id))))`.
                //
                // Handles two last-stmt shapes:
                //   1. MirStmt::Call { dest: Some(Local(id)), func: FnId, args }
                //   2. MirStmt::Assign(Local(id), Rvalue::Use(Operand::Call(name, args)))
                let opp_tco_candidate = !is_main_fn && self.opt_level >= 1 && n_stmts > 0 && {
                    let last = &block.stmts[n_stmts - 1];
                    let ret_local = match &block.terminator {
                        Terminator::Return(Some(Operand::Place(Place::Local(id)))) => Some(id.0),
                        _ => None,
                    };
                    if let Some(ret_id) = ret_local {
                        match last {
                            MirStmt::Call {
                                dest: Some(Place::Local(dest_id)),
                                ..
                            } => dest_id.0 == ret_id,
                            MirStmt::Assign(
                                Place::Local(dest_id),
                                Rvalue::Use(Operand::Call(_, _)),
                            ) => dest_id.0 == ret_id,
                            _ => false,
                        }
                    } else {
                        false
                    }
                };

                // Determine how many statements to emit in the main loop.
                // If opportunistic TCO is a candidate, hold back the last statement
                // and attempt tail-call emission after the loop.
                let emit_up_to = if opp_tco_candidate {
                    n_stmts - 1
                } else {
                    n_stmts
                };
                for stmt in &block.stmts[..emit_up_to] {
                    self.emit_stmt(&mut w, stmt);
                }

                // Emit the last statement as a tail call (opportunistic path)
                // or fall through to the normal terminator handler.
                let opp_tco_fired = if opp_tco_candidate {
                    if let Terminator::Return(Some(return_op)) = &block.terminator {
                        self.try_emit_let_call_tail_return(
                            &mut w,
                            &block.stmts[n_stmts - 1],
                            return_op,
                        )
                    } else {
                        false
                    }
                } else {
                    false
                };

                // If opportunistic TCO did NOT fire, emit the held-back statement now.
                if opp_tco_candidate && !opp_tco_fired {
                    self.emit_stmt(&mut w, &block.stmts[n_stmts - 1]);
                }

                // Handle terminator (skipped when opportunistic TCO fired)
                if !opp_tco_fired {
                    match &block.terminator {
                        Terminator::Return(Some(op)) => {
                            if is_main_fn {
                                // WASI _start must be () -> (); emit for side effects but discard result
                                if !matches!(op, Operand::Unit) {
                                    self.emit_operand(&mut w, op);
                                    if self.operand_produces_value(op) {
                                        w.instruction(&Instruction::Drop);
                                    }
                                }
                            } else if self.opt_level >= 1
                                && self.try_emit_tail_call_return(&mut w, op)
                            {
                                // return_call (or return_call via IfExpr branch) was emitted;
                                // no Return instruction needed.
                            } else {
                                self.emit_operand(&mut w, op);
                                // Box value types when returning from generic function with anyref return
                                // i31ref unboxed scalar optimization (issue #070)
                                if self.current_fn_return_ty == Type::Any {
                                    let op_vt = self.infer_operand_type(op);
                                    if op_vt == ValType::I32 {
                                        i31::emit_bool_to_anyref(&mut w);
                                    }
                                }
                                w.instruction(&Instruction::Return);
                            }
                        }
                        Terminator::Return(None) => {
                            w.instruction(&Instruction::Return);
                        }
                        Terminator::TailCall {
                            func: callee_name,
                            args,
                        } => {
                            // Emit `return_call <func>` — tail-call terminator from MIR optimiser.
                            let canonical = normalize_intrinsic(callee_name).to_string();
                            if let Some(&fn_idx) = self.fn_map.get(canonical.as_str()) {
                                let param_types =
                                    self.fn_param_types.get(canonical.as_str()).cloned();
                                for (i, arg) in args.iter().enumerate() {
                                    self.emit_operand(&mut w, arg);
                                    if let Some(ref pts) = param_types
                                        && i < pts.len()
                                        && pts[i] == Type::Any
                                    {
                                        let arg_vt = self.infer_operand_type(arg);
                                        if arg_vt == ValType::I32 {
                                            // i31ref unboxed scalar boxing (issue #070)
                                            i31::emit_bool_to_anyref(&mut w);
                                        }
                                    }
                                }
                                w.instruction(&Instruction::ReturnCall(fn_idx));
                            } else {
                                // Fallback: emit as regular call + return
                                for arg in args {
                                    self.emit_operand(&mut w, arg);
                                }
                                if let Some(&fn_idx) = self.fn_map.get(canonical.as_str()) {
                                    w.instruction(&Instruction::Call(fn_idx));
                                }
                                w.instruction(&Instruction::Return);
                            }
                        }
                        Terminator::TailCallIndirect { callee, args } => {
                            // If the callee is a typed GC function reference (FnRef directly
                            // or a local whose assigned value was tracked in fn_ref_locals),
                            // emit `ref.func $fn + return_call_ref $type` instead of the
                            // table-based `return_call_indirect`.  This is the correct
                            // lowering for the Wasm function-references / GC proposal.
                            let fn_name_opt: Option<String> = match callee.as_ref() {
                                Operand::FnRef(name) => Some(name.clone()),
                                Operand::Place(Place::Local(local_id)) => {
                                    self.fn_ref_locals.get(&local_id.0).cloned()
                                }
                                _ => None,
                            };
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
                            if let Some(fn_name) = fn_name_opt
                                && let Some(&fn_idx) = self.fn_map.get(fn_name.as_str())
                            {
                                // return_call_ref: push args, then funcref, then return_call_ref.
                                for arg in args {
                                    self.emit_operand(&mut w, arg);
                                }
                                w.instruction(&Instruction::RefFunc(fn_idx));
                                w.instruction(&Instruction::ReturnCallRef(type_index));
                            } else {
                                // Fallback: table-based return_call_indirect.
                                for arg in args {
                                    self.emit_operand(&mut w, arg);
                                }
                                self.emit_operand(&mut w, callee);
                                w.instruction(&Instruction::ReturnCallIndirect {
                                    type_index,
                                    table_index: 0,
                                });
                            }
                        }
                        _ => {}
                    }
                } // end if !opp_tco_fired
            }
            w.instruction(&Instruction::End);
            w.flush();
            (w.tee_count(), w.rmw_count())
        };
        let (tee_count, rmw_count) = tee_count;
        if tee_count > 0 {
            // Peephole applied: `tee_count` local.set/get pairs → local.tee
            let _ = tee_count;
        }
        if rmw_count > 0 {
            // RMW peephole applied: `rmw_count` struct.get/struct.set no-op
            // sequences eliminated (issue #156).
            let _ = rmw_count;
        }
        codes.function(&f);
    }

    /// Emit an operand that is in result position within an IfExpr branch that is
    /// itself in tail position (i.e., `Terminator::Return(Some(IfExpr))`).
    ///
    /// For `Call` operands at opt_level ≥ 1: emit `return_call` (tail call).
    /// For all others: emit normally (pushes result value on stack for the block).
    fn emit_operand_try_tco(&mut self, f: &mut PeepholeWriter<'_>, op: &Operand) {
        if self.opt_level >= 1
            && let Operand::Call(name, args) = op
        {
            let canonical = normalize_intrinsic(name).to_string();
            if !self.is_builtin_name(&canonical) {
                let callee_ret_is_any = self
                    .fn_ret_types
                    .get(canonical.as_str())
                    .is_some_and(|t| *t == Type::Any);
                let current_ret_is_any = self.current_fn_return_ty == Type::Any;
                if callee_ret_is_any == current_ret_is_any
                    && let Some(&fn_idx) = self.fn_map.get(canonical.as_str())
                {
                    let param_types = self.fn_param_types.get(canonical.as_str()).cloned();
                    for (i, arg) in args.iter().enumerate() {
                        self.emit_operand(f, arg);
                        if let Some(ref pts) = param_types
                            && i < pts.len()
                            && pts[i] == Type::Any
                        {
                            let arg_vt = self.infer_operand_type(arg);
                            if arg_vt == ValType::I32 {
                                // i31ref unboxed scalar boxing (issue #070)
                                i31::emit_bool_to_anyref(f);
                            }
                        }
                    }
                    f.instruction(&Instruction::ReturnCall(fn_idx));
                    return;
                }
            }
        }
        // Fallback: emit normally (leaves value on stack for block result)
        self.emit_operand(f, op);
    }

    /// Try to emit a tail-call return for operand `op` when in tail position.
    ///
    /// Returns `true` if a `return_call` (or equivalent) was emitted and the
    /// caller must NOT emit an additional `return` instruction.
    pub(super) fn try_emit_tail_call_return(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        op: &Operand,
    ) -> bool {
        match op {
            // ── Direct tail call ──
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic(name).to_string();
                if self.is_builtin_name(&canonical) {
                    return false;
                }
                let callee_ret_is_any = self
                    .fn_ret_types
                    .get(canonical.as_str())
                    .is_some_and(|t| *t == Type::Any);
                let current_ret_is_any = self.current_fn_return_ty == Type::Any;
                if callee_ret_is_any != current_ret_is_any {
                    return false;
                }
                if let Some(&fn_idx) = self.fn_map.get(canonical.as_str()) {
                    let param_types = self.fn_param_types.get(canonical.as_str()).cloned();
                    for (i, arg) in args.iter().enumerate() {
                        self.emit_operand(f, arg);
                        if let Some(ref pts) = param_types
                            && i < pts.len()
                            && pts[i] == Type::Any
                        {
                            let arg_vt = self.infer_operand_type(arg);
                            if arg_vt == ValType::I32 {
                                // i31ref unboxed scalar boxing (issue #070)
                                i31::emit_bool_to_anyref(f);
                            }
                        }
                    }
                    f.instruction(&Instruction::ReturnCall(fn_idx));
                    return true;
                }
                false
            }
            // ── Indirect tail call ──
            Operand::CallIndirect { callee, args } => {
                if self.current_fn_return_ty == Type::Any {
                    return false; // boxing/unboxing required; skip TCO
                }
                // If the callee is a local that was assigned a FnRef, or is a FnRef
                // directly, emit return_call_ref instead of return_call_indirect.
                let fn_name_opt: Option<String> = match callee.as_ref() {
                    Operand::FnRef(name) => Some(name.clone()),
                    Operand::Place(Place::Local(local_id)) => {
                        self.fn_ref_locals.get(&local_id.0).cloned()
                    }
                    _ => None,
                };
                if let Some(fn_name) = fn_name_opt {
                    if let Some(&fn_idx) = self.fn_map.get(fn_name.as_str()) {
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
                        for arg in args.iter() {
                            self.emit_operand(f, arg);
                        }
                        f.instruction(&Instruction::RefFunc(fn_idx));
                        f.instruction(&Instruction::ReturnCallRef(type_index));
                        return true;
                    }
                }
                // Fallback: table-based indirect tail call.
                for arg in args.iter() {
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
                true
            }
            // ── IfExpr with Call results in tail position ──
            // Emit as `if (result_type) { ... } else { return_call }`.
            // The `return_call` in the tail-call branch is polymorphic on the result
            // side, so it satisfies the block's result type and exits the function
            // directly.  Non-call branches just push their result value for the block.
            // The outer `return` instruction (emitted by the `both_tail = false` path)
            // is only reachable from non-tail branches.
            //
            // Returns `false` always: the outer caller must emit `return`.
            Operand::IfExpr {
                cond,
                then_body,
                then_result,
                else_body,
                else_result,
            } => {
                let then_is_tail_call = matches!(
                    then_result.as_deref(),
                    Some(Operand::Call(_, _)) | Some(Operand::CallIndirect { .. })
                );
                let else_is_tail_call = matches!(
                    else_result.as_deref(),
                    Some(Operand::Call(_, _)) | Some(Operand::CallIndirect { .. })
                );
                if !then_is_tail_call && !else_is_tail_call {
                    return false;
                }

                // Determine the block result type from the non-call branch.
                let result_vt = {
                    let check = if !then_is_tail_call {
                        then_result.as_deref()
                    } else {
                        else_result.as_deref()
                    };
                    check
                        .map(|r| self.infer_operand_type(r))
                        .unwrap_or(ValType::I32)
                };

                // Emit condition + if block with function result type.
                // `return_call` is polymorphic so it satisfies `BlockType::Result(vt)`.
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(result_vt)));
                self.loop_break_extra_depth += 1;

                // Emit then-branch statements
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                // Emit then-result: tail call → return_call; otherwise push value
                if let Some(r) = then_result.as_deref() {
                    // Try as tail call (return_call), otherwise emit as value
                    self.emit_operand_try_tco(f, r);
                }

                f.instruction(&Instruction::Else);

                // Emit else-branch statements
                for s in else_body {
                    self.emit_stmt(f, s);
                }
                // Emit else-result: tail call → return_call; otherwise push value
                if let Some(r) = else_result.as_deref() {
                    self.emit_operand_try_tco(f, r);
                }

                self.loop_break_extra_depth -= 1;
                f.instruction(&Instruction::End);

                // Return false: the outer code MUST emit `return` for the if-block result.
                // The `return_call` branches already exited the function; the `return`
                // only applies to the non-tail-call branch.
                false
            }
            _ => false,
        }
    }

    /// Opportunistic TCO: detect and emit a `let result = user_call(args); return result`
    /// pattern that the MIR-level tail-call detection pass missed.
    ///
    /// Handles two last-statement shapes:
    ///   1. `MirStmt::Call { dest: Some(Place::Local(dest_id)), func: FnId, args }`
    ///   2. `MirStmt::Assign(Place::Local(dest_id), Rvalue::Use(Operand::Call(name, args)))`
    ///
    /// In both cases the `return_op` must be `Operand::Place(Place::Local(dest_id))`.
    ///
    /// Returns `true` if `return_call` was emitted.  The caller must NOT emit the
    /// last statement again or any `return` instruction when `true` is returned.
    pub(super) fn try_emit_let_call_tail_return(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        last_stmt: &MirStmt,
        return_op: &Operand,
    ) -> bool {
        // Confirm the return operand is exactly a plain local read.
        let ret_local_id = match return_op {
            Operand::Place(Place::Local(id)) => id.0,
            _ => return false,
        };

        match last_stmt {
            // ── Shape 1: MirStmt::Call { dest: Some(Local(id)), func: FnId, args } ──
            MirStmt::Call {
                dest: Some(Place::Local(dest_id)),
                func: fn_id,
                args,
            } if dest_id.0 == ret_local_id => {
                let fn_idx_mir = fn_id.0 as usize;
                let fn_name = match self.fn_names.get(fn_idx_mir).cloned() {
                    Some(n) => n,
                    None => return false,
                };
                let canonical = normalize_intrinsic(&fn_name).to_string();
                let lookup_name = fn_name.rsplit("::").next().unwrap_or(&fn_name).to_string();
                let is_lookup_builtin = self.is_builtin_name(&lookup_name);
                let prefer_user_fn = fn_name.contains("::") && !is_lookup_builtin;
                let is_http_wrapper = self.http_wrapper_fns.contains(&fn_name);
                // Skip functions that would be dispatched via the builtin path in emit_stmt.
                if (self.is_builtin_name(&canonical) || is_lookup_builtin)
                    && !prefer_user_fn
                    && !is_http_wrapper
                {
                    return false;
                }
                // Resolve to Wasm function index (same lookup order as stmts.rs).
                let (effective_name, fn_idx) = if let Some(&idx) = self.fn_map.get(&fn_name) {
                    (fn_name.clone(), idx)
                } else if let Some(&idx) = self.fn_map.get(lookup_name.as_str()) {
                    (lookup_name.clone(), idx)
                } else {
                    return false;
                };
                // Type-compatibility check (same as try_emit_tail_call_return).
                let callee_ret_is_any = self
                    .fn_ret_types
                    .get(effective_name.as_str())
                    .is_some_and(|t| *t == Type::Any);
                if callee_ret_is_any != (self.current_fn_return_ty == Type::Any) {
                    return false;
                }
                // Emit args with type coercion (mirrors stmts.rs MirStmt::Call emission).
                let param_types = self.fn_param_types.get(effective_name.as_str()).cloned();
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
                    if need_any {
                        let arg_vt = self.infer_operand_type(arg);
                        if arg_vt == ValType::I32 {
                            // i31ref unboxed scalar boxing (issue #070)
                            i31::emit_bool_to_anyref(f);
                        }
                    }
                }
                f.instruction(&Instruction::ReturnCall(fn_idx));
                true
            }

            // ── Shape 2: Assign(Local(id), Use(Operand::Call(name, args))) ──
            MirStmt::Assign(Place::Local(dest_id), Rvalue::Use(Operand::Call(name, args)))
                if dest_id.0 == ret_local_id =>
            {
                let canonical = normalize_intrinsic(name).to_string();
                if self.is_builtin_name(&canonical) {
                    return false;
                }
                // Type-compatibility check.
                let callee_ret_is_any = self
                    .fn_ret_types
                    .get(canonical.as_str())
                    .is_some_and(|t| *t == Type::Any);
                if callee_ret_is_any != (self.current_fn_return_ty == Type::Any) {
                    return false;
                }
                let fn_idx = match self.fn_map.get(canonical.as_str()).copied() {
                    Some(i) => i,
                    None => return false,
                };
                // Emit args with anyref boxing (mirrors try_emit_tail_call_return).
                let param_types = self.fn_param_types.get(canonical.as_str()).cloned();
                for (i, arg) in args.iter().enumerate() {
                    self.emit_operand(f, arg);
                    if let Some(ref pts) = param_types
                        && i < pts.len()
                        && pts[i] == Type::Any
                    {
                        let arg_vt = self.infer_operand_type(arg);
                        if arg_vt == ValType::I32 {
                            // i31ref unboxed scalar boxing (issue #070)
                            i31::emit_bool_to_anyref(f);
                        }
                    }
                }
                f.instruction(&Instruction::ReturnCall(fn_idx));
                true
            }

            _ => false,
        }
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use ark_diagnostics::DiagnosticSink;
    use ark_mir::mir::{
        BasicBlock, BinOp, BlockId, FnId, InstanceKey, LocalId, MirFnSig, MirFunction, MirLocal,
        MirModule, MirStmt, Operand, Place, Rvalue, SourceInfo, Terminator,
    };
    use ark_typecheck::types::Type;
    use wasmparser::{BlockType, Operator, Parser, Payload};

    fn make_simple_func(id: u32, name: &str, ret_ty: Type, terminator: Terminator) -> MirFunction {
        MirFunction {
            id: FnId(id),
            name: name.to_string(),
            instance: InstanceKey::simple(name),
            params: Vec::new(),
            return_ty: ret_ty,
            locals: Vec::new(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: Vec::new(),
                terminator,
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: Vec::new(),
            source: SourceInfo::unknown(),
            is_exported: false,
        }
    }

    fn count_br_tables(wasm: &[u8]) -> usize {
        let mut count = 0;
        for payload in Parser::new(0).parse_all(wasm) {
            let Ok(payload) = payload else {
                continue;
            };
            if let Payload::CodeSectionEntry(body) = payload {
                let mut reader = body.get_operators_reader().expect("operators reader");
                while !reader.eof() {
                    if matches!(reader.read().expect("operator"), Operator::BrTable { .. }) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn make_enum_dispatch_module(mismatch_last_arm: bool) -> MirModule {
        let mut mir = MirModule::new();
        let enum_variants = vec![
            ("Red".to_string(), vec![]),
            ("Yellow".to_string(), vec![]),
            ("Green".to_string(), vec![]),
        ];
        mir.enum_defs
            .insert("Traffic".to_string(), enum_variants.clone());
        mir.type_table
            .enum_defs
            .insert("Traffic".to_string(), enum_variants);

        let branch_body = |value: i32| {
            vec![MirStmt::Assign(
                Place::Local(LocalId(1)),
                Rvalue::Use(Operand::ConstI32(value)),
            )]
        };

        let last_cond = if mismatch_last_arm {
            Operand::BinOp(
                BinOp::Eq,
                Box::new(Operand::EnumTag(Box::new(Operand::Place(Place::Local(
                    LocalId(0),
                ))))),
                Box::new(Operand::ConstI32(2)),
            )
        } else {
            Operand::BinOp(
                BinOp::Eq,
                Box::new(Operand::EnumTag(Box::new(Operand::Place(Place::Local(
                    LocalId(0),
                ))))),
                Box::new(Operand::ConstI32(1)),
            )
        };

        let stmts = vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::EnumInit {
                    enum_name: "Traffic".to_string(),
                    variant: "Yellow".to_string(),
                    tag: 1,
                    payload: vec![],
                }),
            ),
            MirStmt::IfStmt {
                cond: Operand::BinOp(
                    BinOp::Eq,
                    Box::new(Operand::EnumTag(Box::new(Operand::Place(Place::Local(
                        LocalId(0),
                    ))))),
                    Box::new(Operand::ConstI32(2)),
                ),
                then_body: branch_body(30),
                else_body: vec![MirStmt::IfStmt {
                    cond: Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(Operand::Place(Place::Local(
                            LocalId(0),
                        ))))),
                        Box::new(Operand::ConstI32(0)),
                    ),
                    then_body: branch_body(10),
                    else_body: vec![MirStmt::IfStmt {
                        cond: last_cond,
                        then_body: branch_body(20),
                        else_body: vec![],
                    }],
                }],
            },
        ];

        mir.functions.push(MirFunction {
            id: FnId(0),
            name: "dispatch".to_string(),
            instance: InstanceKey::simple("dispatch"),
            params: Vec::new(),
            return_ty: Type::I32,
            locals: vec![
                MirLocal {
                    id: LocalId(0),
                    name: Some("signal".to_string()),
                    ty: Type::Any,
                },
                MirLocal {
                    id: LocalId(1),
                    name: Some("result".to_string()),
                    ty: Type::I32,
                },
            ],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts,
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(1))))),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: [(0, "Traffic".to_string())].into_iter().collect(),
            type_params: Vec::new(),
            source: SourceInfo::unknown(),
            is_exported: false,
        });
        mir.entry_fn = Some(FnId(0));
        mir
    }

    /// Verify that `Terminator::TailCallIndirect { callee: FnRef(name), args }` causes the
    /// T3 emitter to produce a `return_call_ref` instruction (opcode 0x15) instead of
    /// `return_call_indirect` (opcode 0x13) when the callee is a typed GC function reference.
    #[test]
    fn tail_call_indirect_fn_ref_emits_return_call_ref() {
        // Build a two-function module:
        //   target_fn() -> i32   (just returns 0)
        //   caller_fn() -> i32   (TailCallIndirect via FnRef("target_fn"))
        // caller_fn is the module entry so both functions are reachable.
        let mut mir = MirModule::new();
        // target_fn at vector index 0 → use FnId(0)
        mir.functions.push(make_simple_func(
            0,
            "target_fn",
            Type::I32,
            Terminator::Return(Some(Operand::ConstI32(0))),
        ));
        // caller_fn at vector index 1 → use FnId(1)
        mir.functions.push(make_simple_func(
            1,
            "caller_fn",
            Type::I32,
            Terminator::TailCallIndirect {
                callee: Box::new(Operand::FnRef("target_fn".into())),
                args: Vec::new(),
            },
        ));
        // entry_fn index 1 → caller_fn is the module root; reachability will
        // follow FnRef("target_fn") and include target_fn as well.
        mir.entry_fn = Some(FnId(1));

        let mut sink = DiagnosticSink::new();
        let wasm = super::super::emit(&mir, &mut sink, 1, true);

        // return_call_ref uses opcode 0x15.
        assert!(
            wasm.contains(&0x15),
            "expected return_call_ref (0x15) in emitted wasm for TailCallIndirect+FnRef"
        );
        // Confirm return_call_indirect (0x13) was NOT used for this call.
        // (0x15 must appear and 0x13 must not appear from the fn-ref path;
        //  the exact count depends on helper function count, so we just
        //  check that the fn-ref path did not produce 0x13.)
        // NB: It is acceptable for 0x13 to appear in stdlib helpers, but
        //     our two-function-only module has no helpers, so 0x13 = 0.
        assert!(
            !wasm.contains(&0x13),
            "return_call_indirect (0x13) should not appear when callee is a FnRef"
        );
    }

    /// Verify that identical string literals appearing twice in the same module are
    /// deduplicated to a single passive data segment in the emitted Wasm binary.
    ///
    /// At opt_level >= 1 the T3 emitter uses `alloc_string_data` with a cache
    /// (`string_seg_cache`) so two occurrences of `"intern_dedup"` produce exactly
    /// one passive data segment containing those bytes.
    #[test]
    fn same_string_literal_deduplicates_data_segments() {
        use ark_mir::mir::{
            BasicBlock, BlockId, FnId, InstanceKey, MirFunction, MirModule, MirStmt, Operand,
            SourceInfo, Terminator,
        };
        use ark_typecheck::types::Type;

        // Build a minimal module: one function that uses "intern_dedup" twice.
        // Two CallBuiltin("println") statements with the same ConstString operand.
        let func = MirFunction {
            id: FnId(0),
            name: "main".to_string(),
            instance: InstanceKey::simple("main"),
            params: Vec::new(),
            return_ty: Type::I32,
            locals: Vec::new(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![
                    MirStmt::CallBuiltin {
                        dest: None,
                        name: "println".to_string(),
                        args: vec![Operand::ConstString("intern_dedup".to_string())],
                    },
                    MirStmt::CallBuiltin {
                        dest: None,
                        name: "println".to_string(),
                        args: vec![Operand::ConstString("intern_dedup".to_string())],
                    },
                ],
                terminator: Terminator::Return(Some(Operand::ConstI32(0))),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: Vec::new(),
            source: SourceInfo::unknown(),
            is_exported: false,
        };

        let mut mir = MirModule::new();
        mir.functions.push(func);
        mir.entry_fn = Some(FnId(0));

        let mut sink = DiagnosticSink::new();
        // opt_level = 1: string interning + data-segment deduplication enabled.
        let wasm = super::super::emit(&mir, &mut sink, 1, true);

        // Count how many times the UTF-8 bytes of "intern_dedup" appear in the
        // emitted binary.  If deduplication works, the string bytes appear exactly
        // once in the data section even though the source uses the literal twice.
        let needle = b"intern_dedup";
        let occurrences = wasm.windows(needle.len()).filter(|w| *w == needle).count();

        assert_eq!(
            occurrences, 1,
            "expected 'intern_dedup' bytes to appear exactly once in wasm binary \
             (data-segment deduplication), but found {} occurrence(s)",
            occurrences
        );
    }

    #[test]
    fn tuple_ifexpr_uses_multivalue_block_and_single_struct_new() {
        let mut mir = MirModule::new();
        let tuple_layout = vec![
            ("0".to_string(), "i32".to_string()),
            ("1".to_string(), "i32".to_string()),
        ];
        mir.struct_defs
            .insert("__tuple2".to_string(), tuple_layout.clone());
        mir.type_table
            .struct_defs
            .insert("__tuple2".to_string(), tuple_layout);
        mir.type_table.fn_sigs.insert(
            "pick_pair".to_string(),
            MirFnSig {
                name: "pick_pair".to_string(),
                params: Vec::new(),
                ret: "(i32, i32)".to_string(),
            },
        );

        mir.functions.push(make_simple_func(
            0,
            "pick_pair",
            Type::I32,
            Terminator::Return(Some(Operand::IfExpr {
                cond: Box::new(Operand::ConstBool(true)),
                then_body: Vec::new(),
                then_result: Some(Box::new(Operand::StructInit {
                    name: "__tuple2".to_string(),
                    fields: vec![
                        ("0".to_string(), Operand::ConstI32(10)),
                        ("1".to_string(), Operand::ConstI32(20)),
                    ],
                })),
                else_body: Vec::new(),
                else_result: Some(Box::new(Operand::StructInit {
                    name: "__tuple2".to_string(),
                    fields: vec![
                        ("0".to_string(), Operand::ConstI32(30)),
                        ("1".to_string(), Operand::ConstI32(40)),
                    ],
                })),
            })),
        ));
        mir.entry_fn = Some(FnId(0));

        let mut sink = DiagnosticSink::new();
        let wasm = super::super::emit(&mir, &mut sink, 1, true);

        let mut saw_multivalue_if = false;
        let mut struct_new_count = 0usize;
        for payload in Parser::new(0).parse_all(&wasm) {
            let Ok(payload) = payload else {
                continue;
            };
            if let Payload::CodeSectionEntry(body) = payload {
                let mut reader = body.get_operators_reader().expect("operators reader");
                while !reader.eof() {
                    match reader.read().expect("operator") {
                        Operator::If { blockty } => {
                            if matches!(blockty, BlockType::FuncType(_)) {
                                saw_multivalue_if = true;
                            }
                        }
                        Operator::StructNew { .. } => {
                            struct_new_count += 1;
                        }
                        _ => {}
                    }
                }
            }
        }

        assert!(
            saw_multivalue_if,
            "expected tuple-valued if-expression to use a function-typed multi-value block"
        );
        assert_eq!(
            struct_new_count, 1,
            "expected tuple-valued if-expression to materialize the tuple only once after the block"
        );
    }

    #[test]
    fn enum_if_chain_emits_br_table_at_o1() {
        let mir = make_enum_dispatch_module(false);
        let mut sink = DiagnosticSink::new();
        let wasm = super::super::emit(&mir, &mut sink, 1, true);

        assert_eq!(
            count_br_tables(&wasm),
            1,
            "expected exactly one br_table for a 3-arm linear enum dispatch at opt_level=1"
        );
    }

    #[test]
    fn enum_if_chain_falls_back_when_pattern_does_not_match() {
        let mir = make_enum_dispatch_module(true);
        let mut sink = DiagnosticSink::new();
        let wasm = super::super::emit(&mir, &mut sink, 1, true);

        assert_eq!(
            count_br_tables(&wasm),
            0,
            "non-matching enum IfStmt chains must keep the existing if/else lowering"
        );
    }

    /// `metadata.code.branch_hint` must contain at least one recorded hint when
    /// `IfStmt` lowers to Wasm `if` and the panic-path heuristic applies.
    #[test]
    fn branch_hint_custom_section_non_empty_for_ifstmt_panic_heuristic() {
        use ark_mir::mir::{
            BasicBlock, BlockId, FnId, InstanceKey, MirFunction, MirLocal, MirModule, MirStmt,
            Operand, Place, Rvalue, SourceInfo, Terminator,
        };
        use ark_typecheck::types::Type;
        use wasmparser::{KnownCustom, Operator, Parser, Payload};

        let func = MirFunction {
            id: FnId(0),
            name: "main".to_string(),
            instance: InstanceKey::simple("main"),
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: None,
                ty: Type::Bool,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![
                    MirStmt::Assign(
                        Place::Local(LocalId(0)),
                        Rvalue::Use(Operand::ConstBool(true)),
                    ),
                    MirStmt::IfStmt {
                        cond: Operand::Place(Place::Local(LocalId(0))),
                        then_body: vec![MirStmt::CallBuiltin {
                            dest: None,
                            name: "panic".into(),
                            args: vec![],
                        }],
                        else_body: vec![MirStmt::Return(None)],
                    },
                ],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: SourceInfo::unknown(),
            is_exported: false,
        };

        let mut mir = MirModule::new();
        mir.functions.push(func);
        mir.entry_fn = Some(FnId(0));

        let mut sink = DiagnosticSink::new();
        let wasm = super::super::emit(&mir, &mut sink, 0, true);
        assert!(!wasm.is_empty());

        let mut hint_payload_len = 0usize;
        let mut parsed_taken: Option<bool> = None;
        for payload in Parser::new(0).parse_all(&wasm) {
            let Ok(payload) = payload else {
                continue;
            };
            if let Payload::CustomSection(c) = payload {
                if c.name() == "metadata.code.branch_hint" {
                    hint_payload_len = c.data().len();
                    if let KnownCustom::BranchHints(reader) = c.as_known() {
                        let fh = reader
                            .into_iter()
                            .next()
                            .expect("branch hint section")
                            .expect("function hint group");
                        let hint = fh
                            .hints
                            .into_iter()
                            .next()
                            .expect("hint list")
                            .expect("branch hint entry");
                        parsed_taken = Some(hint.taken);
                    }
                }
            }
        }

        assert!(
            hint_payload_len > 1,
            "expected non-empty branch_hint custom section payload"
        );
        assert_eq!(
            parsed_taken,
            Some(false),
            "then-panic path should be hinted as unlikely (not taken)"
        );

        let mut if_op_count = 0usize;
        for payload in Parser::new(0).parse_all(&wasm) {
            let Ok(payload) = payload else {
                continue;
            };
            if let Payload::CodeSectionEntry(body) = payload {
                let mut reader = body.get_operators_reader().expect("operators reader");
                while !reader.eof() {
                    if matches!(
                        reader.read().expect("operator"),
                        Operator::If { .. }
                    ) {
                        if_op_count += 1;
                    }
                }
            }
        }
        assert!(
            if_op_count >= 1,
            "expected at least one Wasm `if` (control instruction) in emitted code"
        );
    }

    /// Issue #070: `bool` (as i32) flowing through a generic `T` / `Any` parameter is boxed with
    /// `ref.i31` and unboxed with `i31.get_s` — no GC heap allocation for that scalar path.
    #[test]
    fn generic_identity_bool_emits_ref_i31_and_i31_get_s() {
        let identity = MirFunction {
            id: FnId(0),
            name: "identity".to_string(),
            instance: InstanceKey::simple("identity"),
            params: vec![MirLocal {
                id: LocalId(0),
                name: Some("x".into()),
                ty: Type::Any,
            }],
            return_ty: Type::Any,
            locals: Vec::new(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: Vec::new(),
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec!["T".into()],
            source: SourceInfo::unknown(),
            is_exported: false,
        };

        let main_fn = MirFunction {
            id: FnId(1),
            name: "main".to_string(),
            instance: InstanceKey::simple("main"),
            params: Vec::new(),
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: None,
                ty: Type::Bool,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Call {
                    dest: Some(Place::Local(LocalId(0))),
                    func: FnId(0),
                    args: vec![Operand::ConstBool(true)],
                }],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: Vec::new(),
            source: SourceInfo::unknown(),
            is_exported: false,
        };

        let mut mir = MirModule::new();
        mir.functions.push(identity);
        mir.functions.push(main_fn);
        mir.entry_fn = Some(FnId(1));

        let mut sink = DiagnosticSink::new();
        // O2 is the planned “bool as i31ref local” tier; boxing/unboxing must still work here.
        let wasm = super::super::emit(&mir, &mut sink, 2, true);

        let mut ref_i31 = 0usize;
        let mut i31_get_s = 0usize;
        for payload in Parser::new(0).parse_all(&wasm) {
            let Ok(payload) = payload else {
                continue;
            };
            if let Payload::CodeSectionEntry(body) = payload {
                let mut reader = body.get_operators_reader().expect("operators reader");
                while !reader.eof() {
                    match reader.read().expect("operator") {
                        Operator::RefI31 => ref_i31 += 1,
                        Operator::I31GetS => i31_get_s += 1,
                        _ => {}
                    }
                }
            }
        }

        assert!(
            ref_i31 >= 1 && i31_get_s >= 1,
            "expected ref.i31 (box) and i31.get_s (unbox) for bool through Any generic; \
             got ref_i31={ref_i31}, i31_get_s={i31_get_s}"
        );
    }
}
