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
    WitEnum, WitError, WitFunction, WitRecord, WitType, WitVariant, WitWorld, generate_wit,
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
    mir_to_wit_world_with_warnings(mir, world_name).map(|(world, _)| world)
}

/// Like `mir_to_wit_world` but also returns diagnostic warnings for non-exportable functions.
pub fn mir_to_wit_world_with_warnings(
    mir: &MirModule,
    world_name: &str,
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

    // Convert struct definitions to WIT records — only if referenced by exports
    for (name, fields) in &mir.type_table.struct_defs {
        if name.starts_with("__") || !referenced_types.contains(name.as_str()) {
            continue;
        }
        let wit_fields: Vec<(String, WitType)> = fields
            .iter()
            .filter_map(|(fname, ftype)| type_name_to_wit(ftype).map(|wt| (fname.clone(), wt)))
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
                        (vname.clone(), type_name_to_wit(&payloads[0]))
                    } else {
                        let elems: Vec<WitType> = payloads
                            .iter()
                            .filter_map(|t| type_name_to_wit(t))
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
            } else {
                // Named struct/enum — assume record
                Some(WitType::Record(other.to_string()))
            }
        }
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
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test").unwrap();
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
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test").unwrap();
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
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test").unwrap();
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
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test").unwrap();
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
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test").unwrap();
        assert_eq!(world.functions.len(), 0);
        assert!(warnings.is_empty());
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
        let (world, warnings) = mir_to_wit_world_with_warnings(&mir, "test").unwrap();
        assert_eq!(world.functions.len(), 2);
        assert_eq!(world.functions[0].name, "add");
        assert_eq!(world.functions[1].name, "len");
        assert_eq!(warnings.len(), 1);
    }
}
