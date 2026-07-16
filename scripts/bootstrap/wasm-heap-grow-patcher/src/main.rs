use std::collections::HashSet;
use std::env;
use walrus::{
    FunctionBuilder, FunctionId, GlobalId, GlobalKind, LocalFunction, MemoryId, Module,
    ModuleLocals, ModuleTypes, ValType,
};
use walrus::ir::{
    Binop, BinaryOp, Const, Drop, GlobalGet, GlobalSet, Instr, InstrSeqId, LocalGet, LocalSet,
    LocalTee, MemArg, Return, Select, Store, StoreKind, Value,
};

fn read_leb_u32(data: &[u8], mut offset: usize) -> Option<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0u32;
    while offset < data.len() {
        let byte = data[offset];
        offset += 1;
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((result, offset));
        }
        shift += 7;
        if shift >= 35 {
            return None;
        }
    }
    None
}

fn write_leb_u32(mut value: u32) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

/// Rewrite export section bytes, keeping the first export for each name.
fn dedupe_export_section_raw(data: Vec<u8>) -> (Vec<u8>, usize) {
    if data.len() < 8 || &data[0..4] != b"\0asm" {
        return (data, 0);
    }
    let mut offset = 8usize;
    let mut sections: Vec<(u8, usize, usize)> = Vec::new();
    while offset < data.len() {
        let section_id = data[offset];
        offset += 1;
        let Some((size, payload_start)) = read_leb_u32(&data, offset) else {
            return (data, 0);
        };
        let payload_end = payload_start + size as usize;
        if payload_end > data.len() {
            return (data, 0);
        }
        sections.push((section_id, payload_start, payload_end));
        offset = payload_end;
    }

    let mut rebuilt = Vec::with_capacity(data.len());
    rebuilt.extend_from_slice(&data[0..8]);
    let mut removed = 0usize;

    for (section_id, start, end) in sections {
        let payload = &data[start..end];
        if section_id != 7 {
            rebuilt.push(section_id);
            rebuilt.extend(write_leb_u32(payload.len() as u32));
            rebuilt.extend_from_slice(payload);
            continue;
        }

        let Some((count, mut pos)) = read_leb_u32(payload, 0) else {
            rebuilt.push(section_id);
            rebuilt.extend(write_leb_u32(payload.len() as u32));
            rebuilt.extend_from_slice(payload);
            continue;
        };

        let mut seen: HashSet<Vec<u8>> = HashSet::new();
        let mut export_entries: Vec<Vec<u8>> = Vec::new();
        let mut i = 0u32;
        while i < count {
            let Some((name_len, name_start)) = read_leb_u32(payload, pos) else {
                break;
            };
            let name_end = name_start + name_len as usize;
            if name_end > payload.len() {
                break;
            }
            let name = payload[name_start..name_end].to_vec();
            pos = name_end;
            if pos >= payload.len() {
                break;
            }
            let kind = payload[pos];
            pos += 1;
            let Some((index, index_end)) = read_leb_u32(payload, pos) else {
                break;
            };
            pos = index_end;
            let mut entry = write_leb_u32(name.len() as u32);
            entry.extend_from_slice(&name);
            entry.push(kind);
            entry.extend(write_leb_u32(index));
            if seen.insert(name) {
                export_entries.push(entry);
            } else {
                removed += 1;
            }
            i += 1;
        }

        let mut new_payload = write_leb_u32(export_entries.len() as u32);
        for entry in export_entries {
            new_payload.extend(entry);
        }
        rebuilt.push(section_id);
        rebuilt.extend(write_leb_u32(new_payload.len() as u32));
        rebuilt.extend(new_payload);
    }

    (rebuilt, removed)
}

fn load_module(path: &str) -> Module {
    let bytes = std::fs::read(path).expect("read wasm");
    let (deduped, removed) = dedupe_export_section_raw(bytes);
    if removed > 0 {
        eprintln!("raw export dedupe removed {} duplicate names", removed);
    }
    Module::from_buffer(&deduped).expect("parse wasm")
}

