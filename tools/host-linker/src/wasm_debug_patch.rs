//! Post-link Wasm debug preparation: source maps + breakpoint hook injection (#638).
//!
//! Implemented on wasmparser + wasm-encoder (via the reencode round-trip
//! helpers) instead of walrus: the selfhost emitter produces Wasm GC types
//! (struct/array rec groups) in the type section, which walrus cannot parse.

use crate::source_map::{line_to_code_offset, SourceMapEntry};
use wasm_encoder::reencode::Reencode;
use wasm_encoder::{
    CodeSection, EntityType, ExportSection, Function, ImportSection, Instruction, Module,
    TypeSection,
};
use wasmparser::{ExternalKind, Operator, Parser, Payload, TypeRef};

pub fn prepare_debug_wasm(
    raw: &[u8],
    ark_source: &str,
    breakpoint_line: u32,
) -> Result<Vec<u8>, String> {
    let info = analyze_module(raw)?;
    let entries = synthesize_source_map(&info, ark_source);
    if line_to_code_offset(&entries, breakpoint_line).is_none() {
        return Err(format!("no source-map entry for line {}", breakpoint_line));
    }
    let mut out = inject_breakpoint_hook(raw, &info, breakpoint_line)?;
    append_source_map_custom_section(&mut out, &entries);
    Ok(out)
}

/// Static facts about the input module needed for patching.
struct ModuleInfo {
    /// Number of imported functions (index-space offset of defined functions).
    imported_funcs: u32,
    /// Number of entries in the type section.
    type_count: u32,
    /// Function index of the `_start` export.
    start_func: u32,
    /// Flat operator positions of `call` instructions in `_start`'s body.
    start_call_offsets: Vec<u32>,
    /// Local index used by the first `local.get` in `_start` (breakpoint value).
    start_first_local_get: Option<u32>,
}

fn analyze_module(raw: &[u8]) -> Result<ModuleInfo, String> {
    let mut imported_funcs: u32 = 0;
    let mut type_count: u32 = 0;
    let mut start_func: Option<u32> = None;
    let mut defined_func_bodies_seen: u32 = 0;
    let mut start_call_offsets: Vec<u32> = Vec::new();
    let mut start_first_local_get: Option<u32> = None;

    // First pass: imports / types / exports.
    for payload in Parser::new(0).parse_all(raw) {
        match payload.map_err(|e| format!("wasm parse: {}", e))? {
            Payload::TypeSection(reader) => {
                type_count = reader.count();
            }
            Payload::ImportSection(reader) => {
                for imports in reader {
                    let imports = imports.map_err(|e| format!("wasm parse: {}", e))?;
                    for import in imports {
                        let (_, import) = import.map_err(|e| format!("wasm parse: {}", e))?;
                        if matches!(import.ty, TypeRef::Func(_)) {
                            imported_funcs += 1;
                        }
                    }
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = export.map_err(|e| format!("wasm parse: {}", e))?;
                    if export.kind == ExternalKind::Func && export.name == "_start" {
                        start_func = Some(export.index);
                    }
                }
            }
            _ => {}
        }
    }
    let start_func = start_func.ok_or_else(|| "missing _start export".to_string())?;
    if start_func < imported_funcs {
        return Err("_start is an imported function".to_string());
    }
    let start_defined_index = start_func - imported_funcs;

    // Second pass: scan `_start`'s body.
    for payload in Parser::new(0).parse_all(raw) {
        if let Payload::CodeSectionEntry(body) = payload.map_err(|e| format!("wasm parse: {}", e))? {
            let this_index = defined_func_bodies_seen;
            defined_func_bodies_seen += 1;
            if this_index != start_defined_index {
                continue;
            }
            let mut ops = body
                .get_operators_reader()
                .map_err(|e| format!("wasm parse: {}", e))?;
            let mut pos: u32 = 0;
            while !ops.eof() {
                let op = ops.read().map_err(|e| format!("wasm parse: {}", e))?;
                match op {
                    Operator::Call { .. } => start_call_offsets.push(pos),
                    Operator::LocalGet { local_index } => {
                        if start_first_local_get.is_none() {
                            start_first_local_get = Some(local_index);
                        }
                    }
                    _ => {}
                }
                pos += 1;
            }
        }
    }

    Ok(ModuleInfo {
        imported_funcs,
        type_count,
        start_func,
        start_call_offsets,
        start_first_local_get,
    })
}

fn synthesize_source_map(info: &ModuleInfo, ark_source: &str) -> Vec<SourceMapEntry> {
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
        return Vec::new();
    }

    let call_sites = &info.start_call_offsets;
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
    entries
}

