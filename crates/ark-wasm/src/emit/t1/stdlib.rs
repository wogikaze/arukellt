//! Stdlib builtin emission: vec ops (map/filter/fold), hashmap ops,
//! option ops, find/any.

use super::*;
use wasm_encoder::MemArg;

impl EmitCtx {
    /// __map_i32(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    /// Allocates a new Vec, iterates src, applies fn via call_indirect, pushes result.
    pub(super) fn build_map_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data
        let mut f = Function::new(vec![(5, ValType::I32)]);
        // n = vec.len
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        // src_data = vec.data_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        // Allocate new vec: [len, cap, data_ptr]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5)); // new_vec = heap
        // new_vec.len = n
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        // new_vec.cap = n
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        // new_data = heap + 12
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        // new_vec.data_ptr = new_data
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        // Loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= n, break
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // new_data[i] = call_indirect(fn_idx, src_data[i])
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add); // &new_data[i]
        // call_indirect(fn_idx, src_data[i])
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // src_data[i]
        f.instruction(&Instruction::LocalGet(1)); // fn table index
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::I32Store(ma)); // store result
        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block
        // return new_vec
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __filter_i32(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    pub(super) fn build_filter_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data, 7=new_len
        let mut f = Function::new(vec![(6, ValType::I32)]);
        // n = vec.len
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        // src_data = vec.data_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        // Allocate new vec with cap = n
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5));
        // new_vec.len = 0 (will be updated)
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        // new_vec.cap = n
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        // new_data = heap + 12
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        // new_vec.data_ptr = new_data
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // new_len = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(7));
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        // Loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // val = src_data[i]
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // val
        // if fn(val) != 0, push
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // val again for fn call
        f.instruction(&Instruction::LocalGet(1)); // fn table index
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // new_data[new_len] = val (val is on stack from earlier load)
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        // reload val
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));
        // new_len++
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::End); // end if
        // drop the val that was loaded before the if
        f.instruction(&Instruction::Drop);
        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block
        // update new_vec.len = new_len
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Store(ma));
        // return new_vec
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __fold_i32(vec_ptr: i32, init: i32, fn_idx: i32) -> i32
    pub(super) fn build_fold_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=init, 2=fn_idx
        // locals: 3=i, 4=n, 5=src_data, 6=acc
        let mut f = Function::new(vec![(4, ValType::I32)]);
        // n = vec.len
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        // src_data = vec.data_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(5));
        // acc = init
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalSet(6));
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3));
        // Loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // acc = call_indirect(fn_idx, acc, src_data[i])
        f.instruction(&Instruction::LocalGet(6)); // acc
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // src_data[i]
        f.instruction(&Instruction::LocalGet(2)); // fn table index
        let ty_i32x2_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32; 2], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32x2_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::LocalSet(6));
        // i++
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        // return acc
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::End);
        f
    }

    /// __map_i64(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    /// Same as map_i32 but elements are 8-byte i64 values.
    pub(super) fn build_map_i64(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma8 = MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx; locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data
        let mut f = Function::new(vec![(5, ValType::I32)]);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*8
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // &new_data[i]
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        // src_data[i] as i64
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I64Load(ma8));
        f.instruction(&Instruction::LocalGet(1));
        let ty_i64_i64 =
            self.lookup_or_register_indirect_type(vec![ValType::I64], vec![ValType::I64]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i64_i64,
            table_index: 0,
        });
        f.instruction(&Instruction::I64Store(ma8));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __filter_i64(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    pub(super) fn build_filter_i64(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma8 = MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx; locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data, 7=new_len
        let mut f = Function::new(vec![(6, ValType::I32)]);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*8
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // Load val (kept on stack for drop after if)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I64Load(ma8));
        // call predicate: fn(val) -> i32 (bool)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I64Load(ma8));
        f.instruction(&Instruction::LocalGet(1));
        let ty_i64_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I64], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i64_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // store val at new_data[new_len]
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I64Load(ma8));
        f.instruction(&Instruction::I64Store(ma8));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::End); // end if
        f.instruction(&Instruction::Drop); // drop val from before if
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __fold_i64(vec_ptr: i32, init: i64, fn_idx: i32) -> i64
    pub(super) fn build_fold_i64(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma8 = MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        };
        // params: 0=vec_ptr(i32), 1=init(i64), 2=fn_idx(i32)
        // locals: 3=i(i32), 4=n(i32), 5=src_data(i32), 6=acc(i64)
        let mut f = Function::new(vec![(3, ValType::I32), (1, ValType::I64)]);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(5));
        // acc = init (already i64)
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // acc = fn(acc, src_data[i])
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I64Load(ma8));
        f.instruction(&Instruction::LocalGet(2));
        let ty_i64x2_i64 =
            self.lookup_or_register_indirect_type(vec![ValType::I64; 2], vec![ValType::I64]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i64x2_i64,
            table_index: 0,
        });
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        // return acc (i64)
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::End);
        f
    }

    /// __map_f64(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    pub(super) fn build_map_f64(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma8 = MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx; locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data
        let mut f = Function::new(vec![(5, ValType::I32)]);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*8
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::F64Load(ma8));
        f.instruction(&Instruction::LocalGet(1));
        let ty_f64_f64 =
            self.lookup_or_register_indirect_type(vec![ValType::F64], vec![ValType::F64]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_f64_f64,
            table_index: 0,
        });
        f.instruction(&Instruction::F64Store(ma8));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __filter_f64(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    pub(super) fn build_filter_f64(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma8 = MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx; locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data, 7=new_len
        let mut f = Function::new(vec![(6, ValType::I32)]);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*8
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // Load val (kept on stack)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::F64Load(ma8));
        // call predicate: fn(val) -> i32 (bool)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::F64Load(ma8));
        f.instruction(&Instruction::LocalGet(1));
        let ty_f64_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::F64], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_f64_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::F64Load(ma8));
        f.instruction(&Instruction::F64Store(ma8));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::End); // end if
        f.instruction(&Instruction::Drop); // drop val from before if
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __map_option_i32(opt_ptr: i32, fn_idx: i32) -> i32 (new option ptr)
    /// Option layout: [tag: i32, payload: i32] — tag 0 = Some, tag 1 = None
    pub(super) fn build_map_option_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=opt_ptr, 1=fn_idx
        // locals: 2=new_opt
        let mut f = Function::new(vec![(1, ValType::I32)]);
        // Allocate new option: 8 bytes
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // Read tag
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma)); // tag
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // tag != 0 → None: copy tag
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Else);
        // tag == 0 → Some: new_opt.tag = 0, new_opt.payload = fn(old_payload)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        // call fn(payload)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // payload
        f.instruction(&Instruction::LocalGet(1)); // fn table index
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End);
        // return new_opt
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::End);
        f
    }

    pub(super) fn build_any_i32(&self) -> Function {
        // any_i32(vec_ptr: i32, fn_idx: i32) -> i32 (0 or 1)
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // n = mem[vec_ptr] (vec length)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));

        // src_data = mem[vec_ptr + 8] (data pointer)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));

        // loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= n, break (return false)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_data[i*4]
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));

        // call_indirect pred(elem) -> bool
        f.instruction(&Instruction::LocalGet(1)); // fn_idx
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });

        // if result != 0, return 1
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));

        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // return 0 (false)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::End);
        f
    }

    pub(super) fn build_find_i32(&self) -> Function {
        // find_i32(vec_ptr: i32, fn_idx: i32) -> i32 (Option ptr)
        // Returns heap-allocated Option: [tag=0(Some), payload=value] or [tag=1(None)]
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // n = mem[vec_ptr]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));

        // src_data = mem[vec_ptr + 8]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));

        // loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= n, break
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // call_indirect pred(elem) where elem = src_data[i*4]
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalGet(1)); // fn_idx
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });

        // if pred returned true, build Some(elem) and return
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Allocate Some(elem) on heap: [tag=0][payload=elem]
            f.instruction(&Instruction::GlobalGet(0)); // base ptr (will be result)
            f.instruction(&Instruction::GlobalGet(0));
            f.instruction(&Instruction::I32Const(0)); // tag = Some
            f.instruction(&Instruction::I32Store(ma));
            // payload = elem (reload)
            f.instruction(&Instruction::GlobalGet(0));
            f.instruction(&Instruction::I32Const(4));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(4));
            f.instruction(&Instruction::I32Mul);
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
            // bump heap
            f.instruction(&Instruction::GlobalGet(0));
            f.instruction(&Instruction::I32Const(8));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::GlobalSet(0));
            // return base ptr
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));

        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Not found: allocate None: [tag=1]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(1)); // tag = None
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(0)); // padding
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // base ptr on stack
        f.instruction(&Instruction::End);
        f
    }

    // ── HashMap<i32,i32> helpers ────────────────────────────────────────

    /// `__hashmap_i32_new() -> i32` (returns ptr to 12-byte header)
    /// Header layout: [cap:i32, len:i32, buckets_ptr:i32]
    /// Bucket layout: [key:i32, value:i32] × cap  (empty sentinel: key = i32::MIN)
    pub(super) fn build_hashmap_i32_new(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // locals: 0=header_ptr, 1=buckets_ptr, 2=i (loop counter)
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // header_ptr = heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(0));
        // bump heap past header (12 bytes)
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // header[0] = cap = 16
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(16));
        f.instruction(&Instruction::I32Store(ma));
        // header[4] = len = 0
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // buckets_ptr = heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(1));
        // bump heap past buckets: 16 * 8 = 128 bytes
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(128));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // header[8] = buckets_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Store(ma));

        // Fill all 16 bucket keys with i32::MIN (empty sentinel)
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= 16 break
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(16));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // buckets_ptr + i*8 = i32::MIN
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(i32::MIN));
        f.instruction(&Instruction::I32Store(ma));
        // i += 1
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // return header_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::End);
        f
    }

    /// `__hashmap_i32_insert(m: i32, k: i32, v: i32)`
    /// Linear probing with Fibonacci hashing. Rehashes at 75% load.
    pub(super) fn build_hashmap_i32_insert(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=m, 1=k, 2=v
        // locals: 3=cap, 4=buckets_ptr, 5=idx, 6=bucket_addr, 7=bucket_key
        //         8=len, 9=new_cap, 10=new_buckets, 11=old_buckets, 12=i, 13=old_key
        let mut f = Function::new(vec![(11, ValType::I32)]);

        // Load cap = mem[m]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        // Load buckets_ptr = mem[m+8]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));

        // idx = (k * 2654435761) % cap  (Fibonacci hash)
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(0x9E3779B1_u32 as i32)); // 2654435761
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(5));

        // Linear probe loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // bucket_addr = buckets_ptr + idx * 8
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));

        // bucket_key = mem[bucket_addr]
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(7));

        // if bucket_key == k → update existing, branch out
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Update value: mem[bucket_addr+4] = v
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma));
        // Return (no len change)
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End); // end if (key match)

        // if bucket_key == i32::MIN → empty slot, insert here
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(i32::MIN));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Store key
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Store(ma));
        // Store value
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma));
        // Increment len: mem[m+4] += 1
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        // Break out of probe loop
        f.instruction(&Instruction::Br(2));
        f.instruction(&Instruction::End); // end if (empty slot)

        // Collision: idx = (idx + 1) % cap
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Check load factor: if len*4 > cap*3, rehash
        // Load len
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(8));

        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(3));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32GtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        // --- Rehash ---
        // new_cap = cap * 2
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Shl);
        f.instruction(&Instruction::LocalSet(9));
        // Allocate new buckets: new_cap * 8
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(10)); // new_buckets
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Fill new buckets with i32::MIN
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(12));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(i32::MIN));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(12));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Re-insert all entries from old buckets
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalSet(11)); // old_buckets = buckets_ptr
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(12)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::LocalGet(3)); // old cap
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // old_key = old_buckets[i].key
        f.instruction(&Instruction::LocalGet(11));
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(13));

        // if old_key != i32::MIN → re-insert
        f.instruction(&Instruction::LocalGet(13));
        f.instruction(&Instruction::I32Const(i32::MIN));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        // idx = (old_key * FIBONACCI) % new_cap
        f.instruction(&Instruction::LocalGet(13));
        f.instruction(&Instruction::I32Const(0x9E3779B1_u32 as i32));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(5));

        // Probe loop for rehash
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // bucket_addr = new_buckets + idx*8
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        // if new_buckets[idx].key == i32::MIN → place here
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(i32::MIN));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Store key
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(13));
        f.instruction(&Instruction::I32Store(ma));
        // Store value from old bucket
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(11));
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(2)); // break rehash probe
        f.instruction(&Instruction::End); // end if (empty slot)
        // idx = (idx + 1) % new_cap
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::Br(0)); // continue probe
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        f.instruction(&Instruction::End); // end if (old_key != MIN)

        // i += 1
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(12));
        f.instruction(&Instruction::Br(0)); // continue outer loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Update header: cap = new_cap, buckets_ptr = new_buckets
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::End); // end if (rehash needed)

        f.instruction(&Instruction::End);
        f
    }

    /// `__hashmap_i32_get(m: i32, k: i32) -> i32` (returns Option ptr)
    pub(super) fn build_hashmap_i32_get(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=m, 1=k
        // locals: 2=cap, 3=buckets_ptr, 4=idx, 5=bucket_addr, 6=bucket_key, 7=result
        let mut f = Function::new(vec![(6, ValType::I32)]);

        // Load cap
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(2));
        // Load buckets_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));

        // idx = (k * FIBONACCI) % cap
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(0x9E3779B1_u32 as i32));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(4));

        // Probe loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // outer block for "not found"
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // inner block for "found"
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // bucket_addr = buckets_ptr + idx*8
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(5));

        // bucket_key = mem[bucket_addr]
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(6));

        // if bucket_key == i32::MIN → not found
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(i32::MIN));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::BrIf(2)); // branch to "not found" block

        // if bucket_key == k → found
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::BrIf(1)); // branch to "found" block

        // Collision: idx = (idx+1) % cap
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop

        // "found" block exit: allocate Some(value)
        f.instruction(&Instruction::End); // end inner block
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(7));
        // tag = 0 (Some)
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        // payload = mem[bucket_addr+4]
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // return result
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::Return);

        // "not found" block exit: allocate None
        f.instruction(&Instruction::End); // end outer block
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(7));
        // tag = 1 (None)
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        // padding
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        f.instruction(&Instruction::LocalGet(7));

        f.instruction(&Instruction::End);
        f
    }

    /// `__hashmap_i32_contains_key(m: i32, k: i32) -> i32` (0 or 1)
    pub(super) fn build_hashmap_i32_contains(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=m, 1=k
        // locals: 2=cap, 3=buckets_ptr, 4=idx, 5=bucket_key
        let mut f = Function::new(vec![(4, ValType::I32)]);

        // Load cap
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(2));
        // Load buckets_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));

        // idx = (k * FIBONACCI) % cap
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(0x9E3779B1_u32 as i32));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(4));

        // Probe loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // not found
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // found
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // bucket_key = mem[buckets_ptr + idx*8]
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(5));

        // if bucket_key == i32::MIN → not found
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(i32::MIN));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::BrIf(2));

        // if bucket_key == k → found
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::BrIf(1));

        // idx = (idx+1) % cap
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop

        // found
        f.instruction(&Instruction::End); // end found block
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Return);

        // not found
        f.instruction(&Instruction::End); // end not-found block
        f.instruction(&Instruction::I32Const(0));

        f.instruction(&Instruction::End);
        f
    }

    /// `__hashmap_i32_len(m: i32) -> i32`
    pub(super) fn build_hashmap_i32_len(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=m
        let mut f = Function::new(vec![]);
        // return mem[m+4]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::End);
        f
    }

    /// Build arg_count() → i32: calls args_sizes_get, returns argc-1 (skip argv[0]).
    pub(super) fn build_arg_count(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![]);
        // args_sizes_get(SCRATCH, SCRATCH+4)
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        self.call_fn(&mut f, FN_ARGS_SIZES_GET);
        f.instruction(&Instruction::Drop);
        // return argc - 1
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::End);
        f
    }

    /// Build arg_at(i: i32) → i32 (T1 string ptr): returns the i-th user arg (0-based, skip argv[0]).
    /// Uses WASI args_sizes_get + args_get, then builds a length-prefixed string on the heap.
    pub(super) fn build_arg_at(&self) -> Function {
        // Param 0: i (user 0-based index)
        // Locals 1..8: argc, buf_size, argv_ptrs_start, argv_buf_start, raw_ptr, str_len, byte_j, str_data_ptr
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
        let mut f = Function::new(vec![(8, ValType::I32)]);

        // argc = args_sizes_get(SCRATCH, SCRATCH+4) → SCRATCH
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        self.call_fn(&mut f, FN_ARGS_SIZES_GET);
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(1)); // argc
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(2)); // buf_size

        // argv_ptrs_start = heap_ptr; heap_ptr += argc * 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // argv_buf_start = heap_ptr; heap_ptr += buf_size
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Align heap_ptr to 4 bytes after variable-length buf
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(3));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(-4i32));
        f.instruction(&Instruction::I32And);
        f.instruction(&Instruction::GlobalSet(0));

        // args_get(argv_ptrs_start, argv_buf_start)
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(4));
        self.call_fn(&mut f, FN_ARGS_GET);
        f.instruction(&Instruction::Drop);

        // raw_ptr = *(argv_ptrs_start + (i+1)*4)
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(5)); // raw_ptr

        // strlen loop: str_len = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // exit block (found null)
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6)); // str_len++
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Align heap_ptr to 4 bytes before string allocation
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(3));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(-4i32));
        f.instruction(&Instruction::I32And);
        f.instruction(&Instruction::GlobalSet(0));

        // str_data_ptr = heap_ptr + 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(8));

        // Write length prefix at heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));

        // Copy bytes: byte_j = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // exit block
        // *(str_data_ptr + byte_j) = *(raw_ptr + byte_j)
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // heap_ptr += 4 + str_len
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::LocalGet(8)); // return str_data_ptr
        f.instruction(&Instruction::End);
        f
    }

    /// Build args() → i32 (Vec<String> ptr): builds a Vec<String> of all user args (skip argv[0]).
    /// Layout: Vec header {len, cap, data_ptr}, then string ptrs, then length-prefixed string data.
    pub(super) fn build_args_vec(&self) -> Function {
        // Locals 0..10: argc, buf_size, argv_ptrs_start, argv_buf_start, vec_ptr, vec_data_ptr,
        //               i, raw_ptr, str_len, byte_j, str_data_ptr
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
        let mut f = Function::new(vec![(11, ValType::I32)]);

        // args_sizes_get(SCRATCH, SCRATCH+4)
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        self.call_fn(&mut f, FN_ARGS_SIZES_GET);
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(0)); // argc
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(1)); // buf_size

        // argv_ptrs_start = heap_ptr; heap_ptr += argc * 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // argv_buf_start = heap_ptr; heap_ptr += buf_size
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Align heap_ptr to 4 bytes after variable-length buf
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(3));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(-4i32));
        f.instruction(&Instruction::I32And);
        f.instruction(&Instruction::GlobalSet(0));

        // args_get(argv_ptrs_start, argv_buf_start)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        self.call_fn(&mut f, FN_ARGS_GET);
        f.instruction(&Instruction::Drop);

        // vec_ptr = heap_ptr; heap_ptr += 12 (Vec header)
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // vec_data_ptr = heap_ptr; heap_ptr += (argc-1) * 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Write Vec header: {len=argc-1, cap=argc-1, data_ptr=vec_data_ptr}
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Store(ma)); // len
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Store(ma)); // cap
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Store(ma)); // data_ptr

        // Loop i = 1..argc (skip argv[0])
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(6)); // i = 1
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= argc, break
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // raw_ptr = *(argv_ptrs_start + i*4)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(7));

        // Align heap_ptr to 4 bytes before string allocation
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(3));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(-4i32));
        f.instruction(&Instruction::I32And);
        f.instruction(&Instruction::GlobalSet(0));

        // strlen: str_len = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(8));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(8));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // str_data_ptr = heap_ptr + 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(10));

        // Write length prefix at heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Store(ma));

        // Copy bytes: byte_j = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(9));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(9));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // heap_ptr += 4 + str_len
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // vec_data_ptr[(i-1)*4] = str_data_ptr
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::I32Store(ma));

        // i++
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Align heap_ptr to 4 bytes after all string allocations
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(3));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(-4i32));
        f.instruction(&Instruction::I32And);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::LocalGet(4)); // return vec_ptr
        f.instruction(&Instruction::End);
        f
    }
}