fn stack_top_is_i32(instrs: &[(Instr, walrus::ir::InstrLocId)]) -> bool {
    let Some((instr, _)) = instrs.last() else {
        return false;
    };
    match instr {
        Instr::Const(c) => matches!(c.value, Value::I32(_)),
        Instr::Binop(_) | Instr::Unop(_) | Instr::LocalGet(_) | Instr::LocalTee(_) => true,
        Instr::Call(_) | Instr::Load(_) | Instr::MemorySize(_) => true,
        Instr::GlobalGet(_) => true,
        _ => false,
    }
}

fn recent_heap_get(instrs: &[(Instr, walrus::ir::InstrLocId)], heap_global: GlobalId) -> bool {
    let start = instrs.len().saturating_sub(12);
    for (instr, _) in &instrs[start..] {
        if let Instr::GlobalGet(GlobalGet { global }) = instr {
            if *global == heap_global {
                return true;
            }
        }
    }
    false
}

fn patch_instr_seq(
    func: &mut LocalFunction,
    seq_id: InstrSeqId,
    heap_global: GlobalId,
    grow_fn: FunctionId,
    patched: &mut usize,
) {
    let instrs: Vec<(Instr, walrus::ir::InstrLocId)> =
        std::mem::take(&mut func.block_mut(seq_id).instrs);
    let mut out = Vec::with_capacity(instrs.len() + 8);
    for (instr, loc) in instrs {
        if let Instr::Block(ref b) = instr {
            patch_instr_seq(func, b.seq, heap_global, grow_fn, patched);
        } else if let Instr::Loop(ref l) = instr {
            patch_instr_seq(func, l.seq, heap_global, grow_fn, patched);
        } else if let Instr::IfElse(ref ie) = instr {
            patch_instr_seq(func, ie.consequent, heap_global, grow_fn, patched);
            patch_instr_seq(func, ie.alternative, heap_global, grow_fn, patched);
        }
        if let Instr::GlobalSet(ref gs) = instr {
            if gs.global == heap_global
                && recent_heap_get(&out, heap_global)
                && stack_top_is_i32(&out)
            {
                out.push((Instr::Call(walrus::ir::Call { func: grow_fn }), loc));
                *patched += 1;
            }
        }
        out.push((instr, loc));
    }
    func.block_mut(seq_id).instrs = out;
}

fn is_vec_new_header_bump(instrs: &[(Instr, walrus::ir::InstrLocId)], heap_global: GlobalId) -> bool {
    // Look for the classic __intrinsic_Vec_new prologue:
    //   global.get 0; local.set _;
    //   global.get 0; i32.const 52; i32.add; global.set 0
    if instrs.len() < 6 {
        return false;
    }
    let p = instrs;
    matches!(&p[0].0, Instr::GlobalGet(GlobalGet { global }) if *global == heap_global)
        && matches!(&p[2].0, Instr::GlobalGet(GlobalGet { global }) if *global == heap_global)
        && matches!(&p[3].0, Instr::Const(Const { value: Value::I32(52) }))
        && matches!(&p[4].0, Instr::Binop(Binop { op: BinaryOp::I32Add }))
        && matches!(&p[5].0, Instr::GlobalSet(GlobalSet { global }) if *global == heap_global)
}

fn is_vec_new_function_name(name: Option<&str>) -> bool {
    match name {
        Some(n) if n.starts_with("Vec_new") => true,
        Some(n) if n.starts_with("__intrinsic_vec_new") => true,
        _ => false,
    }
}

