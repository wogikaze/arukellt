//! WIT / Component Model generation for T3 targets.
//!
//! Generates WIT (Wasm Interface Type) descriptions from the compiler's
//! public API surface. Component wrapping uses external `wasm-tools`.

pub mod canonical_abi;
pub mod handle_table;
mod wit;
pub mod wit_parse;
pub mod wrap;

pub use wit::{
    WitEnum, WitError, WitFunction, WitRecord, WitType, WitVariant, WitWorld, WitWorldSpec,
    generate_wit, parse_world_spec,
};
pub use wit_parse::{
    WitDocument, WitInterface, WitParseError, parse_wit, wit_interface_to_mir_imports,
};

use ark_mir::mir::MirModule;
use ark_typecheck::types::Type;

/// Convert a MIR module into a WIT world descriptor.
///
/// Extracts all non-internal, non-main functions as exports.
/// Converts struct_defs and enum_defs to WIT records/variants.
/// Returns (WitWorld, warnings) where warnings describe non-exportable functions.
pub fn mir_to_wit_world(mir: &MirModule, world_name: &str) -> Result<WitWorld, WitError> {
    mir_to_wit_world_with_warnings(mir, world_name, None).map(|(world, _)| world)
}

/// Like `mir_to_wit_world` but also returns diagnostic warnings for non-exportable functions.
/// Accepts an optional `world_spec` string (e.g., "wasi:cli/command") which adapts the
/// generated world to conform to a standard WASI world.
pub fn mir_to_wit_world_with_warnings(
    mir: &MirModule,
    world_name: &str,
    world_spec: Option<&str>,
) -> Result<(WitWorld, Vec<String>), WitError> {
    let mut warnings = Vec::new();
    let mut world = WitWorld {
        name: world_name.to_string(),
        functions: Vec::new(),
        imports: Vec::new(),
        records: Vec::new(),
        enums: Vec::new(),
        variants: Vec::new(),
        resources: Vec::new(),
        world_spec: None,
    };

    // Populate imports from MIR
    for imp in &mir.imports {
        let params: Vec<(String, WitType)> = imp
            .param_types
            .iter()
            .enumerate()
            .filter_map(|(i, ty)| wit_type_name_to_wit(ty).map(|wt| (format!("p{}", i), wt)))
            .collect();
        let result = imp
            .return_type
            .as_ref()
            .and_then(|ty| wit_type_name_to_wit(ty));
        world.imports.push(WitFunction {
            name: imp.name.clone(),
            params,
            result,
        });
    }

    // First pass: collect exported functions and the type names they reference
    let mut exported_fns = Vec::new();
    for func in &mir.functions {
        let name = &func.name;
        if name.starts_with("__") || name == "_start" || name == "main" {
            continue;
        }
        if !func.is_exported {
            continue;
        }

        let params: Vec<(String, WitType)> = func
            .params
            .iter()
            .filter_map(|p| {
                let pname = p.name.clone().unwrap_or_else(|| format!("p{}", p.id.0));
                type_to_wit(&p.ty).map(|wt| (pname, wt))
            })
            .collect();

        if params.len() != func.params.len() {
            let non_exportable: Vec<_> = func
                .params
                .iter()
                .filter(|p| type_to_wit(&p.ty).is_none())
                .map(|p| format!("{:?}", p.ty))
                .collect();
            warnings.push(format!(
                "function `{}` has non-exportable parameter type(s): {}",
                name,
                non_exportable.join(", ")
            ));
            continue;
        }

        let result = type_to_wit(&func.return_ty);
        let result = match &func.return_ty {
            Type::Unit => None,
            _ => {
                if result.is_none() && func.return_ty != Type::Unit {
                    warnings.push(format!(
                        "function `{}` has non-exportable return type: {:?}",
                        name, func.return_ty
                    ));
                    continue;
                }
                result
            }
        };

        exported_fns.push(WitFunction {
            name: name.clone(),
            params,
            result,
        });
    }

    // Collect type names referenced by exported functions (and imports)
    let mut referenced_types = std::collections::HashSet::new();
    for f in exported_fns.iter().chain(world.imports.iter()) {
        for (_, wt) in &f.params {
            collect_wit_type_refs(wt, &mut referenced_types);
        }
        if let Some(wt) = &f.result {
            collect_wit_type_refs(wt, &mut referenced_types);
        }
    }

    // Pre-compute special enum types (Option/Result patterns) for inline resolution
    let mut special_enums: std::collections::HashMap<String, WitType> =
        std::collections::HashMap::new();
    for (name, variants) in &mir.type_table.enum_defs {
        if let Some(inner_type_name) = is_option_enum(variants) {
            if let Some(inner_wt) = type_name_to_wit(inner_type_name) {
                special_enums.insert(name.clone(), WitType::Option(Box::new(inner_wt)));
            }
        } else if let Some((ok_name, err_name)) = is_result_enum(variants) {
            let ok_wt = type_name_to_wit(ok_name).map(Box::new);
            let err_wt = type_name_to_wit(err_name).map(Box::new);
            special_enums.insert(
                name.clone(),
                WitType::Result {
                    ok: ok_wt,
                    err: err_wt,
                },
            );
        }
    }

    // Pre-compute tuple struct types for inline resolution
    let mut tuple_structs: std::collections::HashMap<String, WitType> =
        std::collections::HashMap::new();
    for (name, fields) in &mir.type_table.struct_defs {
        if name.starts_with("__tuple") {
            let elem_types: Vec<WitType> = fields
                .iter()
                .filter_map(|(_, ftype)| type_name_to_wit(ftype))
                .collect();
            if elem_types.len() == fields.len() {
                tuple_structs.insert(name.clone(), WitType::Tuple(elem_types));
            }
        }
    }

    // Convert struct definitions to WIT records — only if referenced by exports
    for (name, fields) in &mir.type_table.struct_defs {
        if name.starts_with("__") || !referenced_types.contains(name.as_str()) {
            continue;
        }
        let wit_fields: Vec<(String, WitType)> = fields
            .iter()
            .filter_map(|(fname, ftype)| {
                type_name_to_wit_ctx(ftype, &special_enums, &tuple_structs)
                    .map(|wt| (fname.clone(), wt))
            })
            .collect();
        if wit_fields.len() == fields.len() {
            world.records.push(WitRecord {
                name: name.clone(),
                fields: wit_fields,
            });
        }
    }

    // Convert enum definitions to WIT variants — only if referenced by exports
    for (name, variants) in &mir.type_table.enum_defs {
        if name.starts_with("__") || !referenced_types.contains(name.as_str()) {
            continue;
        }
        // Skip Option/Result patterns — they are inlined as option<T>/result<T, E>
        if special_enums.contains_key(name) {
            continue;
        }
        let all_unit = variants.iter().all(|(_, payloads)| payloads.is_empty());
        if all_unit {
            world.enums.push(WitEnum {
                name: name.clone(),
                variants: variants.iter().map(|(vname, _)| vname.clone()).collect(),
            });
        } else {
            let cases: Vec<(String, Option<WitType>)> = variants
                .iter()
                .map(|(vname, payloads)| {
                    if payloads.is_empty() {
                        (vname.clone(), None)
                    } else if payloads.len() == 1 {
                        (
                            vname.clone(),
                            type_name_to_wit_ctx(
                                &payloads[0],
                                &special_enums,
                                &tuple_structs,
                            ),
                        )
                    } else {
                        let elems: Vec<WitType> = payloads
                            .iter()
                            .filter_map(|t| {
                                type_name_to_wit_ctx(t, &special_enums, &tuple_structs)
                            })
                            .collect();
                        if elems.len() == payloads.len() {
                            (vname.clone(), Some(WitType::Tuple(elems)))
                        } else {
                            (vname.clone(), None)
                        }
                    }
                })
                .collect();
            world.variants.push(WitVariant {
                name: name.clone(),
                cases,
            });
        }
    }

    world.functions = exported_fns;

    // Apply standard world spec if requested
    if let Some(spec_str) = world_spec {
        let spec = parse_world_spec(spec_str).ok_or_else(|| WitError::UnknownWorld {
            spec: spec_str.to_string(),
        })?;

        // Validate required exports
        for (required_path, required_fn) in &spec.required_exports {
            let has_export = world.functions.iter().any(|f| f.name == *required_fn);
            if !has_export {
                return Err(WitError::MissingWorldExport {
                    world: spec_str.to_string(),
                    required: required_path.clone(),
                });
            }
        }

        world.name = spec.world_name.clone();
        world.world_spec = Some(spec);
    }

    Ok((world, warnings))
}

