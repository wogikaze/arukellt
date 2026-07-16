//! Wasm32 → memory64 converter.
//!
//! Converts a wasm32 module to memory64 by blanket-converting all i32
//! operations to i64.  This allows the linear memory to grow beyond
//! the 4 GB wasm32 limit.
//!
//! **Strategy**: ALL local function types are converted from i32 to i64.
//! WASI import types stay i32.  For each WASI import, a **wrapper function**
//! is appended that accepts i64 params, wraps them to i32, calls the import,
//! and extends i32 results to i64.  Calls to WASI imports are redirected to
//! the wrapper functions.

use std::collections::{HashMap, HashSet};
use wasm_encoder::reencode::RoundtripReencoder;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, Encode, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, Instruction, MemArg, MemorySection,
    MemoryType, Module, RawSection, StartSection, TypeSection, ValType,
};
use wasmparser::{
    BlockType as WBlockType, ExternalKind, GlobalType as WGlobalType, MemArg as WMemArg, Operator,
    Parser, Payload, TypeRef, ValType as WValType,
};

// ── Helper conversion functions ───────────────────────────────────

/// Convert a wasmparser ValType to a wasm-encoder ValType, changing
/// i32 to i64.
fn cv(vt: WValType) -> ValType {
    match vt {
        WValType::I32 => ValType::I64,
        WValType::I64 => ValType::I64,
        WValType::F32 => ValType::F32,
        WValType::F64 => ValType::F64,
        WValType::V128 => ValType::V128,
        WValType::Ref(rt) => {
            let mut re = RoundtripReencoder;
            ValType::Ref(
                wasm_encoder::reencode::Reencode::ref_type(&mut re, rt)
                    .unwrap_or(wasm_encoder::RefType::FUNCREF),
            )
        }
    }
}

/// Convert a wasmparser ValType to a wasm-encoder ValType WITHOUT
/// changing i32 to i64 (for WASI types).
fn cv_keep(vt: WValType) -> ValType {
    match vt {
        WValType::I32 => ValType::I32,
        WValType::I64 => ValType::I64,
        WValType::F32 => ValType::F32,
        WValType::F64 => ValType::F64,
        WValType::V128 => ValType::V128,
        WValType::Ref(rt) => {
            let mut re = RoundtripReencoder;
            ValType::Ref(
                wasm_encoder::reencode::Reencode::ref_type(&mut re, rt)
                    .unwrap_or(wasm_encoder::RefType::FUNCREF),
            )
        }
    }
}

/// Convert a wasmparser BlockType, changing i32 results to i64.
fn cv_bt(bt: WBlockType) -> BlockType {
    match bt {
        WBlockType::Empty => BlockType::Empty,
        WBlockType::Type(vt) => BlockType::Result(cv(vt)),
        WBlockType::FuncType(idx) => BlockType::FunctionType(idx),
    }
}

/// Convert a wasmparser MemArg to a wasm-encoder MemArg.
fn cv_ma(ma: WMemArg) -> MemArg {
    MemArg {
        offset: ma.offset,
        align: ma.align as u32,
        memory_index: ma.memory,
    }
}

/// Convert a const expression.  i32.const is changed to i64.const.
fn cv_ce(expr: &wasmparser::ConstExpr) -> ConstExpr {
    let mut reader = expr.get_operators_reader();
    let mut bytes = Vec::new();
    while let Ok(op) = reader.read() {
        match op {
            Operator::I32Const { value } => {
                Instruction::I64Const(value as i64).encode(&mut bytes);
            }
            Operator::I64Const { value } => {
                Instruction::I64Const(value).encode(&mut bytes);
            }
            Operator::GlobalGet { global_index } => {
                Instruction::GlobalGet(global_index).encode(&mut bytes);
            }
            Operator::RefNull { hty } => {
                let mut re = RoundtripReencoder;
                let ht = wasm_encoder::reencode::Reencode::heap_type(&mut re, hty)
                    .unwrap_or(wasm_encoder::HeapType::FUNC);
                Instruction::RefNull(ht).encode(&mut bytes);
            }
            Operator::RefFunc { function_index } => {
                Instruction::RefFunc(function_index).encode(&mut bytes);
            }
            Operator::End => break,
            _ => {
                let mut br = expr.get_binary_reader();
                let remaining = br.bytes_remaining();
                let raw = br.read_bytes(remaining).unwrap_or(&[]);
                let stripped = if raw.last() == Some(&0x0b) {
                    &raw[..raw.len() - 1]
                } else {
                    raw
                };
                return ConstExpr::raw(stripped.iter().copied());
            }
        }
    }
    ConstExpr::raw(bytes)
}

fn is_wasi_import(module: &str) -> bool {
    module.starts_with("wasi")
}

// ── SubType conversion ────────────────────────────────────────────

fn cv_subtype(st: &wasmparser::SubType, re: &mut RoundtripReencoder) -> wasm_encoder::SubType {
    let inner = match &st.composite_type.inner {
        wasmparser::CompositeInnerType::Func(ft) => {
            let p: Vec<ValType> = ft.params().iter().map(|&v| cv(v)).collect();
            let r: Vec<ValType> = ft.results().iter().map(|&v| cv(v)).collect();
            wasm_encoder::CompositeInnerType::Func(wasm_encoder::FuncType::new(p, r))
        }
        wasmparser::CompositeInnerType::Array(at) => {
            let f = wasm_encoder::reencode::Reencode::field_type(re, at.0.clone()).unwrap_or(
                wasm_encoder::FieldType {
                    element_type: wasm_encoder::StorageType::Val(ValType::I32),
                    mutable: false,
                },
            );
            wasm_encoder::CompositeInnerType::Array(wasm_encoder::ArrayType(f))
        }
        wasmparser::CompositeInnerType::Struct(sd) => {
            let fields: Vec<wasm_encoder::FieldType> = sd
                .fields
                .iter()
                .map(|f| {
                    wasm_encoder::reencode::Reencode::field_type(re, f.clone()).unwrap_or(
                        wasm_encoder::FieldType {
                            element_type: wasm_encoder::StorageType::Val(ValType::I32),
                            mutable: false,
                        },
                    )
                })
                .collect();
            wasm_encoder::CompositeInnerType::Struct(wasm_encoder::StructType {
                fields: fields.into(),
            })
        }
    };
    wasm_encoder::SubType {
        is_final: st.is_final,
        supertype_idx: st.supertype_idx.and_then(|i| i.as_module_index()),
        composite_type: wasm_encoder::CompositeType {
            inner,
            shared: st.composite_type.shared,
        },
    }
}