fn patch_vec_new(
    func: &mut LocalFunction,
    name: Option<&str>,
    locals: &mut ModuleLocals,
    types: &ModuleTypes,
    heap_global: GlobalId,
    memory_id: MemoryId,
    patched: &mut usize,
) {
    let ty = func.ty();
    if types.results(ty) != [ValType::I32] {
        return;
    }
    if !is_vec_new_function_name(name) {
        return;
    }
    let entry = func.entry_block();
    let instrs = &func.block(entry).instrs;
    if !is_vec_new_header_bump(instrs, heap_global) {
        return;
    }

    // Distinguish the generic Vec_new (three i32 args: size, len, cap)
    // from the specialized monomorphic Vec_new variants (len, cap) which
    // omit the size field and use a 12-byte header.
    let params = types.params(ty);
    let (is_generic, _header_size, total_size, len_arg, cap_arg, size_arg) = match params {
        [ValType::I32, ValType::I32, ValType::I32] => {
            (true, 16u32, 24u32, func.args[1], func.args[2], Some(func.args[0]))
        }
        [ValType::I32, ValType::I32] => {
            (false, 12u32, 20u32, func.args[0], func.args[1], None)
        }
        [ValType::I32, ValType::F64, ValType::I32] => {
            (false, 12u32, 20u32, func.args[0], func.args[2], None)
        }
        _ => return,
    };

    let old_result = locals.add(ValType::I32);
    let result = locals.add(ValType::I32);
    let new_end = locals.add(ValType::I32);
    let data = locals.add(ValType::I32);
    let loc = walrus::ir::InstrLocId::default();
    let store = |offset: u32| Store {
        memory: memory_id,
        kind: StoreKind::I32 { atomic: false },
        arg: MemArg { align: 2, offset },
    };

    let mut new_instrs: Vec<(Instr, walrus::ir::InstrLocId)> = Vec::new();

    // old_result = result = current heap pointer
    new_instrs.push((Instr::GlobalGet(GlobalGet { global: heap_global }), loc));
    new_instrs.push((Instr::LocalTee(LocalTee { local: old_result }), loc));
    new_instrs.push((Instr::LocalSet(LocalSet { local: result }), loc));

    // new_end = old_result + total_size (may wrap around 2^32)
    new_instrs.push((Instr::LocalGet(LocalGet { local: old_result }), loc));
    new_instrs.push((Instr::Const(Const { value: Value::I32(total_size as i32) }), loc));
    new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Add }), loc));
    new_instrs.push((Instr::LocalSet(LocalSet { local: new_end }), loc));

    // result = (new_end < old_result) ? 0 : old_result
    new_instrs.push((Instr::Const(Const { value: Value::I32(0) }), loc));
    new_instrs.push((Instr::LocalGet(LocalGet { local: old_result }), loc));
    new_instrs.push((Instr::LocalGet(LocalGet { local: new_end }), loc));
    new_instrs.push((Instr::LocalGet(LocalGet { local: old_result }), loc));
    new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32LtU }), loc));
    new_instrs.push((Instr::Select(Select { ty: Some(ValType::I32) }), loc));
    new_instrs.push((Instr::LocalSet(LocalSet { local: result }), loc));

    // new_end = (new_end < old_result) ? total_size : new_end
    new_instrs.push((Instr::Const(Const { value: Value::I32(total_size as i32) }), loc));
    new_instrs.push((Instr::LocalGet(LocalGet { local: new_end }), loc));
    new_instrs.push((Instr::LocalGet(LocalGet { local: new_end }), loc));
    new_instrs.push((Instr::LocalGet(LocalGet { local: old_result }), loc));
    new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32LtU }), loc));
    new_instrs.push((Instr::Select(Select { ty: Some(ValType::I32) }), loc));
    new_instrs.push((Instr::LocalSet(LocalSet { local: new_end }), loc));

    // global.get 0 (to satisfy heap-set pre-grow detector) and advance the heap
    new_instrs.push((Instr::GlobalGet(GlobalGet { global: heap_global }), loc));
    new_instrs.push((Instr::LocalGet(LocalGet { local: new_end }), loc));
    new_instrs.push((Instr::GlobalSet(GlobalSet { global: heap_global }), loc));
    new_instrs.push((Instr::Drop(Drop {}), loc));

    // data = global.get 0 - 8 (the inline data slot just allocated)
    new_instrs.push((Instr::GlobalGet(GlobalGet { global: heap_global }), loc));
    new_instrs.push((Instr::Const(Const { value: Value::I32(8) }), loc));
    new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Sub }), loc));
    new_instrs.push((Instr::LocalSet(LocalSet { local: data }), loc));

    // data[0] = 0
    new_instrs.push((Instr::LocalGet(LocalGet { local: data }), loc));
    new_instrs.push((Instr::Const(Const { value: Value::I32(0) }), loc));
    new_instrs.push((Instr::Store(store(0)), loc));
    // data[4] = 0
    new_instrs.push((Instr::LocalGet(LocalGet { local: data }), loc));
    new_instrs.push((Instr::Const(Const { value: Value::I32(4) }), loc));
    new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Add }), loc));
    new_instrs.push((Instr::Const(Const { value: Value::I32(0) }), loc));
    new_instrs.push((Instr::Store(store(0)), loc));

    if is_generic {
        let size = size_arg.unwrap();
        // result[0] = size
        new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
        new_instrs.push((Instr::LocalGet(LocalGet { local: size }), loc));
        new_instrs.push((Instr::Store(store(0)), loc));
        // result[4] = cap
        new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
        new_instrs.push((Instr::Const(Const { value: Value::I32(4) }), loc));
        new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Add }), loc));
        new_instrs.push((Instr::LocalGet(LocalGet { local: cap_arg }), loc));
        new_instrs.push((Instr::Store(store(0)), loc));
        // result[12] = len
        new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
        new_instrs.push((Instr::Const(Const { value: Value::I32(12) }), loc));
        new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Add }), loc));
        new_instrs.push((Instr::LocalGet(LocalGet { local: len_arg }), loc));
        new_instrs.push((Instr::Store(store(0)), loc));
        // result[8] = data
        new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
        new_instrs.push((Instr::Const(Const { value: Value::I32(8) }), loc));
        new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Add }), loc));
        new_instrs.push((Instr::LocalGet(LocalGet { local: data }), loc));
        new_instrs.push((Instr::Store(store(0)), loc));
    } else {
        // result[0] = len
        new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
        new_instrs.push((Instr::LocalGet(LocalGet { local: len_arg }), loc));
        new_instrs.push((Instr::Store(store(0)), loc));
        // result[4] = cap
        new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
        new_instrs.push((Instr::Const(Const { value: Value::I32(4) }), loc));
        new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Add }), loc));
        new_instrs.push((Instr::LocalGet(LocalGet { local: cap_arg }), loc));
        new_instrs.push((Instr::Store(store(0)), loc));
        // result[8] = data
        new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
        new_instrs.push((Instr::Const(Const { value: Value::I32(8) }), loc));
        new_instrs.push((Instr::Binop(Binop { op: BinaryOp::I32Add }), loc));
        new_instrs.push((Instr::LocalGet(LocalGet { local: data }), loc));
        new_instrs.push((Instr::Store(store(0)), loc));
    }

    // return result
    new_instrs.push((Instr::LocalGet(LocalGet { local: result }), loc));
    new_instrs.push((Instr::Return(Return {}), loc));

    func.block_mut(entry).instrs = new_instrs;
    *patched += 1;
}

