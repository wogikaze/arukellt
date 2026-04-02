//! Stdlib/builtin operation emission for the T3 Wasm GC emitter.
//!
//! String GC operations, higher-order function emission (map/filter/fold),
//! vector operations, hashmap operations, and filesystem operations.

use ark_mir::mir::*;
use wasm_encoder::{HeapType, Instruction, MemArg, ValType};

use super::peephole::PeepholeWriter;
use super::{Ctx, ref_nullable};
use super::{
    FS_BUF_SIZE, FS_SCRATCH, SCR_A_LEN, SCR_A_PTR, SCR_B_PTR, SCR_DST_PTR, SCR_I, SCR_J, SCR_VAL64,
    SCRATCH,
};

impl Ctx {
    pub(super) fn emit_concat_gc(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::ArrayNew(self.string_ty));
            return;
        }
        let s0 = self.si(4); // a_ref
        let s1 = self.si(5); // b_ref
        let s_result = self.si(8); // result_ref (3rd ref scratch)
        let len_a = self.si(0);
        let len_b = self.si(1);
        let sty = self.string_ty;

        // Evaluate both operands onto the Wasm stack first, then pop into scratch
        // locals. This prevents inner (nested) concat calls from clobbering s0/s1
        // before the outer call has a chance to read them.
        self.emit_operand(f, &args[0]); // stack: [ref_a]
        self.emit_operand(f, &args[1]); // stack: [ref_a, ref_b]
        f.instruction(&Instruction::LocalSet(s1)); // pop ref_b
        f.instruction(&Instruction::LocalSet(s0)); // pop ref_a

        // Get lengths
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len_a));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len_b));

        // Create result array of len_a + len_b, store in s_result
        f.instruction(&Instruction::I32Const(0)); // fill value
        f.instruction(&Instruction::LocalGet(len_a));
        f.instruction(&Instruction::LocalGet(len_b));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalSet(s_result));

        // array.copy result[0..len_a] = a[0..len_a]
        f.instruction(&Instruction::LocalGet(s_result));
        f.instruction(&Instruction::I32Const(0)); // dst_off
        f.instruction(&Instruction::LocalGet(s0)); // src
        f.instruction(&Instruction::I32Const(0)); // src_off
        f.instruction(&Instruction::LocalGet(len_a)); // len
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });

        // array.copy result[len_a..] = b[0..len_b]
        f.instruction(&Instruction::LocalGet(s_result));
        f.instruction(&Instruction::LocalGet(len_a)); // dst_off
        f.instruction(&Instruction::LocalGet(s1)); // src
        f.instruction(&Instruction::I32Const(0)); // src_off
        f.instruction(&Instruction::LocalGet(len_b)); // len
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });

        // Push result
        f.instruction(&Instruction::LocalGet(s_result));
    }

    /// split(s, delim) → Vec<String>
    /// Splits `s` on occurrences of `delim`.  Empty delimiter pushes `s` as
    /// the sole element.  Scratch locals: si(4)=s, si(5)=delim, si(12)=result
    /// vec, si(8)=segment, si(0)=s_len, si(1)=delim_len, si(2)=i, si(3)=j/tmp,
    /// si(9)=start.
    pub(super) fn emit_split_gc(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let s_len = self.si(0);
        let delim_len = self.si(1);
        let i = self.si(2);
        let j = self.si(3);
        let start = self.si(9);
        let seg_ref = self.si(8);
        let vec_ref = self.si(12);
        let sty = self.string_ty;
        let vty = self.vec_string_ty;
        let aty = self.arr_string_ty;

        // Load s and delim
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(s0));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(s1));

        // Compute lengths
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(s_len));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(delim_len));

        // Allocate result vec
        f.instruction(&Instruction::RefNull(HeapType::Concrete(sty)));
        f.instruction(&Instruction::I32Const(16384));
        f.instruction(&Instruction::ArrayNew(aty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::StructNew(vty));
        f.instruction(&Instruction::LocalSet(vec_ref));

        // Init loop state
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(start));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));

        // if delim_len == 0: push s as sole element; else: run loop
        f.instruction(&Instruction::LocalGet(delim_len));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // empty delimiter → push s as sole element
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::StructGet {
            struct_type_index: vty,
            field_index: 0,
        });
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::ArraySet(aty));
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::StructSet {
            struct_type_index: vty,
            field_index: 1,
        });
        f.instruction(&Instruction::Else);

        // Main loop: while i + delim_len <= s_len
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // break if i + delim_len > s_len
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(delim_len));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::I32GtU);
        f.instruction(&Instruction::BrIf(1));

        // Inner: try to match delim at position i; j counts matched bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(j));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(delim_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // all bytes matched
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::BrIf(1)); // mismatch
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(j));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end inner loop
        f.instruction(&Instruction::End); // end inner block

        // if j == delim_len: full match
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(delim_len));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        // Extract segment s[start..i]
        // seg_len = i - start (reuse j for this)
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(j)); // j = seg_len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalSet(seg_ref));
        f.instruction(&Instruction::LocalGet(seg_ref));
        f.instruction(&Instruction::I32Const(0)); // dst_off
        f.instruction(&Instruction::LocalGet(s0)); // src
        f.instruction(&Instruction::LocalGet(start)); // src_off
        f.instruction(&Instruction::LocalGet(j)); // len
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });

        // Inline push: backing[vec_len] = seg_ref; vec_len += 1
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::StructGet {
            struct_type_index: vty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(j)); // j = vec_len (reuse)
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::StructGet {
            struct_type_index: vty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(seg_ref));
        f.instruction(&Instruction::ArraySet(aty));
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::StructSet {
            struct_type_index: vty,
            field_index: 1,
        });

        // Advance: start = i + delim_len; i = start
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(delim_len));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(start));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::LocalSet(i));

        f.instruction(&Instruction::Else);
        // No match: i++
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::End); // end if match

        f.instruction(&Instruction::Br(0)); // continue outer loop
        f.instruction(&Instruction::End); // end outer loop
        f.instruction(&Instruction::End); // end outer block

        // Final segment: s[start..s_len]
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(j)); // j = seg_len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalSet(seg_ref));
        f.instruction(&Instruction::LocalGet(seg_ref));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });
        // Inline push final segment
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::StructGet {
            struct_type_index: vty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(j));
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::StructGet {
            struct_type_index: vty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(seg_ref));
        f.instruction(&Instruction::ArraySet(aty));
        f.instruction(&Instruction::LocalGet(vec_ref));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::StructSet {
            struct_type_index: vty,
            field_index: 1,
        });

        f.instruction(&Instruction::End); // end else branch

        // Leave result vec on stack
        f.instruction(&Instruction::LocalGet(vec_ref));
    }
    pub(super) fn emit_string_clone_gc(&mut self, f: &mut PeepholeWriter<'_>, arg: &Operand) {
        let s0 = self.si(4);
        let len = self.si(0);
        let sty = self.string_ty;

        self.emit_operand(f, arg);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len));

        // Create new array of same length
        f.instruction(&Instruction::I32Const(0)); // fill
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayNew(sty));

        // array.copy dst dst_off src src_off len
        // But we need to keep the result ref. Use local.tee.
        let result = self.si(5);
        f.instruction(&Instruction::LocalTee(result));
        f.instruction(&Instruction::I32Const(0)); // dst_off
        f.instruction(&Instruction::LocalGet(s0)); // src
        f.instruction(&Instruction::I32Const(0)); // src_off
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });
        f.instruction(&Instruction::LocalGet(result));
    }

    /// substring(s, start, end) → new GC string
    pub(super) fn emit_substring_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        s: &Operand,
        start: &Operand,
        end: &Operand,
    ) {
        let s0 = self.si(4);
        let result = self.si(5);
        let start_idx = self.si(0);
        let sub_len = self.si(1);
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalSet(s0));
        self.emit_operand(f, start);
        f.instruction(&Instruction::LocalSet(start_idx));
        self.emit_operand(f, end);
        f.instruction(&Instruction::LocalGet(start_idx));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(sub_len)); // sub_len = end - start

        // Create new array
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalTee(result));

        // array.copy result 0 s0 start sub_len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(start_idx));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });
        f.instruction(&Instruction::LocalGet(result));
    }

    /// to_uppercase/to_lowercase: clone + byte-by-byte transform
    pub(super) fn emit_case_transform_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        arg: &Operand,
        to_upper: bool,
    ) {
        let s0 = self.si(4);
        let result = self.si(5);
        let len = self.si(0);
        let i = self.si(1);
        let ch = self.si(2);
        let sty = self.string_ty;

        // Clone the string
        self.emit_operand(f, arg);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalTee(result));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });

        // Transform each byte in-place
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // ch = result[i]
        f.instruction(&Instruction::LocalGet(result));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalSet(ch));

        if to_upper {
            // if ch >= 'a' && ch <= 'z' then ch - 32
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(97)); // 'a'
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(122)); // 'z'
            f.instruction(&Instruction::I32LeU);
            f.instruction(&Instruction::I32And);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(result));
            f.instruction(&Instruction::LocalGet(i));
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(32));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::ArraySet(sty));
            f.instruction(&Instruction::End);
        } else {
            // if ch >= 'A' && ch <= 'Z' then ch + 32
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(65)); // 'A'
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(90)); // 'Z'
            f.instruction(&Instruction::I32LeU);
            f.instruction(&Instruction::I32And);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(result));
            f.instruction(&Instruction::LocalGet(i));
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::ArraySet(sty));
            f.instruction(&Instruction::End);
        }

        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(result));
    }

    /// trim(s) → new string with leading/trailing whitespace removed
    pub(super) fn emit_trim_gc(&mut self, f: &mut PeepholeWriter<'_>, arg: &Operand) {
        let s0 = self.si(4);
        let len = self.si(0);
        let start = self.si(1);
        let end_idx = self.si(2);
        let sty = self.string_ty;

        self.emit_operand(f, arg);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalTee(len));
        f.instruction(&Instruction::LocalSet(end_idx)); // end = len

        // Find start: skip leading spaces/tabs/newlines
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(start));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Const(32)); // space
        f.instruction(&Instruction::I32LeU); // byte <= ' ' (covers space, tab, newline, etc.)
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // break if not whitespace
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(start));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Find end: skip trailing whitespace
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(end_idx));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::I32LeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(end_idx));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Const(32));
        f.instruction(&Instruction::I32LeU);
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(end_idx));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(end_idx));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Create substring [start..end]
        self.emit_substring_gc_raw(f, s0, start, end_idx);
    }

    /// Helper: substring from scratch locals (src_ref_local, start_local, end_local)
    pub(super) fn emit_substring_gc_raw(
        &self,
        f: &mut PeepholeWriter<'_>,
        src_local: u32,
        start_local: u32,
        end_local: u32,
    ) {
        let result = self.si(5);
        let sub_len = self.si(3);
        let sty = self.string_ty;

        f.instruction(&Instruction::LocalGet(end_local));
        f.instruction(&Instruction::LocalGet(start_local));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalTee(sub_len));

        // Handle empty/negative length
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LeS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ref_nullable(sty),
        )));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::Else);

        f.instruction(&Instruction::I32Const(0)); // fill
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalTee(result));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(src_local));
        f.instruction(&Instruction::LocalGet(start_local));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });
        f.instruction(&Instruction::LocalGet(result));

        f.instruction(&Instruction::End);
    }

    /// starts_with(s, prefix) → i32 (0 or 1)
    pub(super) fn emit_starts_with_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        s: &Operand,
        prefix: &Operand,
    ) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let s_len = self.si(0);
        let p_len = self.si(1);
        let i = self.si(2);
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(s_len));
        self.emit_operand(f, prefix);
        f.instruction(&Instruction::LocalTee(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(p_len));

        // If s_len < p_len, return false
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(p_len));
        f.instruction(&Instruction::I32LtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Else);

        // Compare bytes using scratch local for result
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        let result_scr = self.si(3);
        f.instruction(&Instruction::I32Const(1)); // assume true
        f.instruction(&Instruction::LocalSet(result_scr));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(p_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // done, true
        // Compare s[i] vs prefix[i]
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(result_scr)); // mark false
        f.instruction(&Instruction::Br(2)); // break out
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(result_scr));

        f.instruction(&Instruction::End); // end if
    }

    /// ends_with(s, suffix) → i32
    pub(super) fn emit_ends_with_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        s: &Operand,
        suffix: &Operand,
    ) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let s_len = self.si(0);
        let sf_len = self.si(1);
        let i = self.si(2);
        let offset = self.si(3);
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(s_len));
        self.emit_operand(f, suffix);
        f.instruction(&Instruction::LocalTee(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(sf_len));

        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(sf_len));
        f.instruction(&Instruction::I32LtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Else);

        // offset = s_len - sf_len
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(sf_len));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(offset));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        // Use s_len (si(0)) as result scratch — s_len is no longer needed after offset calc
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(s_len));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(sf_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(offset));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(s_len)); // mark false
        f.instruction(&Instruction::Br(2));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(s_len)); // push result

        f.instruction(&Instruction::End);
    }

    /// String equality: [ref $string, ref $string] → i32
    /// Compares two GC string arrays byte-by-byte
    pub(super) fn emit_string_eq_gc(&mut self, f: &mut PeepholeWriter<'_>) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let len_a = self.si(0);
        let i = self.si(1);
        let result_scr = self.si(9); // use extra scratch to avoid conflicts
        let sty = self.string_ty;

        // Store both refs
        f.instruction(&Instruction::LocalSet(s1));
        f.instruction(&Instruction::LocalSet(s0));

        // Compare lengths first
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalTee(len_a));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(0)); // different lengths → not equal
        f.instruction(&Instruction::Else);

        // Same length — compare bytes using a scratch local for result
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::I32Const(1)); // assume equal
        f.instruction(&Instruction::LocalSet(result_scr));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len_a: break (equal)
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(len_a));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if s0[i] != s1[i]: not equal
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(result_scr)); // mark not equal
        f.instruction(&Instruction::Br(2)); // break out of block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Push result from scratch local
        f.instruction(&Instruction::LocalGet(result_scr));

        f.instruction(&Instruction::End); // end if (len mismatch)
    }

    /// contains(s, sub) → i32
    pub(super) fn emit_contains_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        s: &Operand,
        sub: &Operand,
    ) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let s_len = self.si(0);
        let sub_len = self.si(1);
        let i = self.si(2);
        let j = self.si(3);
        let result = self.si(9); // extra i32 scratch for result
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(s_len));
        self.emit_operand(f, sub);
        f.instruction(&Instruction::LocalTee(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(sub_len));

        // Empty substring always matches
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Else);

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(result)); // assume not found
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));

        // outer loop: for i in 0..=(s_len - sub_len)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // inner check: compare sub bytes using j as match flag
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(j));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // all bytes matched
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::BrIf(1)); // mismatch — break inner
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(j));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end inner loop
        f.instruction(&Instruction::End); // end inner block

        // After inner: if j == sub_len, all bytes matched → found
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(result));
        f.instruction(&Instruction::Br(1)); // break outer
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end outer loop
        f.instruction(&Instruction::End); // end outer block

        f.instruction(&Instruction::LocalGet(result));

        f.instruction(&Instruction::End); // end if (empty sub)
    }

    /// join(parts_vec, separator) → String
    pub(super) fn emit_join_gc(&mut self, f: &mut PeepholeWriter<'_>, _args: &[Operand]) {
        // Stub: return empty string for now (join requires Vec<String> access)
        let sty = self.string_ty;
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayNew(sty));
    }

    /// Emit inline filter HOF: filter(vec, predicate_fn) -> new_vec
    /// Uses scratch memory for loop state.
    pub(super) fn emit_filter_hof_inline(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let (elem_size, is_f64, is_i64) = match canonical {
            "filter_f64" => (8i32, true, false),
            "filter_i64" => (8, false, true),
            _ => (4, false, false),
        };

        let pred_type = if is_f64 {
            self.indirect_types
                .get(&(vec![ValType::F64], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        } else if is_i64 {
            self.indirect_types
                .get(&(vec![ValType::I64], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        } else {
            self.indirect_types
                .get(&(vec![ValType::I32], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        };

        // SCR_A_PTR = vec_ptr, SCR_B_PTR = fn_idx
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_A_LEN = len(vec)
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Create new_vec (same capacity as original)
        self.emit_vec_new_inline(f, elem_size);
        // SCR_DST_PTR = new_vec
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        // Swap: the vec ptr is on stack
        f.instruction(&Instruction::I32Store(ma));

        // SCR_I = 0 (loop counter), SCR_J = 0 (new_len)
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= n: break
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // Store element to SCR_VAL64
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // data_ptr
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(elem_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        if is_f64 {
            f.instruction(&Instruction::F64Load(ma));
            f.instruction(&Instruction::F64Store(ma));
        } else if is_i64 {
            f.instruction(&Instruction::I64Load(ma));
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
        }

        // Call predicate: pred(value) -> i32
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if is_f64 {
            f.instruction(&Instruction::F64Load(ma));
        } else if is_i64 {
            f.instruction(&Instruction::I64Load(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
        }
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma)); // fn_idx
        f.instruction(&Instruction::CallIndirect {
            type_index: pred_type,
            table_index: 0,
        });

        // if predicate returned true: push value to new_vec
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // new_data + new_len * elem_size = value
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // new_data_ptr
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(elem_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if is_f64 {
            f.instruction(&Instruction::F64Load(ma));
            f.instruction(&Instruction::F64Store(ma));
        } else if is_i64 {
            f.instruction(&Instruction::I64Load(ma));
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
        }
        // new_len++
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End); // end if

        // i++
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Set new_vec len = new_len
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Result: new_vec ptr
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
    }

    /// Emit inline map HOF: map(vec, mapper_fn) -> new_vec
    pub(super) fn emit_map_hof_inline(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let (in_size, out_size, in_f64, in_i64, out_f64, out_i64) = match canonical {
            "map_f64_f64" => (8i32, 8i32, true, false, true, false),
            "map_i64_i64" => (8, 8, false, true, false, true),
            _ => (4, 4, false, false, false, false), // i32->i32 or i32->String
        };

        let map_type = if in_f64 {
            self.indirect_types
                .get(&(vec![ValType::F64], vec![ValType::F64]))
                .copied()
                .unwrap_or(0)
        } else if in_i64 {
            self.indirect_types
                .get(&(vec![ValType::I64], vec![ValType::I64]))
                .copied()
                .unwrap_or(0)
        } else {
            self.indirect_types
                .get(&(vec![ValType::I32], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        };

        // SCR_A_PTR = vec_ptr, SCR_B_PTR = fn_idx
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_A_LEN = len(vec)
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Create new_vec
        self.emit_vec_new_inline(f, out_size);
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Store(ma));

        // SCR_I = 0
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // Load element
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(in_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        if in_f64 {
            f.instruction(&Instruction::F64Load(ma));
        } else if in_i64 {
            f.instruction(&Instruction::I64Load(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
        }

        // Call mapper: fn(val) -> mapped_val
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::CallIndirect {
            type_index: map_type,
            table_index: 0,
        });

        // Store result to new_data[i]
        // First save mapped value to SCR_VAL64
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if out_f64 {
            f.instruction(&Instruction::F64Store(ma));
        } else if out_i64 {
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Store(ma));
        }

        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // new_data_ptr
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(out_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if out_f64 {
            f.instruction(&Instruction::F64Load(ma));
            f.instruction(&Instruction::F64Store(ma));
        } else if out_i64 {
            f.instruction(&Instruction::I64Load(ma));
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
        }

        // i++
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Set new_vec len = original len
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Result: new_vec ptr
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
    }

    /// Emit inline fold HOF: fold(vec, init, folder_fn) -> accumulated
    pub(super) fn emit_fold_hof_inline(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 3 {
            f.instruction(&Instruction::I64Const(0));
            return;
        }

        let fold_type = self
            .indirect_types
            .get(&(vec![ValType::I64, ValType::I64], vec![ValType::I64]))
            .copied()
            .unwrap_or(0);

        // SCR_A_PTR = vec_ptr
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_VAL64 = init (i64, stored as 8 bytes)
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        // Coerce init to i64 if it's a ConstI32
        self.emit_operand_coerced(f, &args[1], true, false);
        f.instruction(&Instruction::I64Store(ma));

        // SCR_B_PTR = fn_idx
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[2]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_A_LEN = len
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // SCR_I = 0
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // acc = folder(acc, element[i])
        // Push store destination address first (for I64Store after call)
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        // Push call args: acc, element
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        f.instruction(&Instruction::I64Load(ma)); // acc
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // data_ptr
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I64Load(ma)); // element
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma)); // fn_idx
        f.instruction(&Instruction::CallIndirect {
            type_index: fold_type,
            table_index: 0,
        });
        // Store result: stack is [addr, i64_result]
        f.instruction(&Instruction::I64Store(ma));

        // i++
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Result: acc
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        f.instruction(&Instruction::I64Load(ma));
    }

    /// Extract a direct function index from a FnRef operand (for GC-native HOF).
    pub(super) fn extract_fn_idx(&self, op: &Operand) -> Option<u32> {
        match op {
            Operand::FnRef(name) => self.fn_map.get(name.as_str()).copied(),
            Operand::Place(Place::Local(_id)) => {
                // Closure: check if the local was bound to a closure → resolve to synthetic fn
                None // For now, only handle FnRef
            }
            _ => None,
        }
    }

    /// Resolve the GC array type index and vec type index for a HOF canonical name.
    pub(super) fn hof_gc_types(&self, canonical: &str) -> (u32, u32) {
        if canonical.contains("String") {
            (self.arr_string_ty, self.vec_string_ty)
        } else if canonical.contains("i64") {
            (self.arr_i64_ty, self.vec_i64_ty)
        } else if canonical.contains("f64") {
            (self.arr_f64_ty, self.vec_f64_ty)
        } else {
            (self.arr_i32_ty, self.vec_i32_ty)
        }
    }

    /// Determine the output GC array/vec types for map operations.
    pub(super) fn hof_map_output_types(&self, canonical: &str) -> (u32, u32) {
        match canonical {
            "map_i32_String" | "map_i64_String" | "map_f64_String" | "map_String_String" => {
                (self.arr_string_ty, self.vec_string_ty)
            }
            "map_i64_i64" => (self.arr_i64_ty, self.vec_i64_ty),
            "map_f64_f64" => (self.arr_f64_ty, self.vec_f64_ty),
            _ => (self.arr_i32_ty, self.vec_i32_ty), // map_i32_i32
        }
    }

    /// GC-native filter HOF: filter(vec, predicate) -> new_vec
    /// Uses GC struct/array ops instead of linear memory.
    pub(super) fn emit_filter_hof_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let pred_fn_idx = self.extract_fn_idx(&args[1]);

        // si(0) = len, si(1) = i, si(2) = j
        // si(10) = src_arr (anyref), si(11) = dst_arr (anyref)

        // Get source array and length
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10))); // src_arr → anyref

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0))); // len

        // Create result array with same capacity
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::ArrayNewDefault(arr_ty));
        f.instruction(&Instruction::LocalSet(self.si(11))); // dst_arr → anyref

        // i = 0, j = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(2)));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len: break
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_arr[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        // elem is on stack — save to appropriate scratch
        let elem_scratch = if canonical.contains("String") {
            self.si(4) // ref $string
        } else if canonical.contains("i64") {
            self.si(6) // i64
        } else if canonical.contains("f64") {
            self.si(7) // f64
        } else {
            self.si(3) // i32
        };
        f.instruction(&Instruction::LocalTee(elem_scratch));

        // Call predicate
        if let Some(fn_idx) = pred_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            // Fallback: emit operand (FnRef) and call_indirect
            self.emit_operand(f, &args[1]);
            let pred_type = self
                .indirect_types
                .get(&(vec![ValType::I32], vec![ValType::I32]))
                .copied()
                .unwrap_or(0);
            f.instruction(&Instruction::CallIndirect {
                type_index: pred_type,
                table_index: 0,
            });
        }

        // if predicate true
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(2))); // j
        f.instruction(&Instruction::LocalGet(elem_scratch));
        f.instruction(&Instruction::ArraySet(arr_ty));
        // j++
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        f.instruction(&Instruction::End); // end if

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Create result vec: struct.new $vec_ty (dst_arr, j)
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::StructNew(vec_ty));
    }

    /// GC-native map HOF: map(vec, mapper) -> new_vec
    pub(super) fn emit_map_hof_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            return;
        }

        let (in_arr_ty, in_vec_ty) = self.hof_gc_types(canonical);
        let (out_arr_ty, out_vec_ty) = self.hof_map_output_types(canonical);
        let map_fn_idx = self.extract_fn_idx(&args[1]);

        // si(0) = len, si(1) = i
        // si(10) = src_arr (anyref), si(11) = dst_arr (anyref)

        // Get source array and length
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: in_vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: in_vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // Create result array with same length
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::ArrayNewDefault(out_arr_ty));
        f.instruction(&Instruction::LocalSet(self.si(11)));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len: break
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // dst_arr[i] = mapper(src_arr[i])
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(out_arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1))); // i (index for array.set)

        // Push src_arr[i] as argument to mapper
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(in_arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(in_arr_ty));

        // Call mapper
        if let Some(fn_idx) = map_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::I32Const(0));
        }

        // array.set dst_arr[i] = result
        f.instruction(&Instruction::ArraySet(out_arr_ty));

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Create result vec
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(out_arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0))); // len = same as input
        f.instruction(&Instruction::StructNew(out_vec_ty));
    }

    /// GC-native fold HOF: fold(vec, init, folder) -> acc
    pub(super) fn emit_fold_hof_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.len() < 3 {
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let fold_fn_idx = self.extract_fn_idx(&args[2]);

        // si(0) = len, si(1) = i, si(3) = acc
        // si(10) = src_arr (anyref)

        // Get source array and length
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // acc = init
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // acc = folder(acc, src_arr[i])
        f.instruction(&Instruction::LocalGet(self.si(3))); // acc
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty)); // element

        if let Some(fn_idx) = fold_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::Drop);
        }
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Result: acc
        f.instruction(&Instruction::LocalGet(self.si(3)));
    }

    /// GC-native any HOF: any(vec, pred) -> bool
    pub(super) fn emit_any_hof_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let pred_fn_idx = self.extract_fn_idx(&args[1]);

        // si(0) = len, si(1) = i, si(3) = result
        // si(10) = src_arr (anyref)

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(3))); // result = false
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_arr[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));

        if let Some(fn_idx) = pred_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::I32Const(0));
        }

        // if pred true: result = true, break
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(3)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(self.si(3)));
    }

    /// GC-native find HOF: find(vec, pred) -> Option<T>
    pub(super) fn emit_find_hof_gc(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            // Return None
            if let Some(&base_ty) = self.enum_base_types.get("Option") {
                f.instruction(&Instruction::StructNew(base_ty + 2)); // None variant
            } else {
                f.instruction(&Instruction::I32Const(0));
            }
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let pred_fn_idx = self.extract_fn_idx(&args[1]);

        // Get Option variant types
        let option_some_ty = self
            .enum_base_types
            .get("Option")
            .map(|b| b + 1)
            .unwrap_or(0);
        let option_none_ty = self
            .enum_base_types
            .get("Option")
            .map(|b| b + 2)
            .unwrap_or(0);

        // si(0) = len, si(1) = i, si(3) = found_elem, si(9) = found flag
        // si(10) = src_arr (anyref)

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9))); // found = false
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(3))); // found_elem = 0

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_arr[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalTee(self.si(3)));

        if let Some(fn_idx) = pred_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::I32Const(0));
        }

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9))); // found = true
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Return Option: if found, Some(elem), else None
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::FunctionType(
            self.types
                .add_func(&[], &[ref_nullable(option_some_ty - 1)]),
        )));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::StructNew(option_some_ty));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::StructNew(option_none_ty));
        f.instruction(&Instruction::End);
    }

    /// GC-native fs_read_file(path) -> Result<String, String>
    /// Copy GC path to linear memory, call path_open + fd_read, build GC string result
    pub(super) fn emit_fs_read_file_gc(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.is_empty() {
            return;
        }

        let result_base = *self.enum_base_types.get("Result_String_String").unwrap();
        let ok_variant = result_base + 1;
        let err_variant = result_base + 2;
        let ma = wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };

        // Pre-allocate error strings as data segments (absolute index = active segs + relative)
        let data_seg_base = self.data_segs.len() as u32;
        let err_open_seg = data_seg_base + self.alloc_string_data(b"file open error");
        let _err_read_seg = data_seg_base + self.alloc_string_data(b"file read error");

        // Step 1: Copy GC path string to linear memory at FS_SCRATCH+32
        // si(0) = path_len, si(1) = loop counter
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // path ref → anyref

        // Get path length
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // path_len

        // Copy path bytes to linear memory at FS_SCRATCH+32
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[FS_SCRATCH+32 + i] = path[i]
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Step 2: path_open(dirfd=3, dirflags=0, path_ptr, path_len, oflags=0,
        //                   rights=FD_READ(2), inheriting=0, fdflags=0, &opened_fd)
        // Store opened_fd at FS_SCRATCH
        f.instruction(&Instruction::I32Const(3)); // dirfd
        f.instruction(&Instruction::I32Const(0)); // dirflags
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32)); // path_ptr
        f.instruction(&Instruction::LocalGet(self.si(0))); // path_len
        f.instruction(&Instruction::I32Const(0)); // oflags
        f.instruction(&Instruction::I64Const(2)); // rights: FD_READ
        f.instruction(&Instruction::I64Const(0)); // inheriting
        f.instruction(&Instruction::I32Const(0)); // fdflags
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32)); // &opened_fd
        f.instruction(&Instruction::Call(self.wasi_path_open));

        // Step 3: Check error
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Return Err("file open error")
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(15)); // "file open error" len
        f.instruction(&Instruction::ArrayNewData {
            array_type_index: self.string_ty,
            array_data_index: err_open_seg,
        });
        f.instruction(&Instruction::StructNew(err_variant));
        f.instruction(&Instruction::LocalSet(self.si(10))); // store result
        f.instruction(&Instruction::Else);

        // Step 4: Read file in loop
        // si(2) = total_read, si(3) = fd
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3))); // fd

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(2))); // total_read = 0

        // Read loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // Set up iovec: base = FS_SCRATCH+32 + total_read, len = BUF_SIZE
        // IOV at mem[0..7]: base(4) + len(4)
        f.instruction(&Instruction::I32Const(0)); // iov_base addr
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(4)); // iov_len addr
        f.instruction(&Instruction::I32Const(FS_BUF_SIZE as i32));
        f.instruction(&Instruction::I32Store(ma));

        // fd_read(fd, &iov, 1, &nread)
        // nread at mem[8]
        f.instruction(&Instruction::LocalGet(self.si(3))); // fd
        f.instruction(&Instruction::I32Const(0)); // iov_ptr
        f.instruction(&Instruction::I32Const(1)); // iov_count
        f.instruction(&Instruction::I32Const(8)); // &nread
        f.instruction(&Instruction::Call(self.wasi_fd_read));
        f.instruction(&Instruction::Drop); // drop errno

        // nread = mem[8]
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Load(ma));

        // if nread == 0: break
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // break out of loop

        // total_read += nread
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));

        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Step 5: Close fd
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::Call(self.wasi_fd_close));
        f.instruction(&Instruction::Drop);

        // Step 6: Build Ok(string) — copy linear memory to GC string
        // Create GC string of total_read length
        f.instruction(&Instruction::LocalGet(self.si(2))); // total_read
        f.instruction(&Instruction::ArrayNewDefault(self.string_ty));
        f.instruction(&Instruction::LocalSet(self.si(4))); // result string ref

        // Copy loop: for i in 0..total_read { string[i] = mem[FS_SCRATCH+32+i] }
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // string[i] = mem[FS_SCRATCH+32 + i]
        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::ArraySet(self.string_ty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Build Ok(string)
        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::StructNew(ok_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));

        f.instruction(&Instruction::End); // end if/else

        // Push result
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNullable(HeapType::Concrete(
            result_base,
        )));
    }

    /// GC-native fs_write_file(path, content) -> Result<(), String>
    pub(super) fn emit_fs_write_file_gc(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }

        let result_base = *self.enum_base_types.get("Result").unwrap();
        let ok_variant = result_base + 1;
        let err_variant = result_base + 2;
        let ma = wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };

        let err_write_seg =
            self.data_segs.len() as u32 + self.alloc_string_data(b"file write error");

        // Step 1: Copy path to linear memory at FS_SCRATCH+32
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // path ref

        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // path_len

        // Copy path bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 2: path_open for writing
        // oflags = O_CREAT(1) | O_TRUNC(8) = 9
        // rights = FD_WRITE(64)
        f.instruction(&Instruction::I32Const(3)); // dirfd
        f.instruction(&Instruction::I32Const(0)); // dirflags
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::LocalGet(self.si(0))); // path_len
        f.instruction(&Instruction::I32Const(9)); // O_CREAT | O_TRUNC
        f.instruction(&Instruction::I64Const(64)); // FD_WRITE
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32)); // &fd
        f.instruction(&Instruction::Call(self.wasi_path_open));

        // Check error
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Err("file write error")
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(16)); // len
        f.instruction(&Instruction::ArrayNewData {
            array_type_index: self.string_ty,
            array_data_index: err_write_seg,
        });
        f.instruction(&Instruction::StructNew(err_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));
        f.instruction(&Instruction::Else);

        // Step 3: Copy content GC string to linear memory
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(11))); // content ref

        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(2))); // content_len

        // Copy content bytes to linear memory at FS_SCRATCH+32
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 4: fd_write
        // Get fd
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3))); // fd

        // iov: base=FS_SCRATCH+32, len=content_len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::LocalGet(self.si(2))); // content_len
        f.instruction(&Instruction::I32Store(ma));

        // fd_write(fd, &iov, 1, &nwritten)
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::I32Const(0)); // iov_ptr
        f.instruction(&Instruction::I32Const(1)); // iov_count
        f.instruction(&Instruction::I32Const(8)); // &nwritten
        f.instruction(&Instruction::Call(self.wasi_fd_write));
        f.instruction(&Instruction::Drop);

        // Close fd
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::Call(self.wasi_fd_close));
        f.instruction(&Instruction::Drop);

        // Build Ok(()) — Ok variant with i32(0) payload
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::StructNew(ok_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));

        f.instruction(&Instruction::End); // end if/else

        // Push result
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNullable(HeapType::Concrete(
            result_base,
        )));
    }

    pub(super) fn emit_fs_write_bytes_gc(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }

        let result_base = *self.enum_base_types.get("Result").unwrap();
        let ok_variant = result_base + 1;
        let err_variant = result_base + 2;
        let ma = wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma8 = wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };

        let err_write_seg =
            self.data_segs.len() as u32 + self.alloc_string_data(b"file write error");

        // Step 1: Copy path bytes to linear memory at FS_SCRATCH+32
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // path ref

        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // path_len

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma8));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 2: path_open for writing
        f.instruction(&Instruction::I32Const(3)); // dirfd
        f.instruction(&Instruction::I32Const(0)); // dirflags
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::LocalGet(self.si(0))); // path_len
        f.instruction(&Instruction::I32Const(9)); // O_CREAT | O_TRUNC
        f.instruction(&Instruction::I64Const(64)); // FD_WRITE
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32)); // &fd
        f.instruction(&Instruction::Call(self.wasi_path_open));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Err("file write error")
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(16)); // len
        f.instruction(&Instruction::ArrayNewData {
            array_type_index: self.string_ty,
            array_data_index: err_write_seg,
        });
        f.instruction(&Instruction::StructNew(err_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));
        f.instruction(&Instruction::Else);

        // Step 3: Copy Vec<i32> bytes to linear memory at FS_SCRATCH+32
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(11))); // bytes vec ref

        // vec_len = StructGet(vec_i32_ty, field 1)
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.vec_i32_ty,
        )));
        f.instruction(&Instruction::StructGet {
            struct_type_index: self.vec_i32_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(2))); // vec_len

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // dst = FS_SCRATCH+32 + i
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        // value = backing_array[i]
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.vec_i32_ty,
        )));
        f.instruction(&Instruction::StructGet {
            struct_type_index: self.vec_i32_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(self.arr_i32_ty));
        f.instruction(&Instruction::I32Store8(ma8));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 4: fd_write
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3))); // fd

        // iov: [base=FS_SCRATCH+32, len=vec_len]
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::Call(self.wasi_fd_write));
        f.instruction(&Instruction::Drop);

        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::Call(self.wasi_fd_close));
        f.instruction(&Instruction::Drop);

        // Build Ok(()) result
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::StructNew(ok_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));

        f.instruction(&Instruction::End); // end if/else

        // Push result
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNullable(HeapType::Concrete(
            result_base,
        )));
    }

    pub(super) fn emit_vec_new(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        element_size: i32,
        dest: Option<&Place>,
    ) {
        self.emit_vec_new_inline(f, element_size);
        if let Some(Place::Local(id)) = dest {
            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
        } else {
            f.instruction(&Instruction::Drop);
        }
    }

    pub(super) fn emit_vec_new_inline(&mut self, f: &mut PeepholeWriter<'_>, _element_size: i32) {
        // GC-native: default to i32 vec (HOF callers should use emit_vec_new_gc)
        self.emit_vec_new_gc(f, self.vec_i32_ty, self.arr_i32_ty);
    }

    pub(super) fn emit_vec_new_gc(&mut self, f: &mut PeepholeWriter<'_>, vec_ty: u32, arr_ty: u32) {
        // Large initial capacity avoids needing grow logic for MVP
        let cap = 16384i32;
        // For string vecs, fill with ref.null; for scalars, fill with 0
        if arr_ty == self.arr_string_ty {
            f.instruction(&Instruction::RefNull(HeapType::Concrete(self.string_ty)));
        } else if arr_ty == self.arr_f64_ty {
            f.instruction(&Instruction::F64Const(0.0));
        } else if arr_ty == self.arr_i64_ty {
            f.instruction(&Instruction::I64Const(0));
        } else {
            // Check if this is a Vec<Struct> array — fill with ref.null of element type
            let mut is_struct_arr = false;
            for (sname, &(a_ty, _)) in &self.custom_vec_types {
                if a_ty == arr_ty {
                    if let Some(&struct_ty) = self.struct_gc_types.get(sname.as_str()) {
                        f.instruction(&Instruction::RefNull(HeapType::Concrete(struct_ty)));
                        is_struct_arr = true;
                    }
                    break;
                }
            }
            if !is_struct_arr {
                f.instruction(&Instruction::I32Const(0)); // fill value for i32 arrays
            }
        }
        f.instruction(&Instruction::I32Const(cap));
        f.instruction(&Instruction::ArrayNew(arr_ty));
        f.instruction(&Instruction::I32Const(0)); // len = 0
        f.instruction(&Instruction::StructNew(vec_ty));
    }

    /// HashMap_i32_i32_insert(map, key, value)
    /// Linear scan keys[0..count], update if found, else append.
    /// Uses scratch: si(0)=count, si(1)=i, si(2)=key, si(3)=value, si(9)=found
    pub(super) fn emit_hashmap_i32_i32_insert(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        args: &[Operand],
    ) {
        if args.len() < 3 {
            return;
        }
        let hm_ty = self.hashmap_i32_i32_ty;
        let arr_ty = self.arr_i32_ty;

        // Push map ref, get count
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // map → anyref scratch

        // key → si(2), value → si(3)
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        self.emit_operand(f, &args[2]);
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // count = map.count
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 2,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // i = 0, found = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9)));

        // Search loop: for i in 0..count { if keys[i] == key { values[i] = value; found = 1; break } }
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= count: break
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if keys[i] == key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Eq);

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // values[i] = value
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::ArraySet(arr_ty));
        // found = 1
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // If not found, append: keys[count] = key, values[count] = value, count++
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        // keys[count] = key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // values[count] = value
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // map.count = count + 1
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::StructSet {
            struct_type_index: hm_ty,
            field_index: 2,
        });

        f.instruction(&Instruction::End); // end if
    }

    /// HashMap_i32_i32_get(map, key) -> Option<i32>
    /// Linear scan, returns Some(value) or None as GC enum variants.
    pub(super) fn emit_hashmap_i32_i32_get(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            // Return None
            if let Some(&base_ty) = self.enum_base_types.get("Option") {
                f.instruction(&Instruction::StructNew(base_ty + 2));
            } else {
                f.instruction(&Instruction::I32Const(0));
            }
            return;
        }
        let hm_ty = self.hashmap_i32_i32_ty;
        let arr_ty = self.arr_i32_ty;
        let option_base = *self.enum_base_types.get("Option").unwrap_or(&0);
        let option_some_ty = option_base + 1;
        let option_none_ty = option_base + 2;

        // map → si(10)
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10)));

        // key → si(2)
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(2)));

        // count = map.count → si(0)
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 2,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // i = 0, found = 0, result_val = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9))); // found flag
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(3))); // result value

        // Search loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if keys[i] == key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Eq);

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // result_val = values[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalSet(self.si(3)));
        // found = 1
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Return Option: if found then Some(result_val) else None
        let option_ref = ref_nullable(option_base);
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            option_ref,
        )));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::StructNew(option_some_ty));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::StructNew(option_none_ty));
        f.instruction(&Instruction::End);
    }

    /// HashMap_i32_i32_contains_key(map, key) -> bool (i32)
    pub(super) fn emit_hashmap_i32_i32_contains_key(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }
        let hm_ty = self.hashmap_i32_i32_ty;
        let arr_ty = self.arr_i32_ty;

        // map → si(10)
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10)));

        // key → si(2)
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(2)));

        // count → si(0)
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 2,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // i = 0, found = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9)));

        // Search loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if keys[i] == key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Eq);

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        f.instruction(&Instruction::LocalGet(self.si(9)));
    }

    pub(super) fn emit_push(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);
        let is_f64 = self.is_f64_vec_operand(&args[0]);
        let is_i64 = self.is_i64_vec_operand(&args[0]);

        let scr_len = self.si(0);

        // len = struct.get $vec 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // array.set backing[len] = val
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_len));
        self.emit_operand_coerced(f, &args[1], is_i64, is_f64);
        f.instruction(&Instruction::ArraySet(arr_ty));

        // struct.set vec.len = len + 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::StructSet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
    }

    pub(super) fn emit_len_inline(&mut self, f: &mut PeepholeWriter<'_>, arg: Option<&Operand>) {
        if let Some(arg) = arg {
            let vt = self.infer_operand_type(arg);
            let is_ref = matches!(vt, ValType::Ref(_));
            self.emit_operand(f, arg);
            if is_ref {
                // GC-native: string → array.len, vec → struct.get $vec 1
                if self.is_string_like_operand(arg) {
                    f.instruction(&Instruction::ArrayLen);
                } else {
                    // Vec: length is field 1 of the vec struct
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: self.infer_vec_type_idx(arg),
                        field_index: 1,
                    });
                }
            } else {
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // Bridge fallback (non-GC operand)
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(ma));
            }
        } else {
            f.instruction(&Instruction::I32Const(0));
        }
    }

    pub(super) fn emit_get_unchecked_inline(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);

        // array.get backing[index]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::ArrayGet(arr_ty));
    }

    pub(super) fn emit_get_inline(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);
        let is_str_vec = self.is_string_vec_operand(&args[0]);

        // get() returns Option<T>; use Option_String for string vecs
        let option_key = if is_str_vec && self.enum_base_types.contains_key("Option_String") {
            "Option_String"
        } else {
            "Option"
        };
        if let Some(&base_ty) = self.enum_base_types.get(option_key) {
            let variants = self.enum_variant_types.get(option_key);
            let some_ty = variants.and_then(|v| v.get("Some").copied()).unwrap_or(0);
            let none_ty = variants.and_then(|v| v.get("None").copied()).unwrap_or(0);

            let scr_idx = self.si(0);
            let option_ref = ref_nullable(base_ty);

            // Bounds check: index < len
            self.emit_operand(f, &args[1]);
            f.instruction(&Instruction::LocalSet(scr_idx));
            f.instruction(&Instruction::LocalGet(scr_idx));
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 1,
            });
            f.instruction(&Instruction::I32LtU);

            f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                option_ref,
            )));

            // Some branch: get element, wrap in Option::Some
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 0,
            });
            f.instruction(&Instruction::LocalGet(scr_idx));
            f.instruction(&Instruction::ArrayGet(arr_ty));
            f.instruction(&Instruction::StructNew(some_ty));

            f.instruction(&Instruction::Else);

            // None branch
            f.instruction(&Instruction::StructNew(none_ty));

            f.instruction(&Instruction::End);
        } else {
            // Fallback: just do unchecked get
            self.emit_get_unchecked_inline(f, args);
        }
    }

    pub(super) fn emit_set(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.len() < 3 {
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);
        let is_f64 = self.is_f64_vec_operand(&args[0]);
        let is_i64 = self.is_i64_vec_operand(&args[0]);

        // array.set backing[index] = val
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        self.emit_operand(f, &args[1]);
        self.emit_operand_coerced(f, &args[2], is_i64, is_f64);
        f.instruction(&Instruction::ArraySet(arr_ty));
    }

    pub(super) fn emit_pop_inline(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.is_empty() {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);
        let is_str_vec = self.is_string_vec_operand(&args[0]);

        // pop() returns Option<T>; use Option_String for string vecs
        let option_key = if is_str_vec && self.enum_base_types.contains_key("Option_String") {
            "Option_String"
        } else {
            "Option"
        };
        if let Some(&base_ty) = self.enum_base_types.get(option_key) {
            let variants = self.enum_variant_types.get(option_key);
            let some_ty = variants.and_then(|v| v.get("Some").copied()).unwrap_or(0);
            let none_ty = variants.and_then(|v| v.get("None").copied()).unwrap_or(0);
            let option_ref = ref_nullable(base_ty);

            let scr_len = self.si(0);

            // len = struct.get $vec 1
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 1,
            });
            f.instruction(&Instruction::LocalSet(scr_len));

            // if len > 0
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                option_ref,
            )));

            // new_len = len - 1
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(scr_len));

            // Get element at new_len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 0,
            });
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::ArrayGet(arr_ty));

            // Update vec.len = new_len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::StructSet {
                struct_type_index: vec_ty,
                field_index: 1,
            });

            // Wrap in Option::Some
            f.instruction(&Instruction::StructNew(some_ty));

            f.instruction(&Instruction::Else);

            // None branch
            f.instruction(&Instruction::StructNew(none_ty));

            f.instruction(&Instruction::End);
        } else {
            // Fallback: return unchecked last element
            let scr_len = self.si(0);
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 1,
            });
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(scr_len));

            // Decrement len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::StructSet {
                struct_type_index: vec_ty,
                field_index: 1,
            });

            // Get element at new len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 0,
            });
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::ArrayGet(arr_ty));
        }
    }

    /// contains_i32(v, x) / contains_String(v, s) → bool
    pub(super) fn emit_contains_inline(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }
        let is_string = canonical.ends_with("String");
        let (vec_ty, arr_ty) = if is_string {
            (self.vec_string_ty, self.arr_string_ty)
        } else {
            let vt = self.infer_vec_type_idx(&args[0]);
            let at = self.infer_arr_type_idx(&args[0]);
            (vt, at)
        };

        // Use si(2),si(3) for our loop vars to avoid conflict with string_eq_gc (uses si(0),si(1))
        let scr_i = self.si(2);
        let scr_len = self.si(3);

        // len = vec.len
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(scr_i));

        // block $done (result i32)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        // block $not_found
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        // loop $search
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len, br $not_found
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // break to $not_found

        // elem = backing[i]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::ArrayGet(arr_ty));

        // compare with target
        if is_string {
            // String comparison: call string_eq helper or element-wise
            self.emit_operand(f, &args[1]);
            self.emit_string_eq_gc(f);
        } else {
            self.emit_operand(f, &args[1]);
            f.instruction(&Instruction::I32Eq);
        }

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Br(3)); // br $done with 1 (if=0, loop=1, not_found=2, done=3)
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_i));
        f.instruction(&Instruction::Br(0)); // br $search

        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end $not_found
        f.instruction(&Instruction::I32Const(0)); // not found
        f.instruction(&Instruction::End); // end $done
    }

    /// reverse_i32(v) / reverse_String(v) — in-place reversal
    pub(super) fn emit_reverse_inline(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.is_empty() {
            return;
        }
        let is_string = canonical.ends_with("String");
        let (vec_ty, arr_ty) = if is_string {
            (self.vec_string_ty, self.arr_string_ty)
        } else {
            (
                self.infer_vec_type_idx(&args[0]),
                self.infer_arr_type_idx(&args[0]),
            )
        };
        let elem_vt = if is_string {
            ref_nullable(self.string_ty)
        } else {
            ValType::I32
        };

        let scr_lo = self.si(0);
        let scr_hi = self.si(1);
        let scr_tmp_idx = self.scratch_local_for_valtype(elem_vt);

        // lo = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(scr_lo));

        // hi = vec.len - 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(scr_hi));

        // loop while lo < hi
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(scr_lo));
        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::I32GeS);
        f.instruction(&Instruction::BrIf(1));

        // tmp = backing[lo]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_lo));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalSet(scr_tmp_idx));

        // backing[lo] = backing[hi]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_lo));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // backing[hi] = tmp
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::LocalGet(scr_tmp_idx));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // lo++; hi--
        f.instruction(&Instruction::LocalGet(scr_lo));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_lo));

        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(scr_hi));

        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block
    }

    /// remove_i32(v, idx) — shift elements left from idx+1..len, decrement len
    pub(super) fn emit_remove_inline(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }
        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);

        let scr_i = self.si(0);
        let scr_len = self.si(1);

        // len = vec.len
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // i = idx
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(scr_i));

        // shift loop: for i in idx..len-1: backing[i] = backing[i+1]
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len - 1, break
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // backing[i] = backing[i+1]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // i++
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_i));
        f.instruction(&Instruction::Br(0));

        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // vec.len = len - 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::StructSet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
    }

    /// sum_i32/i64/f64 / product_i32/i64/f64 — fold over vec
    pub(super) fn emit_sum_product_inline(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        canonical: &str,
        args: &[Operand],
    ) {
        if args.is_empty() {
            f.instruction(&Instruction::I32Const(0));
            return;
        }
        let is_product = canonical.starts_with("product");
        let is_i64 = canonical.ends_with("i64");
        let is_f64 = canonical.ends_with("f64");

        let (vec_ty, arr_ty) = if is_f64 {
            (self.vec_f64_ty, self.arr_f64_ty)
        } else if is_i64 {
            (self.vec_i64_ty, self.arr_i64_ty)
        } else {
            (
                self.infer_vec_type_idx(&args[0]),
                self.infer_arr_type_idx(&args[0]),
            )
        };

        let scr_i = self.si(0);
        let scr_len = self.si(1);
        let scr_acc = if is_f64 {
            self.scratch_local_for_valtype(ValType::F64)
        } else if is_i64 {
            self.scratch_local_for_valtype(ValType::I64)
        } else {
            self.si(2)
        };

        // len = vec.len
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // acc = identity (0 for sum, 1 for product)
        if is_f64 {
            f.instruction(&Instruction::F64Const(if is_product { 1.0 } else { 0.0 }));
        } else if is_i64 {
            f.instruction(&Instruction::I64Const(if is_product { 1 } else { 0 }));
        } else {
            f.instruction(&Instruction::I32Const(if is_product { 1 } else { 0 }));
        }
        f.instruction(&Instruction::LocalSet(scr_acc));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(scr_i));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len, break
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // acc = acc op backing[i]
        f.instruction(&Instruction::LocalGet(scr_acc));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::ArrayGet(arr_ty));

        if is_product {
            if is_f64 {
                f.instruction(&Instruction::F64Mul);
            } else if is_i64 {
                f.instruction(&Instruction::I64Mul);
            } else {
                f.instruction(&Instruction::I32Mul);
            }
        } else if is_f64 {
            f.instruction(&Instruction::F64Add);
        } else if is_i64 {
            f.instruction(&Instruction::I64Add);
        } else {
            f.instruction(&Instruction::I32Add);
        }
        f.instruction(&Instruction::LocalSet(scr_acc));

        // i++
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_i));
        f.instruction(&Instruction::Br(0));

        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // push acc
        f.instruction(&Instruction::LocalGet(scr_acc));
    }

    /// Emit `args()` builtin: returns Vec<String> of CLI arguments.
    ///
    /// Uses WASI `args_sizes_get` + `args_get` to read CLI args from linear
    /// memory, then copies each arg into a GC string array and pushes onto
    /// a Vec<String>.
    pub(super) fn emit_args_builtin(&mut self, f: &mut PeepholeWriter<'_>) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };

        // Scratch local mapping (must match declared types in helpers.rs):
        //   si(0)  = argc          (i32)
        //   si(1)  = reused:       buf_size initially, then j (byte copy), then vec_len (i32)
        //   si(2)  = i outer loop  (i32)
        //   si(3)  = str_ptr       (i32)
        //   si(4)  = str_ref       (ref null $string) ← ref type
        //   si(9)  = len           (i32)
        //   si(12) = vec result    (ref null $vec_string_ty)

        // Step 1: args_sizes_get
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
        f.instruction(&Instruction::Call(self.wasi_args_sizes_get));
        f.instruction(&Instruction::Drop);

        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(0))); // argc

        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(1))); // buf_size (temp)

        // Step 2: args_get — argv pointers at FS_SCRATCH, buf after argc*4
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::Call(self.wasi_args_get));
        f.instruction(&Instruction::Drop);

        // Step 3: create empty Vec<String>
        self.emit_vec_new_gc(f, self.vec_string_ty, self.arr_string_ty);
        f.instruction(&Instruction::LocalSet(self.si(12)));

        // Step 4: loop i = 1..argc (skip argv[0] = program name)
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(2)));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // str_ptr = mem[FS_SCRATCH + i * 4]
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // Scan null terminator → si(9) = len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9)));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Allocate GC string → si(4) (ref $string ✓)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(self.si(4)));

        // Copy bytes: j = 0 → si(1)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::ArraySet(self.string_ty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Push str_ref onto Vec<String> — inline push:
        // vec_len = struct.get $vec_string field 1 → si(1) (reuse as vec_len)
        f.instruction(&Instruction::LocalGet(self.si(12)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: self.vec_string_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(1)));

        // backing_array[vec_len] = str_ref
        f.instruction(&Instruction::LocalGet(self.si(12)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: self.vec_string_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::ArraySet(self.arr_string_ty));

        // vec.len = vec_len + 1
        f.instruction(&Instruction::LocalGet(self.si(12)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::StructSet {
            struct_type_index: self.vec_string_ty,
            field_index: 1,
        });

        // i++
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Leave vec ref on stack
        f.instruction(&Instruction::LocalGet(self.si(12)));
    }

    /// Emit `arg_count()` → i32: number of CLI arguments (excluding argv[0]).
    pub(super) fn emit_arg_count_builtin(&mut self, f: &mut PeepholeWriter<'_>) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // args_sizes_get writes argc at SCRATCH, buf_size at SCRATCH+4
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
        f.instruction(&Instruction::Call(self.wasi_args_sizes_get));
        f.instruction(&Instruction::Drop); // drop errno
        // Load argc, then subtract 1 to exclude argv[0]
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
    }

    /// Emit `arg_at(i: i32)` → String: return the i-th CLI argument string.
    pub(super) fn emit_arg_at_builtin(&mut self, f: &mut PeepholeWriter<'_>, index_op: &Operand) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };

        // Scratch local mapping (must match the declared types in helpers.rs):
        //   si(0) = argc        (i32)
        //   si(1) = buf_size    (i32)
        //   si(2) = adj_index   (i32) — reused as byte_loop_j after str_ptr is loaded
        //   si(3) = str_ptr     (i32)
        //   si(4) = str_ref     (ref null $string) ← ref type
        //   si(9) = len         (i32)

        // Step 1: args_sizes_get
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
        f.instruction(&Instruction::Call(self.wasi_args_sizes_get));
        f.instruction(&Instruction::Drop);

        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(0))); // argc

        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(1))); // buf_size

        // Step 2: args_get — argv pointers at FS_SCRATCH, buf after that
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::Call(self.wasi_args_get));
        f.instruction(&Instruction::Drop);

        // Step 3: adjusted index = i + 1 (skip argv[0])
        self.emit_operand(f, index_op);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));

        // Step 4: str_ptr = mem32[FS_SCRATCH + adj_index * 4]
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // Step 5: scan for null terminator to get length (stored in si(9))
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9))); // len = 0

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 6: allocate GC string of length `len`, stored in si(4) (ref $string)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(self.si(4)));

        // Step 7: copy bytes — reuse si(2) as byte loop counter j
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(2)));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::ArraySet(self.string_ty));

        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Leave str_ref on stack
        f.instruction(&Instruction::LocalGet(self.si(4)));
    }

    /// Emit `env_var(name: String) -> Option<String>`:
    /// Scan WASI environ for a variable matching `name`, return Some(value) or None.
    ///
    /// Scratch local usage:
    ///   si(0) = env_count (i32)
    ///   si(1) = reused: buf_size / eq_pos / byte / key_eq_idx / byte_copy_j (i32)
    ///   si(2) = i (outer loop counter) (i32)
    ///   si(3) = entry_ptr (i32)
    ///   si(4) = value string ref (ref null $string)
    ///   si(9) = name_len / entry total len / value_len (i32)
    ///   si(10) = name ref (anyref)
    pub(super) fn emit_env_var_builtin(&mut self, f: &mut PeepholeWriter<'_>, name_op: &Operand) {
        let ma = wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma1 = wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };

        let option_key = "Option_String";
        let option_some_ty = self
            .enum_base_types
            .get(option_key)
            .map(|b| b + 1)
            .unwrap_or(0);
        let option_none_ty = self
            .enum_base_types
            .get(option_key)
            .map(|b| b + 2)
            .unwrap_or(0);

        // Step 1: Store name ref in si(10), get name_len in si(9)
        self.emit_operand(f, name_op);
        f.instruction(&Instruction::LocalSet(self.si(10)));

        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(9))); // name_len

        // Step 2: Copy name bytes to linear memory at SCRATCH+16 for comparison
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // j = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[SCRATCH+16 + j] = name[j]
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 3: environ_sizes_get → env_count at SCRATCH, buf_size at SCRATCH+4
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
        f.instruction(&Instruction::Call(self.wasi_environ_sizes_get));
        f.instruction(&Instruction::Drop);

        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(0))); // env_count

        // Step 4: environ_get — use safe base past all data segments
        // Pointer table at env_base, buffer at env_base + env_count*4
        let env_base = ((self.data_offset + 255) / 256) * 256; // align to 256
        f.instruction(&Instruction::I32Const(env_base as i32));
        f.instruction(&Instruction::I32Const(env_base as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::Call(self.wasi_environ_get));
        f.instruction(&Instruction::Drop);

        // Step 5: Loop i = 0..env_count, search for matching key
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(2))); // i = 0

        // Outer block (break target for match found)
        // Block structure: block $found { block $not_found { loop $search { ... } } }
        // BrIf(2) from inside loop → $found (match), BrIf(1) from loop → $not_found (i>=count)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::FunctionType(
            self.types
                .add_func(&[], &[ref_nullable(option_some_ty - 1)]),
        )));
        // Inner block (break target for no match / loop exit)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= env_count, break to $not_found
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // → $not_found block

        // entry_ptr = mem32[env_base + i*4]
        f.instruction(&Instruction::I32Const(env_base as i32));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3))); // entry_ptr

        // Compare first name_len bytes of entry against name in SCRATCH+16
        // Then check that entry[name_len] == '='
        // Use si(1) as j for comparison
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // j = 0

        // Block structure: block $match { block $mismatch { loop $cmp { ... } } }
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // $match
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // $mismatch
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // $cmp

        // if j >= name_len → all name bytes matched, break to check '='
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(2)); // → $match (will check '=' next)

        // Compare entry_ptr[j] vs mem[SCRATCH+16 + j]
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma1));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::BrIf(1)); // → $mismatch

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0)); // → $cmp loop
        f.instruction(&Instruction::End); // $cmp loop
        f.instruction(&Instruction::End); // $mismatch block

        // Mismatch path: skip to next entry
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        f.instruction(&Instruction::Br(1)); // → search loop continue
        f.instruction(&Instruction::End); // $match block

        // All name_len bytes matched. Now check entry[name_len] == '='
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(9))); // name_len
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma1));
        f.instruction(&Instruction::I32Const(0x3D)); // '='
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Not '=' after key → skip to next entry
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        f.instruction(&Instruction::Br(1)); // → search loop continue
        f.instruction(&Instruction::End);

        // MATCH FOUND: Extract value (starts at entry_ptr + name_len + 1, ends at null)
        // value_start = entry_ptr + name_len + 1
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(9))); // name_len
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(3))); // si(3) = value_start

        // Scan for null terminator to get value length → si(9)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9))); // len = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma1));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Allocate GC string for value → si(4)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(self.si(4)));

        // Copy value bytes: j = 0..value_len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma1));
        f.instruction(&Instruction::ArraySet(self.string_ty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Return Some(value_str)
        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::StructNew(option_some_ty));
        f.instruction(&Instruction::Br(2)); // → $found typed block

        f.instruction(&Instruction::End); // search loop End
        f.instruction(&Instruction::End); // $not_found block End

        // Not found path: return None
        f.instruction(&Instruction::StructNew(option_none_ty));

        f.instruction(&Instruction::End); // outermost typed block
    }

    // ── HTTP builtins ────────────────────────────────────────────────

    /// Emit `http_get(url: String) -> Result<String, String>`:
    ///
    /// 1. Copy GC url string → linear memory at HTTP_SCRATCH_IN
    /// 2. Call host import http_get(url_ptr, url_len, resp_ptr) → i32
    ///    returns >= 0 → Ok (resp body len), < 0 → Err (abs = err msg len)
    /// 3. Copy response from linear memory → GC string
    /// 4. Wrap in Result::Ok or Result::Err
    ///
    /// Scratch usage: si(0)=url_len, si(1)=loop_i, si(2)=result, si(3)=resp_len,
    ///                si(10)=anyref
    pub(super) fn emit_http_get_builtin(&mut self, f: &mut PeepholeWriter<'_>, args: &[Operand]) {
        use super::{HTTP_SCRATCH_IN, HTTP_SCRATCH_RESP};

        if args.is_empty() {
            // Fallback: return Err("missing url")
            let result_base = *self.enum_base_types.get("Result_String_String").unwrap();
            let err_variant = result_base + 2;
            let seg = self.alloc_string_data(b"missing url");
            let abs_seg = self.data_segs.len() as u32 + seg;
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(11));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: abs_seg,
            });
            f.instruction(&Instruction::StructNew(err_variant));
            return;
        }

        let result_base = *self.enum_base_types.get("Result_String_String").unwrap();
        let ok_variant = result_base + 1;
        let err_variant = result_base + 2;
        let ma8 = wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };

        // Step 1: Copy url GC string → linear memory at HTTP_SCRATCH_IN
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // url ref → anyref

        // Get url length
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // url_len

        // Copy loop: url[i] → mem[HTTP_SCRATCH_IN + i]
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[HTTP_SCRATCH_IN + i] = url[i]
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(HTTP_SCRATCH_IN as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma8));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Step 2: Call host import http_get(url_ptr, url_len, resp_ptr) → i32
        f.instruction(&Instruction::I32Const(HTTP_SCRATCH_IN as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Const(HTTP_SCRATCH_RESP as i32));
        f.instruction(&Instruction::Call(self.host_http_get));
        f.instruction(&Instruction::LocalSet(self.si(2))); // result (signed)

        // Step 3: Determine ok/err and resp_len
        // result >= 0 → Ok, result < 0 → Err
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Err path: resp_len = -result
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalGet(self.si(2)));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(self.si(3)));
        }
        f.instruction(&Instruction::Else);
        {
            // Ok path: resp_len = result
            f.instruction(&Instruction::LocalGet(self.si(2)));
            f.instruction(&Instruction::LocalSet(self.si(3)));
        }
        f.instruction(&Instruction::End);

        // Step 4: Copy response from linear memory to GC string
        // Build new GC string array of length si(3), copy bytes from HTTP_SCRATCH_RESP
        self.emit_http_copy_resp_to_gc_string(f);

        // Step 5: Wrap in Result
        // si(10) now holds the response GC string
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Err variant
            f.instruction(&Instruction::LocalGet(self.si(10)));
            f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
                self.string_ty,
            )));
            f.instruction(&Instruction::StructNew(err_variant));
            f.instruction(&Instruction::LocalSet(self.si(10)));
        }
        f.instruction(&Instruction::Else);
        {
            // Ok variant
            f.instruction(&Instruction::LocalGet(self.si(10)));
            f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
                self.string_ty,
            )));
            f.instruction(&Instruction::StructNew(ok_variant));
            f.instruction(&Instruction::LocalSet(self.si(10)));
        }
        f.instruction(&Instruction::End);

        // Push result
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNullable(HeapType::Concrete(
            result_base,
        )));
    }

    /// Emit `http_request(method: String, url: String, body: String) -> Result<String, String>`:
    ///
    /// Similar to http_get but copies three strings to linear memory.
    /// Layout: method at HTTP_SCRATCH_IN, url after method, body after url.
    ///
    /// Scratch usage: si(0)=len, si(1)=loop_i, si(2)=result, si(3)=resp_len,
    ///                si(9)=offset, si(10)=anyref
    pub(super) fn emit_http_request_builtin(
        &mut self,
        f: &mut PeepholeWriter<'_>,
        args: &[Operand],
    ) {
        use super::{HTTP_SCRATCH_IN, HTTP_SCRATCH_RESP};

        if args.len() < 3 {
            let result_base = *self.enum_base_types.get("Result_String_String").unwrap();
            let err_variant = result_base + 2;
            let seg = self.alloc_string_data(b"missing arguments");
            let abs_seg = self.data_segs.len() as u32 + seg;
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(17));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: abs_seg,
            });
            f.instruction(&Instruction::StructNew(err_variant));
            return;
        }

        let result_base = *self.enum_base_types.get("Result_String_String").unwrap();
        let ok_variant = result_base + 1;
        let err_variant = result_base + 2;
        let ma8 = wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };

        // We'll store method_ptr, method_len, url_ptr, url_len, body_ptr, body_len
        // in fixed scratch i32 locals.  Use si(0)=current_len, si(1)=loop_i,
        // si(9)=write_offset to track where we are in HTTP_SCRATCH_IN.
        //
        // Layout in linear memory:
        //   method: [HTTP_SCRATCH_IN .. HTTP_SCRATCH_IN + method_len)
        //   url:    [method_end .. method_end + url_len)
        //   body:   [url_end .. url_end + body_len)
        //
        // We store method_ptr=HTTP_SCRATCH_IN and lengths in locals.

        // We need to track: method_len, url_start, url_len, body_start, body_len.
        // Use linear memory at a known scratch location for these.
        // Actually simpler: use si(0..9) for tracking.
        //
        // Plan:
        //   Copy method → HTTP_SCRATCH_IN, save method_len in mem[HTTP_SCRATCH_IN-16]
        //   Copy url → HTTP_SCRATCH_IN+method_len, save url_len in mem[HTTP_SCRATCH_IN-12]
        //   Copy body → ..., save body_len in mem[HTTP_SCRATCH_IN-8]
        //
        // Even simpler: just track offsets on scratch locals.

        let method_ptr_val = HTTP_SCRATCH_IN;

        // === Copy method string ===
        self.emit_operand(f, &args[0]); // method
        f.instruction(&Instruction::LocalSet(self.si(10))); // method ref
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // method_len

        // Store method_len in linear memory for later retrieval
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 16) as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Copy method bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(method_ptr_val as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma8));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // url_ptr = method_ptr + method_len
        f.instruction(&Instruction::I32Const(method_ptr_val as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(9))); // url_start offset

        // === Copy url string ===
        self.emit_operand(f, &args[1]); // url
        f.instruction(&Instruction::LocalSet(self.si(10))); // url ref
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // url_len

        // Store url_len
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 12) as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Copy url bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma8));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // body_start = url_start + url_len
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(9))); // body_start

        // === Copy body string ===
        self.emit_operand(f, &args[2]); // body
        f.instruction(&Instruction::LocalSet(self.si(10))); // body ref
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // body_len

        // Store body_len
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 8) as i32));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Copy body bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma8));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // === Call host import ===
        // http_request(method_ptr, method_len, url_ptr, url_len, body_ptr, body_len, resp_ptr)
        f.instruction(&Instruction::I32Const(method_ptr_val as i32)); // method_ptr
        // method_len from saved memory
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 16) as i32));
        f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        // url_ptr = method_ptr + method_len
        f.instruction(&Instruction::I32Const(method_ptr_val as i32));
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 16) as i32));
        f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        f.instruction(&Instruction::I32Add);
        // url_len from saved memory
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 12) as i32));
        f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        // body_ptr = url_ptr + url_len (= method_ptr + method_len + url_len)
        f.instruction(&Instruction::I32Const(method_ptr_val as i32));
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 16) as i32));
        f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 12) as i32));
        f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        f.instruction(&Instruction::I32Add);
        // body_len from saved memory
        f.instruction(&Instruction::I32Const((HTTP_SCRATCH_IN - 8) as i32));
        f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        // resp_ptr
        f.instruction(&Instruction::I32Const(HTTP_SCRATCH_RESP as i32));
        f.instruction(&Instruction::Call(self.host_http_request));
        f.instruction(&Instruction::LocalSet(self.si(2))); // result (signed)

        // Step 3: Determine ok/err and resp_len (same as http_get)
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalGet(self.si(2)));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(self.si(3)));
        }
        f.instruction(&Instruction::Else);
        {
            f.instruction(&Instruction::LocalGet(self.si(2)));
            f.instruction(&Instruction::LocalSet(self.si(3)));
        }
        f.instruction(&Instruction::End);

        // Step 4: Copy response → GC string (reuses shared helper)
        self.emit_http_copy_resp_to_gc_string(f);

        // Step 5: Wrap in Result
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(self.si(10)));
            f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
                self.string_ty,
            )));
            f.instruction(&Instruction::StructNew(err_variant));
            f.instruction(&Instruction::LocalSet(self.si(10)));
        }
        f.instruction(&Instruction::Else);
        {
            f.instruction(&Instruction::LocalGet(self.si(10)));
            f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
                self.string_ty,
            )));
            f.instruction(&Instruction::StructNew(ok_variant));
            f.instruction(&Instruction::LocalSet(self.si(10)));
        }
        f.instruction(&Instruction::End);

        // Push result
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNullable(HeapType::Concrete(
            result_base,
        )));
    }

    /// Shared helper: copy response bytes from linear memory at HTTP_SCRATCH_RESP
    /// into a new GC string array.  Length is in si(3).  Result ref is stored in si(10).
    fn emit_http_copy_resp_to_gc_string(&mut self, f: &mut PeepholeWriter<'_>) {
        use super::HTTP_SCRATCH_RESP;

        let ma8 = wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };

        // Allocate new GC string of length si(3)
        f.instruction(&Instruction::I32Const(0)); // fill value
        f.instruction(&Instruction::LocalGet(self.si(3))); // length
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(self.si(10))); // save new string ref

        // Copy loop: mem[HTTP_SCRATCH_RESP + i] → string[i]
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // string[i] = mem[HTTP_SCRATCH_RESP + i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        // Load byte from linear memory
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(HTTP_SCRATCH_RESP as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma8));
        // array.set
        f.instruction(&Instruction::ArraySet(self.string_ty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block
    }
}