/// Convert a compiler Type to a WitType.
fn type_to_wit(ty: &Type) -> Option<WitType> {
    match ty {
        Type::I32 => Some(WitType::S32),
        Type::I64 => Some(WitType::S64),
        Type::F32 => Some(WitType::F32),
        Type::F64 => Some(WitType::F64),
        Type::Bool => Some(WitType::Bool),
        Type::Char => Some(WitType::Char),
        Type::String => Some(WitType::StringType),
        Type::Unit => None,
        Type::Vec(inner) => type_to_wit(inner).map(|t| WitType::List(Box::new(t))),
        Type::Option(inner) => type_to_wit(inner).map(|t| WitType::Option(Box::new(t))),
        Type::Result(ok, err) => {
            let ok_wit = type_to_wit(ok).map(Box::new);
            let err_wit = type_to_wit(err).map(Box::new);
            Some(WitType::Result {
                ok: ok_wit,
                err: err_wit,
            })
        }
        Type::Struct(id) => Some(WitType::Record(format!("struct-{}", id.0))),
        Type::Enum(id) => Some(WitType::Variant(format!("enum-{}", id.0))),
        Type::Tuple(elems) => {
            let wits: Vec<WitType> = elems.iter().filter_map(type_to_wit).collect();
            if wits.len() == elems.len() {
                Some(WitType::Tuple(wits))
            } else {
                None
            }
        }
        // Closures, type variables, etc. are not WIT-exportable
        _ => None,
    }
}