// ── Main entry point ──────────────────────────────────────────────

/// Convert a wasm32 module to memory64.
pub fn convert_to_memory64(data: &[u8]) -> Result<Vec<u8>, String> {
    // Phase 1: parse all payloads.
    let payloads: Vec<Payload> = Parser::new(0)
        .parse_all(data)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("wasm parse error: {e}"))?;

    // Phase 2: pre-process — collect type, import, and function info.
    // all_func_types[i] = (params, results) for type index i.
    let mut all_func_types: Vec<(Vec<WValType>, Vec<WValType>)> = Vec::new();
    // import_funcs: (module, name, type_idx, is_wasi) for each imported function.
    let mut import_funcs: Vec<(String, String, u32, bool)> = Vec::new();
    let mut import_global_types: Vec<WGlobalType> = Vec::new();
    let mut local_func_types: Vec<u32> = Vec::new();

    for p in &payloads {
        match p {
            Payload::TypeSection(r) => {
                for rg in r.clone() {
                    let rg = rg.map_err(|e| format!("type: {e}"))?;
                    for st in rg.types() {
                        if let wasmparser::CompositeInnerType::Func(ft) =
                            &st.composite_type.inner
                        {
                            all_func_types.push((ft.params().to_vec(), ft.results().to_vec()));
                        } else {
                            all_func_types.push((vec![], vec![]));
                        }
                    }
                }
            }
            Payload::ImportSection(r) => {
                for imp in r.clone() {
                    let imp = imp.map_err(|e| format!("import: {e}"))?;
                    match imp.ty {
                        TypeRef::Func(ti) => {
                            import_funcs.push((
                                imp.module.to_string(),
                                imp.name.to_string(),
                                ti,
                                is_wasi_import(imp.module),
                            ));
                        }
                        TypeRef::Global(gt) => import_global_types.push(gt),
                        _ => {}
                    }
                }
            }
            Payload::FunctionSection(r) => {
                for ti in r.clone() {
                    let ti = ti.map_err(|e| format!("function: {e}"))?;
                    local_func_types.push(ti);
                }
            }
            _ => {}
        }
    }

    let num_import_funcs = import_funcs.len() as u32;
    let num_local_funcs = local_func_types.len() as u32;
    let num_import_globals = import_global_types.len() as u32;
    let num_types = all_func_types.len() as u32;

    // Phase 3: build type mapping.
    //
    // New type section layout:
    //   [0, num_types) = converted types (i32→i64)
    //   [num_types, num_types + num_wasi_types) = original WASI types (i32)
    //
    // Type index mapping:
    //   - For non-WASI functions: old_idx → old_idx (identity, converted)
    //   - For WASI functions: old_idx → num_types + wasi_offset (original i32)

    // Collect unique WASI type indices.
    let mut wasi_type_indices: Vec<u32> = Vec::new();
    let mut wasi_type_set: HashSet<u32> = HashSet::new();
    for (_, _, ti, is_wasi) in &import_funcs {
        if *is_wasi && !wasi_type_set.contains(ti) {
            wasi_type_set.insert(*ti);
            wasi_type_indices.push(*ti);
        }
    }
    wasi_type_indices.sort();

    // Map: old WASI type index → new type index (in the WASI section).
    let mut wasi_type_map: HashMap<u32, u32> = HashMap::new();
    for (i, &ti) in wasi_type_indices.iter().enumerate() {
        wasi_type_map.insert(ti, num_types + i as u32);
    }

    // For non-WASI types, the mapping is identity.
    // (We don't need a separate map — just use the original index.)

    // Phase 4: build WASI wrapper function info.
    //
    // For each WASI import function at index I, we create a wrapper function
    // at index (num_import_funcs + num_local_funcs + wrapper_offset).
    // The wrapper has the CONVERTED type (i64 params/results) and internally
    // calls the original WASI import (with i32 params/results).
    //
    // We also need to create the converted type for each WASI import.

    // Map: WASI import function index → wrapper function index.
    let mut wasi_wrapper_map: HashMap<u32, u32> = HashMap::new();
    // Map: WASI type index → converted type index (for wrapper function types).
    let mut wasi_converted_type_map: HashMap<u32, u32> = HashMap::new();

    // We need to add converted versions of WASI types to the type section.
    // These will be placed after the WASI original types.
    let mut wasi_converted_types: Vec<(u32, Vec<ValType>, Vec<ValType>)> = Vec::new();
    let mut next_converted_type = num_types + wasi_type_indices.len() as u32;

    for (i, (_, _, ti, is_wasi)) in import_funcs.iter().enumerate() {
        if *is_wasi {
            let func_idx = i as u32;
            let wrapper_idx = num_import_funcs + num_local_funcs + wasi_wrapper_map.len() as u32;
            wasi_wrapper_map.insert(func_idx, wrapper_idx);

            // Ensure we have a converted type for this WASI type.
            if !wasi_converted_type_map.contains_key(ti) {
                let (p, r) = &all_func_types[*ti as usize];
                let cp: Vec<ValType> = p.iter().map(|&v| cv(v)).collect();
                let cr: Vec<ValType> = r.iter().map(|&v| cv(v)).collect();
                wasi_converted_types.push((*ti, cp, cr));
                wasi_converted_type_map.insert(*ti, next_converted_type);
                next_converted_type += 1;
            }
        }
    }

    // Phase 5: collect code section entries.
    let mut code_entries: Vec<wasmparser::FunctionBody> = Vec::new();
    for p in &payloads {
        if let Payload::CodeSectionEntry(body) = p {
            code_entries.push(body.clone());
        }
    }

    // Phase 6: build the code section (local functions + WASI wrappers).
    let mut code_section = CodeSection::new();

    // Convert local functions.
    for body in &code_entries {
        let func = convert_function(
            body,
            &wasi_wrapper_map,
            num_import_globals,
            &import_global_types,
        )?;
        code_section.function(&func);
    }

    // Build WASI wrapper functions.
    for (i, (_, _, ti, is_wasi)) in import_funcs.iter().enumerate() {
        if *is_wasi {
            let func_idx = i as u32;
            let (p, r) = &all_func_types[*ti as usize];
            let wrapper = build_wasi_wrapper(func_idx, p, r);
            code_section.function(&wrapper);
        }
    }

    let has_code = !code_entries.is_empty() || !wasi_wrapper_map.is_empty();

    // Phase 7: assemble the final module in section order.
    let mut result = Module::new();
    let mut inserted_code = false;

    for p in &payloads {
        match p {
            Payload::Version { .. } => {}

            Payload::TypeSection(r) => {
                let mut types = TypeSection::new();
                let mut re = RoundtripReencoder;

                // 1. Emit all converted types (i32→i64).
                for rg in r.clone() {
                    let rg = rg.map_err(|e| format!("type: {e}"))?;
                    let explicit = rg.is_explicit_rec_group();
                    let subs: Vec<wasm_encoder::SubType> =
                        rg.types().map(|st| cv_subtype(st, &mut re)).collect();
                    if explicit {
                        types.rec(subs);
                    } else {
                        for s in subs {
                            types.subtype(&s);
                        }
                    }
                }

                // 2. Emit original WASI types (i32, unconverted).
                for &ti in &wasi_type_indices {
                    let (p, r) = &all_func_types[ti as usize];
                    types.function(
                        p.iter().map(|&v| cv_keep(v)),
                        r.iter().map(|&v| cv_keep(v)),
                    );
                }

                // 3. Emit converted WASI types (i64, for wrapper functions).
                for (_, cp, cr) in &wasi_converted_types {
                    types.function(cp.iter().copied(), cr.iter().copied());
                }

                result.section(&types);
            }

            Payload::ImportSection(r) => {
                let mut imports = ImportSection::new();
                let mut re = RoundtripReencoder;
                for imp in r.clone() {
                    let imp = imp.map_err(|e| format!("import: {e}"))?;
                    match imp.ty {
                        TypeRef::Func(ti) => {
                            let is_wasi = is_wasi_import(imp.module);
                            // WASI imports keep their original i32 type index.
                            // Non-WASI imports use the converted (identity) type index.
                            let new_ti = if is_wasi {
                                *wasi_type_map.get(&ti).unwrap_or(&ti)
                            } else {
                                ti // identity mapping
                            };
                            imports.import(imp.module, imp.name, EntityType::Function(new_ti));
                        }
                        TypeRef::Table(tt) => {
                            let et = wasm_encoder::reencode::Reencode::table_type(&mut re, tt)
                                .map_err(|e| format!("table type: {e:?}"))?;
                            imports.import(imp.module, imp.name, et);
                        }
                        TypeRef::Memory(mt) => {
                            imports.import(
                                imp.module,
                                imp.name,
                                EntityType::Memory(MemoryType {
                                    minimum: mt.initial,
                                    maximum: mt.maximum,
                                    memory64: true,
                                    shared: mt.shared,
                                    page_size_log2: mt.page_size_log2,
                                }),
                            );
                        }
                        TypeRef::Global(gt) => {
                            imports.import(
                                imp.module,
                                imp.name,
                                GlobalType {
                                    val_type: cv_keep(gt.content_type),
                                    mutable: gt.mutable,
                                    shared: gt.shared,
                                },
                            );
                        }
                        TypeRef::Tag(_) => { /* skip */ }
                    }
                }
                result.section(&imports);
            }

            Payload::FunctionSection(r) => {
                let mut fs = FunctionSection::new();
                // Original local functions — type indices are identity-mapped.
                for ti in r.clone() {
                    let ti = ti.map_err(|e| format!("function: {e}"))?;
                    fs.function(ti);
                }
                // WASI wrapper functions — use converted WASI types.
                for (_, _, ti, is_wasi) in &import_funcs {
                    if *is_wasi {
                        let converted_ti = wasi_converted_type_map.get(ti).copied().unwrap_or(*ti);
                        fs.function(converted_ti);
                    }
                }
                result.section(&fs);
            }

            Payload::TableSection(r) => {
                let mut tables = wasm_encoder::TableSection::new();
                let mut re = RoundtripReencoder;
                for t in r.clone() {
                    let t = t.map_err(|e| format!("table: {e}"))?;
                    wasm_encoder::reencode::Reencode::parse_table(&mut re, &mut tables, t)
                        .map_err(|e| format!("table reencode: {e:?}"))?;
                }
                result.section(&tables);
            }

            Payload::MemorySection(r) => {
                let mut mems = MemorySection::new();
                for m in r.clone() {
                    let m = m.map_err(|e| format!("memory: {e}"))?;
                    // Drop the wasm32 4GiB page cap so memory.grow can exceed
                    // 65536 pages under memory64.
                    mems.memory(MemoryType {
                        minimum: m.initial.max(256),
                        maximum: None,
                        memory64: true,
                        shared: m.shared,
                        page_size_log2: m.page_size_log2,
                    });
                }
                result.section(&mems);
            }

            Payload::GlobalSection(r) => {
                let mut gs = GlobalSection::new();
                for g in r.clone() {
                    let g = g.map_err(|e| format!("global: {e}"))?;
                    gs.global(
                        GlobalType {
                            val_type: cv(g.ty.content_type),
                            mutable: g.ty.mutable,
                            shared: g.ty.shared,
                        },
                        &cv_ce(&g.init_expr),
                    );
                }
                result.section(&gs);
            }

            Payload::ExportSection(r) => {
                let mut es = ExportSection::new();
                for e in r.clone() {
                    let e = e.map_err(|e| format!("export: {e}"))?;
                    let k = match e.kind {
                        ExternalKind::Func => ExportKind::Func,
                        ExternalKind::Table => ExportKind::Table,
                        ExternalKind::Memory => ExportKind::Memory,
                        ExternalKind::Global => ExportKind::Global,
                        ExternalKind::Tag => ExportKind::Tag,
                    };
                    es.export(e.name, k, e.index);
                }
                result.section(&es);
            }

            Payload::StartSection { func, .. } => {
                result.section(&StartSection { function_index: *func });
            }

            Payload::ElementSection(r) => {
                let rng = r.range();
                result.section(&RawSection {
                    id: 9,
                    data: &data[rng.start..rng.end],
                });
            }

            Payload::DataCountSection { count, .. } => {
                result.section(&wasm_encoder::DataCountSection { count: *count });
            }

            Payload::CodeSectionStart { .. } => {
                if has_code {
                    result.section(&code_section);
                    inserted_code = true;
                }
            }

            Payload::CodeSectionEntry(_) => {}

            Payload::DataSection(r) => {
                if has_code && !inserted_code {
                    result.section(&code_section);
                    inserted_code = true;
                }
                let mut ds = wasm_encoder::DataSection::new();
                for seg in r.clone() {
                    let seg = seg.map_err(|e| format!("data: {e}"))?;
                    match seg.kind {
                        wasmparser::DataKind::Passive => {
                            ds.passive(seg.data.iter().copied());
                        }
                        wasmparser::DataKind::Active {
                            memory_index,
                            offset_expr,
                        } => {
                            ds.active(memory_index, &cv_ce(&offset_expr), seg.data.iter().copied());
                        }
                    }
                }
                result.section(&ds);
            }

            Payload::CustomSection(cs) => {
                result.section(&wasm_encoder::CustomSection {
                    name: std::borrow::Cow::Borrowed(cs.name()),
                    data: std::borrow::Cow::Borrowed(cs.data()),
                });
            }

            Payload::UnknownSection { id, contents, .. } => {
                result.section(&RawSection { id: *id, data: contents });
            }

            Payload::End(_) => {}

            _ => {}
        }
    }

    Ok(result.finish())
}