mod wasm32to64;

/// Rewrite the first memory's initial page count in a memory64 module.
fn set_memory64_initial_pages(data: &[u8], initial_pages: u64) -> Result<Vec<u8>, String> {
    if data.len() < 8 || &data[0..4] != b"\0asm" {
        return Err("not a wasm module".into());
    }
    let mut offset = 8usize;
    let mut sections: Vec<(u8, usize, usize)> = Vec::new();
    while offset < data.len() {
        let section_id = data[offset];
        offset += 1;
        let Some((size, payload_start)) = read_leb_u32(data, offset) else {
            return Err("bad section size".into());
        };
        let payload_end = payload_start + size as usize;
        if payload_end > data.len() {
            return Err("section overrun".into());
        }
        sections.push((section_id, payload_start, payload_end));
        offset = payload_end;
    }

    let mut rebuilt = Vec::with_capacity(data.len() + 16);
    rebuilt.extend_from_slice(&data[0..8]);
    let mut saw_memory = false;
    for (section_id, start, end) in sections {
        let payload = &data[start..end];
        if section_id != 5 {
            rebuilt.push(section_id);
            rebuilt.extend(write_leb_u32(payload.len() as u32));
            rebuilt.extend_from_slice(payload);
            continue;
        }
        saw_memory = true;
        // memory section: count, flags(memory64), min, [max]
        let Some((count, mut pos)) = read_leb_u32(payload, 0) else {
            return Err("bad memory count".into());
        };
        if count == 0 {
            return Err("empty memory section".into());
        }
        let flags = payload[pos];
        pos += 1;
        if flags & 0x04 == 0 {
            return Err("memory is not memory64".into());
        }
        // skip old min (and max if present)
        let Some((_, after_min)) = read_leb_u64(payload, pos) else {
            return Err("bad memory min".into());
        };
        pos = after_min;
        if flags & 0x01 != 0 {
            let Some((_, after_max)) = read_leb_u64(payload, pos) else {
                return Err("bad memory max".into());
            };
            pos = after_max;
        }
        let rest = &payload[pos..];
        // flags: memory64, no max
        let mut new_payload = write_leb_u32(count);
        new_payload.push(0x04);
        new_payload.extend(write_leb_u64(initial_pages));
        // keep any additional memories raw (rare)
        new_payload.extend_from_slice(rest);
        rebuilt.push(5);
        rebuilt.extend(write_leb_u32(new_payload.len() as u32));
        rebuilt.extend(new_payload);
    }
    if !saw_memory {
        return Err("no memory section".into());
    }
    Ok(rebuilt)
}