/// Convert a type name string (from MIR struct/enum defs) to a WitType.
fn type_name_to_wit(name: &str) -> Option<WitType> {
    match name {
        "i32" => Some(WitType::S32),
        "i64" => Some(WitType::S64),
        "f32" => Some(WitType::F32),
        "f64" => Some(WitType::F64),
        "u8" => Some(WitType::U8),
        "u16" => Some(WitType::U16),
        "u32" => Some(WitType::U32),
        "u64" => Some(WitType::U64),
        "bool" => Some(WitType::Bool),
        "char" => Some(WitType::Char),
        "String" => Some(WitType::StringType),
        other => {
            if let Some(inner) = other.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')) {
                type_name_to_wit(inner).map(|t| WitType::List(Box::new(t)))
            } else if let Some(inner) = other
                .strip_prefix("Option<")
                .and_then(|s| s.strip_suffix('>'))
            {
                type_name_to_wit(inner).map(|t| WitType::Option(Box::new(t)))
            } else if let Some(inner) =
                other.strip_prefix("Result<").and_then(|s| s.strip_suffix('>'))
            {
                if let Some((ok_str, err_str)) = split_generic_args(inner) {
                    let ok_wt = type_name_to_wit(ok_str).map(Box::new);
                    let err_wt = type_name_to_wit(err_str).map(Box::new);
                    Some(WitType::Result {
                        ok: ok_wt,
                        err: err_wt,
                    })
                } else {
                    type_name_to_wit(inner)
                        .map(|t| WitType::Result {
                            ok: Some(Box::new(t)),
                            err: None,
                        })
                }
            } else {
                // Named struct/enum — assume record
                Some(WitType::Record(other.to_string()))
            }
        }
    }
}