// ── WASI wrapper function ─────────────────────────────────────────

/// Build a wrapper function for a WASI import.
///
/// The wrapper has i64 params/results (converted type).  It wraps each
/// i32 param to i32, calls the original WASI import, and extends each
/// i32 result to i64.
fn build_wasi_wrapper(
    wasi_func_idx: u32,
    params: &[WValType],
    results: &[WValType],
) -> Function {
    let mut f = Function::new(Vec::<(u32, ValType)>::new());

    // Push each param, wrapping i64→i32 where the original was i32.
    for (i, pt) in params.iter().enumerate() {
        f.instruction(&Instruction::LocalGet(i as u32));
        if *pt == WValType::I32 {
            f.instruction(&Instruction::I32WrapI64);
        }
    }
    // Call the original WASI import.
    f.instruction(&Instruction::Call(wasi_func_idx));
    // Extend i32 results to i64.
    for rt in results {
        if *rt == WValType::I32 {
            f.instruction(&Instruction::I64ExtendI32U);
        }
    }
    f.instruction(&Instruction::End);
    f
}

// ── Function body conversion ──────────────────────────────────────

fn convert_function(
    body: &wasmparser::FunctionBody,
    wasi_wrapper_map: &HashMap<u32, u32>,
    nig: u32,
    igt: &[WGlobalType],
) -> Result<Function, String> {
    // Locals: i32 → i64.
    let lr = body.get_locals_reader().map_err(|e| format!("locals: {e}"))?;
    let mut new_locals: Vec<(u32, ValType)> = Vec::new();
    for l in lr {
        let (c, vt) = l.map_err(|e| format!("local: {e}"))?;
        new_locals.push((c, cv(vt)));
    }
    let mut func = Function::new(new_locals);

    // Instructions.
    let mut ops = body.get_operators_reader().map_err(|e| format!("ops: {e}"))?;
    while let Ok(op) = ops.read() {
        for insn in convert_operator(&op, wasi_wrapper_map, nig, igt) {
            func.instruction(&insn);
        }
    }
    Ok(func)
}