/// Reencoder that shifts defined-function indices by one so a new function
/// import can be appended after the existing imports.
struct ShiftFuncs {
    imported_funcs: u32,
}

impl Reencode for ShiftFuncs {
    type Error = std::convert::Infallible;

    fn function_index(
        &mut self,
        func: u32,
    ) -> Result<u32, wasm_encoder::reencode::Error<Self::Error>> {
        if func >= self.imported_funcs {
            Ok(func + 1)
        } else {
            Ok(func)
        }
    }
}

fn inject_breakpoint_hook(
    raw: &[u8],
    info: &ModuleInfo,
    breakpoint_line: u32,
) -> Result<Vec<u8>, String> {
    let hook_type_index = info.type_count;
    let hook_func_index = info.imported_funcs;
    let local_x = info
        .start_first_local_get
        .ok_or_else(|| "no local.get in _start (no i32 local)".to_string())?;
    let start_defined_index = info.start_func - info.imported_funcs;

    let mut shift = ShiftFuncs {
        imported_funcs: info.imported_funcs,
    };
    let mut module = Module::new();
    let mut saw_import_section = false;
    let mut defined_func_bodies_seen: u32 = 0;
    let mut pending_code: Option<CodeSection> = None;
    let mut code_entry_count: u32 = 0;

    for payload in Parser::new(0).parse_all(raw) {
        let payload = payload.map_err(|e| format!("wasm parse: {}", e))?;
        match payload {
            Payload::Version { .. } | Payload::End(_) => {}
            Payload::TypeSection(reader) => {
                let mut types = TypeSection::new();
                shift
                    .parse_type_section(&mut types, reader)
                    .map_err(|e| format!("reencode types: {}", e))?;
                types.ty().function([wasm_encoder::ValType::I32; 2], []);
                module.section(&types);
            }
            Payload::ImportSection(reader) => {
                saw_import_section = true;
                let mut imports = ImportSection::new();
                shift
                    .parse_import_section(&mut imports, reader)
                    .map_err(|e| format!("reencode imports: {}", e))?;
                imports.import(
                    "arukellt_debug",
                    "breakpoint",
                    EntityType::Function(hook_type_index),
                );
                module.section(&imports);
            }
            Payload::FunctionSection(reader) => {
                if !saw_import_section {
                    // No import section in the input: synthesize one for the hook.
                    let mut imports = ImportSection::new();
                    imports.import(
                        "arukellt_debug",
                        "breakpoint",
                        EntityType::Function(hook_type_index),
                    );
                    module.section(&imports);
                    saw_import_section = true;
                }
                let mut funcs = wasm_encoder::FunctionSection::new();
                shift
                    .parse_function_section(&mut funcs, reader)
                    .map_err(|e| format!("reencode functions: {}", e))?;
                module.section(&funcs);
            }
            Payload::TableSection(reader) => {
                let mut tables = wasm_encoder::TableSection::new();
                shift
                    .parse_table_section(&mut tables, reader)
                    .map_err(|e| format!("reencode tables: {}", e))?;
                module.section(&tables);
            }
            Payload::MemorySection(reader) => {
                let mut memories = wasm_encoder::MemorySection::new();
                shift
                    .parse_memory_section(&mut memories, reader)
                    .map_err(|e| format!("reencode memories: {}", e))?;
                module.section(&memories);
            }
            Payload::TagSection(reader) => {
                let mut tags = wasm_encoder::TagSection::new();
                shift
                    .parse_tag_section(&mut tags, reader)
                    .map_err(|e| format!("reencode tags: {}", e))?;
                module.section(&tags);
            }
            Payload::GlobalSection(reader) => {
                let mut globals = wasm_encoder::GlobalSection::new();
                shift
                    .parse_global_section(&mut globals, reader)
                    .map_err(|e| format!("reencode globals: {}", e))?;
                module.section(&globals);
            }
            Payload::ExportSection(reader) => {
                let mut exports = ExportSection::new();
                shift
                    .parse_export_section(&mut exports, reader)
                    .map_err(|e| format!("reencode exports: {}", e))?;
                module.section(&exports);
            }
            Payload::StartSection { func, .. } => {
                let mapped = shift.function_index(func).map_err(|e| e.to_string())?;
                module.section(&wasm_encoder::StartSection {
                    function_index: mapped,
                });
            }
            Payload::ElementSection(reader) => {
                let mut elements = wasm_encoder::ElementSection::new();
                shift
                    .parse_element_section(&mut elements, reader)
                    .map_err(|e| format!("reencode elements: {}", e))?;
                module.section(&elements);
            }
            Payload::DataCountSection { count, .. } => {
                module.section(&wasm_encoder::DataCountSection { count });
            }
            Payload::DataSection(reader) => {
                let mut data = wasm_encoder::DataSection::new();
                shift
                    .parse_data_section(&mut data, reader)
                    .map_err(|e| format!("reencode data: {}", e))?;
                module.section(&data);
            }
            Payload::CodeSectionStart { count, .. } => {
                pending_code = Some(CodeSection::new());
                code_entry_count = count;
            }
            Payload::CodeSectionEntry(body) => {
                let code = pending_code
                    .as_mut()
                    .ok_or_else(|| "code entry outside code section".to_string())?;
                let this_index = defined_func_bodies_seen;
                defined_func_bodies_seen += 1;
                if this_index == start_defined_index {
                    let func = rewrite_start_body(
                        &mut shift,
                        &body,
                        breakpoint_line,
                        local_x,
                        hook_func_index,
                    )?;
                    code.function(&func);
                } else {
                    shift
                        .parse_function_body(code, body)
                        .map_err(|e| format!("reencode code: {}", e))?;
                }
                if defined_func_bodies_seen == code_entry_count {
                    // Flush in place to preserve section order relative to data.
                    if let Some(code) = pending_code.take() {
                        module.section(&code);
                    }
                }
            }
            Payload::CustomSection(section) => {
                module.section(&wasm_encoder::CustomSection {
                    name: section.name().into(),
                    data: section.data().into(),
                });
            }
            other => {
                return Err(format!("unsupported wasm payload: {:?}", other));
            }
        }
    }
    if let Some(code) = pending_code {
        module.section(&code);
    }
    Ok(module.finish())
}