fn read_leb_u64(data: &[u8], mut offset: usize) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0u32;
    while offset < data.len() {
        let byte = data[offset];
        offset += 1;
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((result, offset));
        }
        shift += 7;
        if shift >= 70 {
            return None;
        }
    }
    None
}

fn write_leb_u64(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

fn main() {
    let path = env::args().nth(1).expect("input wasm");
    let out = env::args().nth(2).expect("output wasm");
    let extra: Vec<String> = env::args().skip(3).collect();
    let dedupe_only = extra.iter().any(|a| a == "--dedupe-exports");
    let to_memory64 = extra.iter().any(|a| a == "--to-memory64");
    // Convert without walrus heap-grow injection. Prefer this when the grow-site
    // heuristic false-positives on newer selfhost compilers; pair with a large
    // memory64 initial size so the bump allocator never needs memory.grow.
    let convert_only = extra.iter().any(|a| a == "--convert-only");
    let initial_pages: Option<u64> = extra.iter().find_map(|a| {
        a.strip_prefix("--initial-pages=")
            .and_then(|v| v.parse::<u64>().ok())
    });

    if dedupe_only {
        let mut module = load_module(&path);
        let removed = dedupe_export_names(&mut module);
        module.emit_wasm_file(&out).expect("write wasm");
        eprintln!("removed {} duplicate exports (no GC); wrote {}", removed, out);
        return;
    }

    if convert_only {
        let bytes = std::fs::read(&path).expect("read wasm");
        match wasm32to64::convert_to_memory64(&bytes) {
            Ok(mut converted) => {
                if let Some(pages) = initial_pages {
                    converted = set_memory64_initial_pages(&converted, pages)
                        .expect("set memory64 initial pages");
                }
                std::fs::write(&out, &converted).expect("write memory64 wasm");
                eprintln!(
                    "converted to memory64 (no heap-grow patch){}; wrote {}",
                    initial_pages
                        .map(|p| format!(", initial_pages={p}"))
                        .unwrap_or_default(),
                    out
                );
            }
            Err(err) => {
                eprintln!("memory64 conversion failed: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    // Heap-grow patch while still memory32, then optionally widen to memory64
    // so the bump allocator can pass the wasm32 4GiB ceiling (#730).
    let mut module = load_module(&path);

    for mem in module.memories.iter_mut() {
        mem.initial = 65535;
        mem.maximum = Some(65536);
    }

    let heap_global = module
        .globals
        .iter()
        .find(|g| matches!(g.kind, GlobalKind::Local(_)))
        .map(|g| g.id())
        .expect("heap global");

    let memory_id = module.memories.iter().next().map(|m| m.id()).expect("memory");

    let end = module.locals.add(ValType::I32);
    let mut builder = FunctionBuilder::new(&mut module.types, &[ValType::I32], &[ValType::I32]);
    builder
        .func_body()
        .local_get(end)
        .memory_size(memory_id)
        .i32_const(16)
        .binop(walrus::ir::BinaryOp::I32Shl)
        .binop(walrus::ir::BinaryOp::I32GeU)
        .if_else(
            None,
            |then_| {
                then_
                    .local_get(end)
                    .memory_size(memory_id)
                    .i32_const(16)
                    .binop(walrus::ir::BinaryOp::I32Shl)
                    .binop(walrus::ir::BinaryOp::I32Sub)
                    .i32_const(65536)
                    .binop(walrus::ir::BinaryOp::I32Add)
                    .i32_const(16)
                    .binop(walrus::ir::BinaryOp::I32ShrU)
                    .memory_grow(memory_id)
                    .drop();
            },
            |_| {},
        )
        .local_get(end);
    let grow_fn = builder.finish(vec![end], &mut module.funcs);

    let local_funcs: Vec<(FunctionId, InstrSeqId)> = module
        .funcs
        .iter()
        .filter_map(|f| {
            if f.id() == grow_fn {
                return None;
            }
            match &f.kind {
                walrus::FunctionKind::Local(local) => Some((f.id(), local.entry_block())),
                walrus::FunctionKind::Import(_) | walrus::FunctionKind::Uninitialized(_) => None,
            }
        })
        .collect();

    let mut total_patched = 0usize;
    let funcs = &mut module.funcs;
    let locals = &mut module.locals;
    let types = &module.types;
    for (func_id, entry) in local_funcs {
        let func_ref = funcs.get_mut(func_id);
        let name = func_ref.name.as_deref();
        let func = func_ref.kind.unwrap_local_mut();
        let mut patched = 0usize;
        patch_vec_new(func, name, locals, types, heap_global, memory_id, &mut patched);
        patch_instr_seq(func, entry, heap_global, grow_fn, &mut patched);
        total_patched += patched;
    }

    let _ = dedupe_export_names(&mut module);

    if to_memory64 {
        let tmp = format!("{}.pre64.wasm", out);
        module.emit_wasm_file(&tmp).expect("write pre-memory64 wasm");
        let bytes = std::fs::read(&tmp).expect("read pre-memory64 wasm");
        match wasm32to64::convert_to_memory64(&bytes) {
            Ok(converted) => {
                std::fs::write(&out, &converted).expect("write memory64 wasm");
                let _ = std::fs::remove_file(&tmp);
                eprintln!(
                    "patched {} heap growth sites; converted to memory64; wrote {}",
                    total_patched, out
                );
            }
            Err(err) => {
                let _ = std::fs::remove_file(&tmp);
                eprintln!("memory64 conversion failed: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    module.emit_wasm_file(&out).expect("write wasm");
    eprintln!(
        "patched {} heap growth sites; memory initial=65536; wrote {}",
        total_patched, out
    );
}

fn dedupe_export_names(module: &mut Module) -> usize {
    let mut seen = std::collections::HashSet::new();
    let to_delete: Vec<_> = module
        .exports
        .iter()
        .filter_map(|export| {
            if export.name == "memory" || export.name == "_start" {
                seen.insert(export.name.clone());
                return None;
            }
            if seen.insert(export.name.clone()) {
                None
            } else {
                Some(export.id())
            }
        })
        .collect();
    let removed = to_delete.len();
    for id in to_delete {
        module.exports.delete(id);
    }
    removed
}
