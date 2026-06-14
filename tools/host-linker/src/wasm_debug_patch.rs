//! Post-link Wasm debug preparation: source maps + breakpoint hook injection (#638).

use crate::source_map::{line_to_code_offset, SourceMapEntry};
use std::path::PathBuf;
use walrus::{FunctionId, FunctionKind, LocalId, Module, ValType};

pub fn prepare_debug_wasm(
    raw: &[u8],
    ark_source: &str,
    breakpoint_line: u32,
) -> Result<Vec<u8>, String> {
    let entries = synthesize_source_map(raw, ark_source)?;
    if line_to_code_offset(&entries, breakpoint_line).is_none() {
        return Err(format!("no source-map entry for line {}", breakpoint_line));
    }
    let mut module = Module::from_buffer(raw).map_err(|e| format!("walrus parse: {}", e))?;
    inject_breakpoint_hook(&mut module, breakpoint_line)?;
    let mut out = module.emit_wasm();
    append_source_map_custom_section(&mut out, &entries);
    Ok(out)
}

fn synthesize_source_map(raw: &[u8], ark_source: &str) -> Result<Vec<SourceMapEntry>, String> {
    let module = Module::from_buffer(raw).map_err(|e| format!("walrus parse: {}", e))?;
    let start = find_start_function(&module)?;
    let meaningful_lines: Vec<u32> = ark_source
        .lines()
        .enumerate()
        .filter_map(|(idx, text)| {
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                None
            } else {
                Some((idx + 1) as u32)
            }
        })
        .collect();
    if meaningful_lines.is_empty() {
        return Ok(Vec::new());
    }

    let call_sites = call_offsets(&module, start);
    let mut entries = Vec::new();
    if call_sites.is_empty() {
        entries.push(SourceMapEntry {
            code_offset: 0,
            source_line: meaningful_lines[0],
        });
    } else {
        for (i, site) in call_sites.iter().enumerate() {
            let line_no = meaningful_lines
                .get(i + 1)
                .or_else(|| meaningful_lines.last())
                .copied()
                .unwrap_or(meaningful_lines[0]);
            entries.push(SourceMapEntry {
                code_offset: *site,
                source_line: line_no,
            });
        }
        entries.push(SourceMapEntry {
            code_offset: call_sites[0],
            source_line: meaningful_lines[0],
        });
    }
    entries.sort_by_key(|e| e.code_offset);
    entries.dedup_by_key(|e| e.code_offset);
    Ok(entries)
}

fn call_offsets(module: &Module, start: FunctionId) -> Vec<u32> {
    let func = module.funcs.get(start);
    let FunctionKind::Local(local) = &func.kind else {
        return Vec::new();
    };
    let entry = local.entry_block();
    let mut offsets = Vec::new();
    for (pos, (ins, _loc)) in local.block(entry).instrs.iter().enumerate() {
        if ins.is_call() {
            offsets.push(pos as u32);
        }
    }
    offsets
}

fn find_start_function(module: &Module) -> Result<FunctionId, String> {
    module
        .exports
        .get_func("_start")
        .map_err(|e| format!("missing _start export: {}", e))
}

fn inject_breakpoint_hook(module: &mut Module, breakpoint_line: u32) -> Result<(), String> {
    let ty = module.types.add(&[ValType::I32, ValType::I32], &[]);
    let (hook_func, _import) = module.add_import_func("arukellt_debug", "breakpoint", ty);
    let start = find_start_function(module)?;
    let local_x = find_first_i32_local(module, start)?;
    patch_start_with_hook(module, start, hook_func, local_x, breakpoint_line)
}

fn find_first_i32_local(module: &Module, func: FunctionId) -> Result<LocalId, String> {
    let f = module.funcs.get(func);
    let FunctionKind::Local(local) = &f.kind else {
        return Err("start is not a local function".into());
    };
    let entry = local.entry_block();
    for (ins, _loc) in &local.block(entry).instrs {
        if let walrus::ir::Instr::LocalGet(g) = ins {
            return Ok(g.local);
        }
    }
    local
        .args
        .first()
        .copied()
        .ok_or_else(|| "no i32 local".to_string())
}

fn patch_start_with_hook(
    module: &mut Module,
    start: FunctionId,
    hook_func: FunctionId,
    local_x: LocalId,
    breakpoint_line: u32,
) -> Result<(), String> {
    let original = {
        let func = module.funcs.get(start);
        let FunctionKind::Local(local) = &func.kind else {
            return Err("start is not a local function".into());
        };
        let entry = local.entry_block();
        local
            .block(entry)
            .instrs
            .iter()
            .map(|(ins, loc)| (ins.clone(), *loc))
            .collect::<Vec<_>>()
    };
    let mut insert_at = original.len();
    for (idx, (ins, _loc)) in original.iter().enumerate().rev() {
        if ins.is_call() {
            insert_at = idx;
            break;
        }
    }
    let hook = vec![
        (
            walrus::ir::Instr::Const(walrus::ir::Const {
                value: walrus::ir::Value::I32(breakpoint_line as i32),
            }),
            original.first().map(|(_, loc)| *loc).unwrap_or_default(),
        ),
        (
            walrus::ir::Instr::LocalGet(walrus::ir::LocalGet { local: local_x }),
            original.first().map(|(_, loc)| *loc).unwrap_or_default(),
        ),
        (
            walrus::ir::Instr::Call(walrus::ir::Call { func: hook_func }),
            original.first().map(|(_, loc)| *loc).unwrap_or_default(),
        ),
    ];
    let mut rebuilt = original;
    rebuilt.splice(insert_at..insert_at, hook);
    module
        .replace_exported_func(start, |(body, _locals)| {
            for (ins, _loc) in rebuilt {
                body.instr(ins);
            }
        })
        .map_err(|e| format!("replace _start: {}", e))?;
    Ok(())
}

fn append_source_map_custom_section(out: &mut Vec<u8>, entries: &[SourceMapEntry]) {
    if entries.is_empty() {
        return;
    }
    let mut payload = Vec::new();
    leb128_append(&mut payload, 1);
    leb128_append(&mut payload, entries.len() as u32);
    for entry in entries {
        leb128_append(&mut payload, entry.code_offset);
        leb128_append(&mut payload, entry.source_line);
    }
    let name = b"metadata.debug.source_map";
    let mut name_enc = Vec::new();
    leb128_append(&mut name_enc, name.len() as u32);
    name_enc.extend_from_slice(name);
    let body_len = name_enc.len() + payload.len();
    out.push(0);
    leb128_append(out, body_len as u32);
    out.extend_from_slice(&name_enc);
    out.extend_from_slice(&payload);
}

#[cfg(test)]
mod tests {
    use super::*;
    use walrus::Module;

    #[test]
    fn round_trip_smoke_fixture() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let wasm_path = repo.join("tests/fixtures/selfhost/debug_smoke.wasm");
        let ark_path = repo.join("tests/fixtures/selfhost/debug_smoke.ark");
        let raw = std::fs::read(&wasm_path).expect("read wasm");
        let source = std::fs::read_to_string(&ark_path).expect("read ark");
        Module::from_buffer(&raw).expect("parse input");
        let patched = prepare_debug_wasm(&raw, &source, 3).expect("patch");
        assert!(patched.len() > raw.len());
        assert!(patched.windows(25).any(|w| w == b"metadata.debug.source_map"));
        Module::from_buffer(&patched).expect("re-parse patched");
    }
}

fn leb128_append(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}