/// Reencode `_start`, inserting `i32.const line; local.get x; call hook`
/// before its last `call` instruction (or at the end when it has no calls).
fn rewrite_start_body(
    shift: &mut ShiftFuncs,
    body: &wasmparser::FunctionBody,
    breakpoint_line: u32,
    local_x: u32,
    hook_func_index: u32,
) -> Result<Function, String> {
    let mut locals = Vec::new();
    for local in body
        .get_locals_reader()
        .map_err(|e| format!("wasm parse: {}", e))?
    {
        let (count, ty) = local.map_err(|e| format!("wasm parse: {}", e))?;
        let ty = shift.val_type(ty).map_err(|e| e.to_string())?;
        locals.push((count, ty));
    }
    let mut func = Function::new(locals);

    // Collect operators to locate the last top-level call.
    let mut ops = Vec::new();
    let mut reader = body
        .get_operators_reader()
        .map_err(|e| format!("wasm parse: {}", e))?;
    while !reader.eof() {
        ops.push(reader.read().map_err(|e| format!("wasm parse: {}", e))?);
    }
    let mut insert_at = ops.len();
    for (idx, op) in ops.iter().enumerate().rev() {
        if matches!(op, Operator::Call { .. }) {
            insert_at = idx;
            break;
        }
    }
    if insert_at >= ops.len() {
        // No call in `_start`: run the hook before the first return so it is
        // not emitted as dead code, otherwise before the trailing `end`.
        insert_at = ops
            .iter()
            .position(|op| matches!(op, Operator::Return))
            .unwrap_or_else(|| ops.len().saturating_sub(1));
    }

    for (idx, op) in ops.iter().enumerate() {
        if idx == insert_at {
            func.instruction(&Instruction::I32Const(breakpoint_line as i32));
            func.instruction(&Instruction::LocalGet(local_x));
            func.instruction(&Instruction::Call(hook_func_index));
        }
        let mapped = shift
            .instruction(op.clone())
            .map_err(|e| format!("reencode instruction: {}", e))?;
        func.instruction(&mapped);
    }
    Ok(func)
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
    use std::path::PathBuf;

    #[test]
    fn round_trip_smoke_fixture() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let wasm_path = repo.join("tests/fixtures/selfhost/debug_smoke.wasm");
        let ark_path = repo.join("tests/fixtures/selfhost/debug_smoke.ark");
        let raw = std::fs::read(&wasm_path).expect("read wasm");
        let source = std::fs::read_to_string(&ark_path).expect("read ark");
        wasmparser::validate(&raw).expect("validate input");
        let patched = prepare_debug_wasm(&raw, &source, 3).expect("patch");
        assert!(patched.len() > raw.len());
        assert!(patched.windows(25).any(|w| w == b"metadata.debug.source_map"));
        wasmparser::validate(&patched).expect("validate patched");
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
