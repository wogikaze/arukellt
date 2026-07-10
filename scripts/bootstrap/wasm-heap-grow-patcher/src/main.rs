mod wasm32to64;

use std::collections::HashSet;
use std::env;
use walrus::{
    FunctionBuilder, FunctionId, GlobalId, GlobalKind, LocalFunction, Module, ValType,
};
use walrus::ir::{GlobalGet, Instr, InstrSeqId, Value};

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

fn main() {
    let path = env::args().nth(1).expect("input wasm");
    let out = env::args().nth(2).expect("output wasm");
    let mode = env::args().nth(3);

    if mode.as_deref() == Some("--memory64") {
        let bytes = std::fs::read(&path).expect("read wasm");
        let converted = wasm32to64::convert_to_memory64(&bytes).expect("convert to memory64");
        std::fs::write(&out, &converted).expect("write wasm");
        eprintln!("converted to memory64; wrote {}", out);
        return;
    }

    let mut module = load_module(&path);

    if mode.as_deref() == Some("--dedupe-exports") {
        let removed = dedupe_export_names(&mut module);
        module.emit_wasm_file(&out).expect("write wasm");
        eprintln!("removed {} duplicate exports (no GC); wrote {}", removed, out);
        return;
    }

    for mem in module.memories.iter_mut() {
        mem.initial = 1;
        mem.maximum = None;
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
        .i32_const(1048576) // headroom: pre-grow 16 pages before reaching boundary
        .binop(walrus::ir::BinaryOp::I32Add)
        .memory_size(memory_id)
        .i32_const(16)
        .binop(walrus::ir::BinaryOp::I32Shl)
        .binop(walrus::ir::BinaryOp::I32GeU)
        .if_else(
            None,
            |then_| {
                then_
                    .local_get(end)
                    .i32_const(1048576) // headroom
                    .binop(walrus::ir::BinaryOp::I32Add)
                    .memory_size(memory_id)
                    .i32_const(16)
                    .binop(walrus::ir::BinaryOp::I32Shl)
                    .binop(walrus::ir::BinaryOp::I32Sub)
                    .i32_const(65535)
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
    for (func_id, entry) in local_funcs {
        let func = module.funcs.get_mut(func_id).kind.unwrap_local_mut();
        let mut patched = 0usize;
        patch_instr_seq(func, entry, heap_global, grow_fn, &mut patched);
        total_patched += patched;
    }

    let _ = dedupe_export_names(&mut module);

    module.emit_wasm_file(&out).expect("write wasm");
    eprintln!(
        "patched {} heap global.set sites; memory initial=1; wrote {}",
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