/// Context-aware type name resolver that checks for special enum types
/// (Option/Result patterns) and tuple struct types before falling back.
fn type_name_to_wit_ctx(
    name: &str,
    special_enums: &std::collections::HashMap<String, WitType>,
    tuple_structs: &std::collections::HashMap<String, WitType>,
) -> Option<WitType> {
    if let Some(wt) = special_enums.get(name) {
        return Some(wt.clone());
    }
    if let Some(wt) = tuple_structs.get(name) {
        return Some(wt.clone());
    }
    type_name_to_wit(name)
}

/// Split two comma-separated generic arguments, respecting nested angle brackets.
fn split_generic_args(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                let first = s[..i].trim();
                let second = s[i + 1..].trim();
                return Some((first, second));
            }
            _ => {}
        }
    }
    None
}

/// Check if an enum definition matches the `Option<T>` pattern.
/// Returns the inner type name if it matches.
fn is_option_enum(variants: &[(String, Vec<String>)]) -> Option<&str> {
    if variants.len() != 2 {
        return None;
    }
    let some_variant = variants.iter().find(|(n, _)| n == "Some");
    let none_variant = variants
        .iter()
        .find(|(n, p)| n == "None" && p.is_empty());
    match (some_variant, none_variant) {
        (Some((_, payloads)), Some(_)) if payloads.len() == 1 => Some(&payloads[0]),
        _ => None,
    }
}

/// Check if an enum definition matches the `Result<T, E>` pattern.
/// Returns (ok_type_name, err_type_name) if it matches.
fn is_result_enum(variants: &[(String, Vec<String>)]) -> Option<(&str, &str)> {
    if variants.len() != 2 {
        return None;
    }
    let ok_variant = variants.iter().find(|(n, _)| n == "Ok");
    let err_variant = variants.iter().find(|(n, _)| n == "Err");
    match (ok_variant, err_variant) {
        (Some((_, ok_p)), Some((_, err_p))) if ok_p.len() == 1 && err_p.len() == 1 => {
            Some((&ok_p[0], &err_p[0]))
        }
        _ => None,
    }
}

/// Collect type names referenced by a WitType (records, variants, enums).
fn collect_wit_type_refs(wt: &WitType, out: &mut std::collections::HashSet<String>) {
    match wt {
        WitType::Record(name) | WitType::Variant(name) => {
            out.insert(name.clone());
        }
        WitType::List(inner) | WitType::Option(inner) => {
            collect_wit_type_refs(inner, out);
        }
        WitType::Result { ok, err } => {
            if let Some(inner) = ok {
                collect_wit_type_refs(inner, out);
            }
            if let Some(inner) = err {
                collect_wit_type_refs(inner, out);
            }
        }
        WitType::Tuple(elems) => {
            for e in elems {
                collect_wit_type_refs(e, out);
            }
        }
        _ => {}
    }
}

/// Validate that all exported functions use only scalar (pass-through) types
/// in the canonical ABI. Returns a list of (function_name, diagnostic) pairs
/// for any function that uses compound types requiring canonical ABI adapters.
pub fn validate_component_export_types(
    world: &WitWorld,
) -> Vec<(String, ark_diagnostics::Diagnostic)> {
    use canonical_abi::{CanonicalAbiClass, classify_wit_type};
    let mut errors = Vec::new();

    for func in &world.functions {
        let all_types: Vec<&WitType> = func
            .params
            .iter()
            .map(|(_, t)| t)
            .chain(func.result.iter())
            .collect();

        for ty in all_types {
            let class = classify_wit_type(ty);
            match &class {
                CanonicalAbiClass::Scalar(_) => {} // pass-through, OK
                CanonicalAbiClass::Handle => {
                    errors.push((
                        func.name.clone(),
                        ark_diagnostics::component_resource_diagnostic(&func.name),
                    ));
                    break;
                }
                _ => {
                    let type_desc = ty.to_wit();
                    errors.push((
                        func.name.clone(),
                        ark_diagnostics::component_compound_type_diagnostic(&func.name, &type_desc),
                    ));
                    break;
                }
            }
        }
    }

    errors
}