// ── Instruction conversion ────────────────────────────────────────

/// Convert a single wasmparser Operator to a list of wasm-encoder Instructions.
fn convert_operator(
    op: &Operator,
    wwm: &HashMap<u32, u32>,
    nig: u32,
    igt: &[WGlobalType],
) -> Vec<Instruction<'static>> {
    let mut r = Vec::new();

    macro_rules! i {
        ($x:expr) => { r.push($x) };
    }

    match op {
        // ── Control ────────────────────────────────────────────────
        Operator::Unreachable => i!(Instruction::Unreachable),
        Operator::Nop => i!(Instruction::Nop),
        Operator::Block { blockty } => i!(Instruction::Block(cv_bt(*blockty))),
        Operator::Loop { blockty } => i!(Instruction::Loop(cv_bt(*blockty))),
        Operator::If { blockty } => {
            i!(Instruction::I32WrapI64);
            i!(Instruction::If(cv_bt(*blockty)));
        }
        Operator::Else => i!(Instruction::Else),
        Operator::End => i!(Instruction::End),
        Operator::Br { relative_depth } => i!(Instruction::Br(*relative_depth)),
        Operator::BrIf { relative_depth } => {
            i!(Instruction::I32WrapI64);
            i!(Instruction::BrIf(*relative_depth));
        }
        Operator::BrTable { targets } => {
            i!(Instruction::I32WrapI64);
            let def = targets.default();
            let tv: Vec<u32> = targets.targets().map(|t| t.unwrap_or(def)).collect();
            i!(Instruction::BrTable(std::borrow::Cow::Owned(tv), def));
        }
        Operator::Return => i!(Instruction::Return),
        Operator::Call { function_index } => {
            // Redirect WASI import calls to wrapper functions.
            let idx = wwm.get(function_index).copied().unwrap_or(*function_index);
            i!(Instruction::Call(idx));
        }
        Operator::CallIndirect { type_index, table_index } => {
            i!(Instruction::CallIndirect { type_index: *type_index, table_index: *table_index });
        }
        Operator::ReturnCall { function_index } => {
            let idx = wwm.get(function_index).copied().unwrap_or(*function_index);
            i!(Instruction::ReturnCall(idx));
        }
        Operator::ReturnCallIndirect { type_index, table_index } => {
            i!(Instruction::ReturnCallIndirect { type_index: *type_index, table_index: *table_index });
        }
        Operator::Drop => i!(Instruction::Drop),
        Operator::Select => {
            i!(Instruction::I32WrapI64);
            i!(Instruction::Select);
        }
        Operator::TypedSelect { ty } => {
            i!(Instruction::TypedSelect(cv(*ty)));
        }

        // ── Variables ──────────────────────────────────────────────
        Operator::LocalGet { local_index } => i!(Instruction::LocalGet(*local_index)),
        Operator::LocalSet { local_index } => i!(Instruction::LocalSet(*local_index)),
        Operator::LocalTee { local_index } => i!(Instruction::LocalTee(*local_index)),
        Operator::GlobalGet { global_index } => {
            // Imported i32 globals: extend after get.
            if *global_index < nig {
                let gt = &igt[*global_index as usize];
                if gt.content_type == WValType::I32 {
                    i!(Instruction::GlobalGet(*global_index));
                    i!(Instruction::I64ExtendI32U);
                } else {
                    i!(Instruction::GlobalGet(*global_index));
                }
            } else {
                i!(Instruction::GlobalGet(*global_index));
            }
        }
        Operator::GlobalSet { global_index } => {
            if *global_index < nig {
                let gt = &igt[*global_index as usize];
                if gt.content_type == WValType::I32 {
                    i!(Instruction::I32WrapI64);
                    i!(Instruction::GlobalSet(*global_index));
                } else {
                    i!(Instruction::GlobalSet(*global_index));
                }
            } else {
                i!(Instruction::GlobalSet(*global_index));
            }
        }

        // ── Memory loads (i32 → keep + extend) ─────────────────────
        // Sign-extend full i32 loads: wasm32 code stores signed sentinels such as
        // local-index "-1". Zero-extending turns those into 0xFFFF_FFFF and breaks
        // `idx >= 0` checks, which then emit invalid `local.get 4294967295`.
        Operator::I32Load { memarg } => { i!(Instruction::I32Load(cv_ma(*memarg))); i!(Instruction::I64ExtendI32S); }
        Operator::I32Load8S { memarg } => { i!(Instruction::I32Load8S(cv_ma(*memarg))); i!(Instruction::I64ExtendI32S); }
        Operator::I32Load8U { memarg } => { i!(Instruction::I32Load8U(cv_ma(*memarg))); i!(Instruction::I64ExtendI32U); }
        Operator::I32Load16S { memarg } => { i!(Instruction::I32Load16S(cv_ma(*memarg))); i!(Instruction::I64ExtendI32S); }
        Operator::I32Load16U { memarg } => { i!(Instruction::I32Load16U(cv_ma(*memarg))); i!(Instruction::I64ExtendI32U); }
        Operator::I64Load { memarg } => i!(Instruction::I64Load(cv_ma(*memarg))),
        Operator::F32Load { memarg } => i!(Instruction::F32Load(cv_ma(*memarg))),
        Operator::F64Load { memarg } => i!(Instruction::F64Load(cv_ma(*memarg))),
        Operator::I64Load8S { memarg } => i!(Instruction::I64Load8S(cv_ma(*memarg))),
        Operator::I64Load8U { memarg } => i!(Instruction::I64Load8U(cv_ma(*memarg))),
        Operator::I64Load16S { memarg } => i!(Instruction::I64Load16S(cv_ma(*memarg))),
        Operator::I64Load16U { memarg } => i!(Instruction::I64Load16U(cv_ma(*memarg))),
        Operator::I64Load32S { memarg } => i!(Instruction::I64Load32S(cv_ma(*memarg))),
        Operator::I64Load32U { memarg } => i!(Instruction::I64Load32U(cv_ma(*memarg))),

        // ── Memory stores (i32 → wrap before) ──────────────────────
        Operator::I32Store { memarg } => { i!(Instruction::I32WrapI64); i!(Instruction::I32Store(cv_ma(*memarg))); }
        Operator::I32Store8 { memarg } => { i!(Instruction::I32WrapI64); i!(Instruction::I32Store8(cv_ma(*memarg))); }
        Operator::I32Store16 { memarg } => { i!(Instruction::I32WrapI64); i!(Instruction::I32Store16(cv_ma(*memarg))); }
        Operator::I64Store { memarg } => i!(Instruction::I64Store(cv_ma(*memarg))),
        Operator::F32Store { memarg } => i!(Instruction::F32Store(cv_ma(*memarg))),
        Operator::F64Store { memarg } => i!(Instruction::F64Store(cv_ma(*memarg))),
        Operator::I64Store8 { memarg } => i!(Instruction::I64Store8(cv_ma(*memarg))),
        Operator::I64Store16 { memarg } => i!(Instruction::I64Store16(cv_ma(*memarg))),
        Operator::I64Store32 { memarg } => i!(Instruction::I64Store32(cv_ma(*memarg))),

        // ── Memory bulk ops ────────────────────────────────────────
        // In memory64 mode, memory.size returns i64 and memory.grow
        // takes i64 and returns i64 — no wrap/extend needed.
        Operator::MemorySize { mem } => i!(Instruction::MemorySize(*mem)),
        Operator::MemoryGrow { mem } => i!(Instruction::MemoryGrow(*mem)),
        Operator::MemoryCopy { dst_mem, src_mem } => i!(Instruction::MemoryCopy { src_mem: *src_mem, dst_mem: *dst_mem }),
        Operator::MemoryFill { mem } => i!(Instruction::MemoryFill(*mem)),
        Operator::MemoryInit { data_index, mem } => i!(Instruction::MemoryInit { mem: *mem, data_index: *data_index }),
        Operator::DataDrop { data_index } => i!(Instruction::DataDrop(*data_index)),

        // ── Constants ──────────────────────────────────────────────
        Operator::I32Const { value } => i!(Instruction::I64Const(*value as i64)),
        Operator::I64Const { value } => i!(Instruction::I64Const(*value)),
        Operator::F32Const { value } => i!(Instruction::F32Const(f32::from_bits(value.bits()))),
        Operator::F64Const { value } => i!(Instruction::F64Const(f64::from_bits(value.bits()))),

        // ── i32 comparisons → i64 + extend ─────────────────────────
        Operator::I32Eqz => { i!(Instruction::I64Eqz); i!(Instruction::I64ExtendI32U); }
        Operator::I32Eq => { i!(Instruction::I64Eq); i!(Instruction::I64ExtendI32U); }
        Operator::I32Ne => { i!(Instruction::I64Ne); i!(Instruction::I64ExtendI32U); }
        Operator::I32LtS => { i!(Instruction::I64LtS); i!(Instruction::I64ExtendI32U); }
        Operator::I32LtU => { i!(Instruction::I64LtU); i!(Instruction::I64ExtendI32U); }
        Operator::I32GtS => { i!(Instruction::I64GtS); i!(Instruction::I64ExtendI32U); }
        Operator::I32GtU => { i!(Instruction::I64GtU); i!(Instruction::I64ExtendI32U); }
        Operator::I32LeS => { i!(Instruction::I64LeS); i!(Instruction::I64ExtendI32U); }
        Operator::I32LeU => { i!(Instruction::I64LeU); i!(Instruction::I64ExtendI32U); }
        Operator::I32GeS => { i!(Instruction::I64GeS); i!(Instruction::I64ExtendI32U); }
        Operator::I32GeU => { i!(Instruction::I64GeU); i!(Instruction::I64ExtendI32U); }

        // ── i64 / float comparisons: extend result to i64 ───────────
        Operator::I64Eqz => { i!(Instruction::I64Eqz); i!(Instruction::I64ExtendI32U); }
        Operator::I64Eq => { i!(Instruction::I64Eq); i!(Instruction::I64ExtendI32U); }
        Operator::I64Ne => { i!(Instruction::I64Ne); i!(Instruction::I64ExtendI32U); }
        Operator::I64LtS => { i!(Instruction::I64LtS); i!(Instruction::I64ExtendI32U); }
        Operator::I64LtU => { i!(Instruction::I64LtU); i!(Instruction::I64ExtendI32U); }
        Operator::I64GtS => { i!(Instruction::I64GtS); i!(Instruction::I64ExtendI32U); }
        Operator::I64GtU => { i!(Instruction::I64GtU); i!(Instruction::I64ExtendI32U); }
        Operator::I64LeS => { i!(Instruction::I64LeS); i!(Instruction::I64ExtendI32U); }
        Operator::I64LeU => { i!(Instruction::I64LeU); i!(Instruction::I64ExtendI32U); }
        Operator::I64GeS => { i!(Instruction::I64GeS); i!(Instruction::I64ExtendI32U); }
        Operator::I64GeU => { i!(Instruction::I64GeU); i!(Instruction::I64ExtendI32U); }
        Operator::F32Eq => { i!(Instruction::F32Eq); i!(Instruction::I64ExtendI32U); }
        Operator::F32Ne => { i!(Instruction::F32Ne); i!(Instruction::I64ExtendI32U); }
        Operator::F32Lt => { i!(Instruction::F32Lt); i!(Instruction::I64ExtendI32U); }
        Operator::F32Gt => { i!(Instruction::F32Gt); i!(Instruction::I64ExtendI32U); }
        Operator::F32Le => { i!(Instruction::F32Le); i!(Instruction::I64ExtendI32U); }
        Operator::F32Ge => { i!(Instruction::F32Ge); i!(Instruction::I64ExtendI32U); }
        Operator::F64Eq => { i!(Instruction::F64Eq); i!(Instruction::I64ExtendI32U); }
        Operator::F64Ne => { i!(Instruction::F64Ne); i!(Instruction::I64ExtendI32U); }
        Operator::F64Lt => { i!(Instruction::F64Lt); i!(Instruction::I64ExtendI32U); }
        Operator::F64Gt => { i!(Instruction::F64Gt); i!(Instruction::I64ExtendI32U); }
        Operator::F64Le => { i!(Instruction::F64Le); i!(Instruction::I64ExtendI32U); }
        Operator::F64Ge => { i!(Instruction::F64Ge); i!(Instruction::I64ExtendI32U); }

        // ── i32 arithmetic/logic → i64 with 32-bit wrap ─────────────
        // Blanket i32→i64 without truncation diverges from wasm32 wraparound
        // (and breaks selfhost compilers built from current sources).  After
        // each former-i32 op, wrap to i32 then sign-extend so signed
        // sentinels and index math stay in the low 32 bits.
        Operator::I32Clz => {
            i!(Instruction::I64Clz);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32U);
        }
        Operator::I32Ctz => {
            i!(Instruction::I64Ctz);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32U);
        }
        Operator::I32Popcnt => {
            i!(Instruction::I64Popcnt);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32U);
        }
        Operator::I32Add => {
            i!(Instruction::I64Add);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32Sub => {
            i!(Instruction::I64Sub);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32Mul => {
            i!(Instruction::I64Mul);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32DivS => {
            i!(Instruction::I64DivS);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32DivU => {
            i!(Instruction::I64DivU);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32U);
        }
        Operator::I32RemS => {
            i!(Instruction::I64RemS);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32RemU => {
            i!(Instruction::I64RemU);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32U);
        }
        Operator::I32And => {
            i!(Instruction::I64And);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32Or => {
            i!(Instruction::I64Or);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32Xor => {
            i!(Instruction::I64Xor);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32Shl => {
            i!(Instruction::I64Shl);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32ShrS => {
            i!(Instruction::I64ShrS);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32ShrU => {
            i!(Instruction::I64ShrU);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32U);
        }
        Operator::I32Rotl => {
            i!(Instruction::I64Rotl);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }
        Operator::I32Rotr => {
            i!(Instruction::I64Rotr);
            i!(Instruction::I32WrapI64);
            i!(Instruction::I64ExtendI32S);
        }

        // ── i64 arithmetic/logic: no change ────────────────────────
        Operator::I64Clz => i!(Instruction::I64Clz),
        Operator::I64Ctz => i!(Instruction::I64Ctz),
        Operator::I64Popcnt => i!(Instruction::I64Popcnt),
        Operator::I64Add => i!(Instruction::I64Add),
        Operator::I64Sub => i!(Instruction::I64Sub),
        Operator::I64Mul => i!(Instruction::I64Mul),
        Operator::I64DivS => i!(Instruction::I64DivS),
        Operator::I64DivU => i!(Instruction::I64DivU),
        Operator::I64RemS => i!(Instruction::I64RemS),
        Operator::I64RemU => i!(Instruction::I64RemU),
        Operator::I64And => i!(Instruction::I64And),
        Operator::I64Or => i!(Instruction::I64Or),
        Operator::I64Xor => i!(Instruction::I64Xor),
        Operator::I64Shl => i!(Instruction::I64Shl),
        Operator::I64ShrS => i!(Instruction::I64ShrS),
        Operator::I64ShrU => i!(Instruction::I64ShrU),
        Operator::I64Rotl => i!(Instruction::I64Rotl),
        Operator::I64Rotr => i!(Instruction::I64Rotr),

        // ── Float arithmetic: no change ────────────────────────────
        Operator::F32Abs => i!(Instruction::F32Abs),
        Operator::F32Neg => i!(Instruction::F32Neg),
        Operator::F32Ceil => i!(Instruction::F32Ceil),
        Operator::F32Floor => i!(Instruction::F32Floor),
        Operator::F32Trunc => i!(Instruction::F32Trunc),
        Operator::F32Nearest => i!(Instruction::F32Nearest),
        Operator::F32Sqrt => i!(Instruction::F32Sqrt),
        Operator::F32Add => i!(Instruction::F32Add),
        Operator::F32Sub => i!(Instruction::F32Sub),
        Operator::F32Mul => i!(Instruction::F32Mul),
        Operator::F32Div => i!(Instruction::F32Div),
        Operator::F32Min => i!(Instruction::F32Min),
        Operator::F32Max => i!(Instruction::F32Max),
        Operator::F32Copysign => i!(Instruction::F32Copysign),
        Operator::F64Abs => i!(Instruction::F64Abs),
        Operator::F64Neg => i!(Instruction::F64Neg),
        Operator::F64Ceil => i!(Instruction::F64Ceil),
        Operator::F64Floor => i!(Instruction::F64Floor),
        Operator::F64Trunc => i!(Instruction::F64Trunc),
        Operator::F64Nearest => i!(Instruction::F64Nearest),
        Operator::F64Sqrt => i!(Instruction::F64Sqrt),
        Operator::F64Add => i!(Instruction::F64Add),
        Operator::F64Sub => i!(Instruction::F64Sub),
        Operator::F64Mul => i!(Instruction::F64Mul),
        Operator::F64Div => i!(Instruction::F64Div),
        Operator::F64Min => i!(Instruction::F64Min),
        Operator::F64Max => i!(Instruction::F64Max),
        Operator::F64Copysign => i!(Instruction::F64Copysign),

        // ── Type conversions ───────────────────────────────────────
        Operator::I32WrapI64 => { /* remove */ }
        Operator::I64ExtendI32S => { /* remove */ }
        Operator::I64ExtendI32U => { /* remove */ }
        Operator::I32TruncF32S => i!(Instruction::I64TruncF32S),
        Operator::I32TruncF32U => i!(Instruction::I64TruncF32U),
        Operator::I32TruncF64S => i!(Instruction::I64TruncF64S),
        Operator::I32TruncF64U => i!(Instruction::I64TruncF64U),
        Operator::I64TruncF32S => i!(Instruction::I64TruncF32S),
        Operator::I64TruncF32U => i!(Instruction::I64TruncF32U),
        Operator::I64TruncF64S => i!(Instruction::I64TruncF64S),
        Operator::I64TruncF64U => i!(Instruction::I64TruncF64U),
        Operator::F32ConvertI32S => i!(Instruction::F32ConvertI64S),
        Operator::F32ConvertI32U => i!(Instruction::F32ConvertI64U),
        Operator::F32ConvertI64S => i!(Instruction::F32ConvertI64S),
        Operator::F32ConvertI64U => i!(Instruction::F32ConvertI64U),
        Operator::F64ConvertI32S => i!(Instruction::F64ConvertI64S),
        Operator::F64ConvertI32U => i!(Instruction::F64ConvertI64U),
        Operator::F64ConvertI64S => i!(Instruction::F64ConvertI64S),
        Operator::F64ConvertI64U => i!(Instruction::F64ConvertI64U),
        Operator::F32DemoteF64 => i!(Instruction::F32DemoteF64),
        Operator::F64PromoteF32 => i!(Instruction::F64PromoteF32),
        Operator::I32ReinterpretF32 => { i!(Instruction::I32ReinterpretF32); i!(Instruction::I64ExtendI32U); }
        Operator::I64ReinterpretF64 => i!(Instruction::I64ReinterpretF64),
        Operator::F32ReinterpretI32 => { i!(Instruction::I32WrapI64); i!(Instruction::F32ReinterpretI32); }
        Operator::F64ReinterpretI64 => i!(Instruction::F64ReinterpretI64),
        Operator::I32Extend8S => i!(Instruction::I64Extend8S),
        Operator::I32Extend16S => i!(Instruction::I64Extend16S),
        Operator::I64Extend8S => i!(Instruction::I64Extend8S),
        Operator::I64Extend16S => i!(Instruction::I64Extend16S),
        Operator::I64Extend32S => i!(Instruction::I64Extend32S),
        Operator::I32TruncSatF32S => i!(Instruction::I64TruncSatF32S),
        Operator::I32TruncSatF32U => i!(Instruction::I64TruncSatF32U),
        Operator::I32TruncSatF64S => i!(Instruction::I64TruncSatF64S),
        Operator::I32TruncSatF64U => i!(Instruction::I64TruncSatF64U),
        Operator::I64TruncSatF32S => i!(Instruction::I64TruncSatF32S),
        Operator::I64TruncSatF32U => i!(Instruction::I64TruncSatF32U),
        Operator::I64TruncSatF64S => i!(Instruction::I64TruncSatF64S),
        Operator::I64TruncSatF64U => i!(Instruction::I64TruncSatF64U),

        // ── Reference instructions ─────────────────────────────────
        Operator::RefNull { hty } => {
            let mut re = RoundtripReencoder;
            let ht = wasm_encoder::reencode::Reencode::heap_type(&mut re, *hty)
                .unwrap_or(wasm_encoder::HeapType::FUNC);
            i!(Instruction::RefNull(ht));
        }
        Operator::RefIsNull => { i!(Instruction::RefIsNull); i!(Instruction::I64ExtendI32U); }
        Operator::RefFunc { function_index } => i!(Instruction::RefFunc(*function_index)),
        Operator::RefEq => { i!(Instruction::RefEq); i!(Instruction::I64ExtendI32U); }

        // ── Table ops ──────────────────────────────────────────────
        Operator::TableInit { elem_index, table } => i!(Instruction::TableInit { elem_index: *elem_index, table: *table }),
        Operator::ElemDrop { elem_index } => i!(Instruction::ElemDrop(*elem_index)),
        Operator::TableCopy { dst_table, src_table } => i!(Instruction::TableCopy { dst_table: *dst_table, src_table: *src_table }),
        Operator::TableSize { table } => { i!(Instruction::TableSize(*table)); i!(Instruction::I64ExtendI32U); }
        Operator::TableGrow { table } => { i!(Instruction::I32WrapI64); i!(Instruction::TableGrow(*table)); i!(Instruction::I64ExtendI32U); }
        Operator::TableGet { table } => i!(Instruction::TableGet(*table)),
        Operator::TableSet { table } => i!(Instruction::TableSet(*table)),
        Operator::TableFill { table } => { i!(Instruction::I32WrapI64); i!(Instruction::TableFill(*table)); }

        // ── SIMD: handle i32-related ops, copy rest ────────────────
        Operator::V128Const { value } => i!(Instruction::V128Const(value.i128())),
        Operator::I32x4ExtractLane { lane } => { i!(Instruction::I32x4ExtractLane(*lane)); i!(Instruction::I64ExtendI32U); }
        Operator::I32x4ReplaceLane { lane } => { i!(Instruction::I32WrapI64); i!(Instruction::I32x4ReplaceLane(*lane)); }
        Operator::I32x4Splat => { i!(Instruction::I32WrapI64); i!(Instruction::I32x4Splat); }
        _ => handle_remaining(op, &mut r),
    }

    r
}

/// Best-effort handling of remaining operators (SIMD, GC, threads, etc.).
fn handle_remaining(op: &Operator, r: &mut Vec<Instruction<'static>>) {
    macro_rules! mem1 {
        ($variant:ident, $memarg:expr) => {
            r.push(Instruction::$variant(cv_ma($memarg)))
        };
    }
    match op {
        Operator::V128Load { memarg } => mem1!(V128Load, *memarg),
        Operator::V128Store { memarg } => mem1!(V128Store, *memarg),
        Operator::V128Load8x8S { memarg } => mem1!(V128Load8x8S, *memarg),
        Operator::V128Load8x8U { memarg } => mem1!(V128Load8x8U, *memarg),
        Operator::V128Load16x4S { memarg } => mem1!(V128Load16x4S, *memarg),
        Operator::V128Load16x4U { memarg } => mem1!(V128Load16x4U, *memarg),
        Operator::V128Load32x2S { memarg } => mem1!(V128Load32x2S, *memarg),
        Operator::V128Load32x2U { memarg } => mem1!(V128Load32x2U, *memarg),
        Operator::V128Load8Splat { memarg } => mem1!(V128Load8Splat, *memarg),
        Operator::V128Load16Splat { memarg } => mem1!(V128Load16Splat, *memarg),
        Operator::V128Load32Splat { memarg } => mem1!(V128Load32Splat, *memarg),
        Operator::V128Load64Splat { memarg } => mem1!(V128Load64Splat, *memarg),
        Operator::V128Load32Zero { memarg } => mem1!(V128Load32Zero, *memarg),
        Operator::V128Load64Zero { memarg } => mem1!(V128Load64Zero, *memarg),

        Operator::I8x16ExtractLaneS { lane } => { r.push(Instruction::I8x16ExtractLaneS(*lane)); r.push(Instruction::I64ExtendI32S); }
        Operator::I8x16ExtractLaneU { lane } => { r.push(Instruction::I8x16ExtractLaneU(*lane)); r.push(Instruction::I64ExtendI32U); }
        Operator::I8x16ReplaceLane { lane } => { r.push(Instruction::I32WrapI64); r.push(Instruction::I8x16ReplaceLane(*lane)); }
        Operator::I16x8ExtractLaneS { lane } => { r.push(Instruction::I16x8ExtractLaneS(*lane)); r.push(Instruction::I64ExtendI32S); }
        Operator::I16x8ExtractLaneU { lane } => { r.push(Instruction::I16x8ExtractLaneU(*lane)); r.push(Instruction::I64ExtendI32U); }
        Operator::I16x8ReplaceLane { lane } => { r.push(Instruction::I32WrapI64); r.push(Instruction::I16x8ReplaceLane(*lane)); }
        Operator::I8x16Splat => { r.push(Instruction::I32WrapI64); r.push(Instruction::I8x16Splat); }
        Operator::I16x8Splat => { r.push(Instruction::I32WrapI64); r.push(Instruction::I16x8Splat); }

        Operator::StructNew { struct_type_index } => r.push(Instruction::StructNew(*struct_type_index)),
        Operator::StructNewDefault { struct_type_index } => r.push(Instruction::StructNewDefault(*struct_type_index)),
        Operator::ArrayNew { array_type_index } => r.push(Instruction::ArrayNew(*array_type_index)),
        Operator::ArrayNewDefault { array_type_index } => r.push(Instruction::ArrayNewDefault(*array_type_index)),
        Operator::ArrayLen => r.push(Instruction::ArrayLen),

        _ => { /* skip */ }
    }
}