/// Validate core Wasm binary: detect exported functions that use GC reference
/// types (which cannot cross the canonical ABI boundary without adapters).
///
/// This catches cases where MIR types are lossy (e.g., struct/enum params
/// reported as I32 in MIR but actually GC refs in Wasm output).
pub fn validate_core_wasm_exports(wasm: &[u8]) -> Vec<(String, ark_diagnostics::Diagnostic)> {
    use wasmparser::{Parser, Payload, ValType};

    let mut errors = Vec::new();
    let mut types: Vec<wasmparser::FuncType> = Vec::new();
    let mut func_types: Vec<u32> = Vec::new();
    let mut num_imports: u32 = 0;

    for payload in Parser::new(0).parse_all(wasm) {
        let payload = match payload {
            Ok(p) => p,
            Err(_) => continue,
        };
        match payload {
            Payload::TypeSection(reader) => {
                for rec_group in reader {
                    let rec_group = match rec_group {
                        Ok(rg) => rg,
                        Err(_) => continue,
                    };
                    for sub_type in rec_group.into_types() {
                        if let wasmparser::CompositeInnerType::Func(func_type) =
                            sub_type.composite_type.inner
                        {
                            types.push(func_type);
                        } else {
                            // Struct/Array GC types — push placeholder
                            types.push(wasmparser::FuncType::new([], []));
                        }
                    }
                }
            }
            Payload::ImportSection(reader) => {
                for import in reader.into_iter().flatten() {
                    if matches!(import.ty, wasmparser::TypeRef::Func(_)) {
                        num_imports += 1;
                    }
                }
            }
            Payload::FunctionSection(reader) => {
                for idx in reader.into_iter().flatten() {
                    func_types.push(idx);
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = match export {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    if export.kind != wasmparser::ExternalKind::Func {
                        continue;
                    }
                    let name = export.name;
                    if name == "_start" || name.starts_with("__") || name == "memory" {
                        continue;
                    }

                    let func_idx = export.index;
                    // Function index = imports + local functions
                    if func_idx < num_imports {
                        continue; // re-exported import, skip
                    }
                    let local_idx = func_idx - num_imports;
                    let type_idx = match func_types.get(local_idx as usize) {
                        Some(&idx) => idx as usize,
                        None => continue,
                    };
                    let func_type = match types.get(type_idx) {
                        Some(ft) => ft,
                        None => continue,
                    };

                    let has_ref_param = func_type
                        .params()
                        .iter()
                        .any(|t| matches!(t, ValType::Ref(_)));
                    let has_ref_result = func_type
                        .results()
                        .iter()
                        .any(|t| matches!(t, ValType::Ref(_)));

                    if has_ref_param || has_ref_result {
                        let desc = if has_ref_param && has_ref_result {
                            "parameter and return type use GC references"
                        } else if has_ref_param {
                            "parameter type uses GC references"
                        } else {
                            "return type uses GC references"
                        };
                        errors.push((
                            name.to_string(),
                            ark_diagnostics::component_compound_type_diagnostic(name, desc),
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    errors
}

/// Convert a WIT type name string (e.g., "s32", "string") to a WitType.
fn wit_type_name_to_wit(name: &str) -> Option<WitType> {
    match name {
        "u8" => Some(WitType::U8),
        "u16" => Some(WitType::U16),
        "u32" => Some(WitType::U32),
        "u64" => Some(WitType::U64),
        "s8" => Some(WitType::S8),
        "s16" => Some(WitType::S16),
        "s32" => Some(WitType::S32),
        "s64" => Some(WitType::S64),
        "f32" => Some(WitType::F32),
        "f64" => Some(WitType::F64),
        "bool" => Some(WitType::Bool),
        "char" => Some(WitType::Char),
        "string" => Some(WitType::StringType),
        other => {
            if let Some(inner) = other
                .strip_prefix("list<")
                .and_then(|s| s.strip_suffix('>'))
            {
                wit_type_name_to_wit(inner).map(|t| WitType::List(Box::new(t)))
            } else if let Some(inner) = other
                .strip_prefix("option<")
                .and_then(|s| s.strip_suffix('>'))
            {
                wit_type_name_to_wit(inner).map(|t| WitType::Option(Box::new(t)))
            } else {
                Some(WitType::Record(other.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_mir::mir::{
        BasicBlock, BlockId, FnId, InstanceKey, LocalId, MirFunction, MirLocal, MirModule,
        SourceInfo,
    };
    use ark_typecheck::types::Type;

    fn make_instance_key() -> InstanceKey {
        InstanceKey {
            item: String::new(),
            substitution: Vec::new(),
            target_shape: String::new(),
        }
    }

    fn make_func(name: &str, params: Vec<(Option<String>, Type)>, ret: Type) -> MirFunction {
        make_func_exported(name, params, ret, true)
    }

    fn make_func_exported(
        name: &str,
        params: Vec<(Option<String>, Type)>,
        ret: Type,
        is_exported: bool,
    ) -> MirFunction {
        let mir_params: Vec<MirLocal> = params
            .into_iter()
            .enumerate()
            .map(|(i, (n, ty))| MirLocal {
                id: LocalId(i as u32),
                name: n,
                ty,
            })
            .collect();
        MirFunction {
            id: FnId(0),
            name: name.to_string(),
            instance: make_instance_key(),
            params: mir_params,
            return_ty: ret,
            locals: Vec::new(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: Vec::new(),
                terminator: ark_mir::mir::Terminator::Return(None),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: Vec::new(),
            source: SourceInfo::unknown(),
            is_exported,
        }
    }

    #[test]
    fn scalar_export_passes_through() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "add",
            vec![(Some("a".into()), Type::I32), (Some("b".into()), Type::I32)],
            Type::I32,
        ));
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert_eq!(world.functions.len(), 1);
        assert_eq!(world.functions[0].name, "add");
        assert!(warnings.is_empty());
    }

    #[test]
    fn string_export_passes_through() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "greet",
            vec![(Some("name".into()), Type::String)],
            Type::String,
        ));
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert_eq!(world.functions.len(), 1);
        assert_eq!(world.functions[0].result, Some(WitType::StringType));
        assert!(warnings.is_empty());
    }

    #[test]
    fn closure_param_excluded_with_warning() {
        let mut mir = MirModule::new();
        let closure_ty = Type::Function {
            params: vec![Type::I32],
            ret: Box::new(Type::I32),
        };
        mir.functions.push(make_func(
            "apply",
            vec![(Some("f".into()), closure_ty)],
            Type::I32,
        ));
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert_eq!(world.functions.len(), 0);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("apply"));
        assert!(warnings[0].contains("non-exportable"));
    }

    #[test]
    fn non_exportable_return_type_warns() {
        let mut mir = MirModule::new();
        let closure_ret = Type::Function {
            params: vec![],
            ret: Box::new(Type::I32),
        };
        mir.functions.push(make_func("get_fn", vec![], closure_ret));
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert_eq!(world.functions.len(), 0);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("get_fn"));
    }

    #[test]
    fn main_and_internal_excluded_silently() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func("main", vec![], Type::Unit));
        mir.functions.push(make_func("__helper", vec![], Type::I32));
        mir.functions.push(make_func("_start", vec![], Type::Unit));
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert_eq!(world.functions.len(), 0);
        assert!(warnings.is_empty());
    }

    #[test]
    fn option_enum_detected_as_option_type() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "maybe_value",
            vec![],
            Type::Option(Box::new(Type::I32)),
        ));
        mir.type_table.enum_defs.insert(
            "Option".to_string(),
            vec![
                ("Some".to_string(), vec!["i32".to_string()]),
                ("None".to_string(), vec![]),
            ],
        );
        let (world, _) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        // Option should NOT be emitted as a variant
        assert!(world.variants.is_empty(), "Option should not be a variant");
        // The function should return option<s32>
        assert_eq!(
            world.functions[0].result,
            Some(WitType::Option(Box::new(WitType::S32)))
        );
    }

    #[test]
    fn result_enum_detected_as_result_type() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "try_parse",
            vec![(Some("s".into()), Type::String)],
            Type::Result(Box::new(Type::I32), Box::new(Type::String)),
        ));
        mir.type_table.enum_defs.insert(
            "Result".to_string(),
            vec![
                ("Ok".to_string(), vec!["i32".to_string()]),
                ("Err".to_string(), vec!["String".to_string()]),
            ],
        );
        let (world, _) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert!(
            world.variants.is_empty(),
            "Result should not be a variant"
        );
        assert_eq!(
            world.functions[0].result,
            Some(WitType::Result {
                ok: Some(Box::new(WitType::S32)),
                err: Some(Box::new(WitType::StringType)),
            })
        );
    }

    #[test]
    fn specialized_result_enum_skipped() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "try_parse_f64",
            vec![],
            Type::Result(Box::new(Type::F64), Box::new(Type::String)),
        ));
        mir.type_table.enum_defs.insert(
            "Result_f64_String".to_string(),
            vec![
                ("Ok".to_string(), vec!["f64".to_string()]),
                ("Err".to_string(), vec!["String".to_string()]),
            ],
        );
        let (world, _) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert!(
            world.variants.is_empty(),
            "Result_f64_String should not be a variant"
        );
    }

    #[test]
    fn tuple_type_generates_wit_tuple() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "get_pair",
            vec![],
            Type::Tuple(vec![Type::I32, Type::I32]),
        ));
        let (world, _) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert_eq!(
            world.functions[0].result,
            Some(WitType::Tuple(vec![WitType::S32, WitType::S32]))
        );
        let wit = generate_wit(&world).unwrap();
        assert!(
            wit.contains("tuple<s32, s32>"),
            "WIT should contain tuple<s32, s32>, got: {}",
            wit
        );
    }

    #[test]
    fn struct_field_with_option_resolves_inline() {
        let mut mir = MirModule::new();
        // A struct with a field that references "Option" by name
        mir.type_table.struct_defs.insert(
            "Config".to_string(),
            vec![
                ("name".to_string(), "String".to_string()),
                ("value".to_string(), "Option".to_string()),
            ],
        );
        // Register Option enum so it can be resolved
        mir.type_table.enum_defs.insert(
            "Option".to_string(),
            vec![
                ("Some".to_string(), vec!["i32".to_string()]),
                ("None".to_string(), vec![]),
            ],
        );
        // Create a function that uses the struct
        mir.functions.push(make_func(
            "get_config",
            vec![],
            Type::Struct(ark_typecheck::types::TypeId(0)),
        ));
        // Manually add "Config" to the struct's type reference
        // (since Type::Struct(id) → WitType::Record("struct-0") won't match "Config")
        // We test the inline resolution via type_name_to_wit_ctx directly
        let special_enums = {
            let mut m = std::collections::HashMap::new();
            m.insert(
                "Option".to_string(),
                WitType::Option(Box::new(WitType::S32)),
            );
            m
        };
        let tuple_structs = std::collections::HashMap::new();
        let resolved =
            type_name_to_wit_ctx("Option", &special_enums, &tuple_structs);
        assert_eq!(resolved, Some(WitType::Option(Box::new(WitType::S32))));
    }

    #[test]
    fn type_name_result_parsing() {
        let r = type_name_to_wit("Result<i32, String>");
        assert_eq!(
            r,
            Some(WitType::Result {
                ok: Some(Box::new(WitType::S32)),
                err: Some(Box::new(WitType::StringType)),
            })
        );
    }

    #[test]
    fn type_name_u_types() {
        assert_eq!(type_name_to_wit("u8"), Some(WitType::U8));
        assert_eq!(type_name_to_wit("u16"), Some(WitType::U16));
        assert_eq!(type_name_to_wit("u32"), Some(WitType::U32));
        assert_eq!(type_name_to_wit("u64"), Some(WitType::U64));
    }

    #[test]
    fn kebab_case_applied_to_exports() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "get_value",
            vec![(Some("my_param".into()), Type::I32)],
            Type::I32,
        ));
        let (world, _) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        let wit = generate_wit(&world).unwrap();
        assert!(
            wit.contains("get-value"),
            "snake_case should be kebab-case: {}",
            wit
        );
        assert!(
            wit.contains("my-param"),
            "param should be kebab-case: {}",
            wit
        );
    }

    #[test]
    fn non_option_result_enum_still_emitted() {
        let mut mir = MirModule::new();
        // A regular enum that doesn't match Option/Result pattern
        mir.type_table.enum_defs.insert(
            "Color".to_string(),
            vec![
                ("Red".to_string(), vec![]),
                ("Green".to_string(), vec![]),
                ("Blue".to_string(), vec![]),
            ],
        );
        // We need a function that references "Color" so it's in referenced_types
        mir.functions.push(make_func(
            "get_color",
            vec![],
            Type::Struct(ark_typecheck::types::TypeId(0)),
        ));
        // Since Type::Struct → WitType::Record("struct-0"), Color won't be referenced
        // through functions. Manually test the detection helpers:
        let variants = vec![
            ("Red".to_string(), vec![]),
            ("Green".to_string(), vec![]),
            ("Blue".to_string(), vec![]),
        ];
        assert!(
            is_option_enum(&variants).is_none(),
            "Color should not match Option pattern"
        );
        assert!(
            is_result_enum(&variants).is_none(),
            "Color should not match Result pattern"
        );
    }

    #[test]
    fn mixed_exportable_and_not() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "add",
            vec![(Some("x".into()), Type::I32)],
            Type::I32,
        ));
        mir.functions.push(make_func(
            "apply",
            vec![(
                Some("f".into()),
                Type::Function {
                    params: vec![Type::I32],
                    ret: Box::new(Type::I32),
                },
            )],
            Type::I32,
        ));
        mir.functions.push(make_func(
            "len",
            vec![(Some("s".into()), Type::String)],
            Type::I32,
        ));
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test", None).unwrap();
        assert_eq!(world.functions.len(), 2);
        assert_eq!(world.functions[0].name, "add");
        assert_eq!(world.functions[1].name, "len");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn world_spec_cli_command_with_run() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func("run", vec![], Type::Unit));
        let (world, _) =
            mir_to_wit_world_with_warnings(&mir, "test", Some("wasi:cli/command")).unwrap();
        assert_eq!(world.name, "command");
        assert!(world.world_spec.is_some());
        let spec = world.world_spec.unwrap();
        assert!(spec.use_imports.iter().any(|s| s.contains("stdin")));
    }

    #[test]
    fn world_spec_cli_command_missing_run_errors() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func(
            "add",
            vec![(Some("a".into()), Type::I32)],
            Type::I32,
        ));
        let result = mir_to_wit_world_with_warnings(&mir, "test", Some("wasi:cli/command"));
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("wasi:cli/run/run"));
        assert!(err.contains("no matching function found"));
    }

    #[test]
    fn world_spec_http_proxy() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func("handle", vec![], Type::Unit));
        let (world, _) =
            mir_to_wit_world_with_warnings(&mir, "test", Some("wasi:http/proxy")).unwrap();
        assert_eq!(world.name, "proxy");
    }

    #[test]
    fn world_spec_unknown_errors() {
        let mir = MirModule::new();
        let result = mir_to_wit_world_with_warnings(&mir, "test", Some("wasi:unknown/thing"));
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("unknown world"));
    }

    #[test]
    fn world_spec_none_preserves_default_name() {
        let mut mir = MirModule::new();
        mir.functions.push(make_func("add", vec![], Type::I32));
        let (world, _) = mir_to_wit_world_with_warnings(&mir, "myapp", None).unwrap();
        assert_eq!(world.name, "myapp");
        assert!(world.world_spec.is_none());
    }
}
