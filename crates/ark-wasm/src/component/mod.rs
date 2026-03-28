//! WIT / Component Model generation for T3 targets.
//!
//! Generates WIT (Wasm Interface Type) descriptions from the compiler's
//! public API surface. Component wrapping uses external `wasm-tools`.

mod wit;
pub mod wit_parse;

pub use wit::{
    WitEnum, WitError, WitFunction, WitRecord, WitType, WitVariant, WitWorld, generate_wit,
};
pub use wit_parse::{WitDocument, WitInterface, WitParseError, parse_wit, wit_interface_to_mir_imports};

use ark_mir::mir::MirModule;
use ark_typecheck::types::Type;

/// Convert a MIR module into a WIT world descriptor.
///
/// Extracts all non-internal, non-main functions as exports.
/// Converts struct_defs and enum_defs to WIT records/variants.
pub fn mir_to_wit_world(mir: &MirModule, world_name: &str) -> Result<WitWorld, WitError> {
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
            .filter_map(|(i, ty)| {
                wit_type_name_to_wit(ty).map(|wt| (format!("p{}", i), wt))
            })
            .collect();
        let result = imp.return_type.as_ref().and_then(|ty| wit_type_name_to_wit(ty));
        world.imports.push(WitFunction {
            name: imp.name.clone(),
            params,
            result,
        });
    }

    // Convert struct definitions to WIT records (from type table)
    for (name, fields) in &mir.type_table.struct_defs {
        if name.starts_with("__") {
            continue; // skip internal structs
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

    // Convert enum definitions to WIT variants (from type table)
    for (name, variants) in &mir.type_table.enum_defs {
        if name.starts_with("__") {
            continue;
        }
        let all_unit = variants.iter().all(|(_, payloads)| payloads.is_empty());
        if all_unit {
            // Simple enum (no payloads)
            world.enums.push(WitEnum {
                name: name.clone(),
                variants: variants.iter().map(|(vname, _)| vname.clone()).collect(),
            });
        } else {
            // Variant with payloads
            let cases: Vec<(String, Option<WitType>)> = variants
                .iter()
                .map(|(vname, payloads)| {
                    if payloads.is_empty() {
                        (vname.clone(), None)
                    } else if payloads.len() == 1 {
                        (vname.clone(), type_name_to_wit(&payloads[0]))
                    } else {
                        // Multi-field payload → tuple
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

    // Convert public functions to WIT exports
    for func in &mir.functions {
        let name = &func.name;
        // Skip internal/helper functions
        if name.starts_with("__") || name == "_start" {
            continue;
        }
        // Skip main — it's the entrypoint, not an export
        if name == "main" {
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

        // Skip functions with non-exportable parameter types
        if params.len() != func.params.len() {
            continue;
        }

        let result = type_to_wit(&func.return_ty);
        // Unit return → no WIT result
        let result = match &func.return_ty {
            Type::Unit => None,
            _ => result,
        };

        world.functions.push(WitFunction {
            name: name.clone(),
            params,
            result,
        });
    }

    Ok(world)
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
            if let Some(inner) = other.strip_prefix("list<").and_then(|s| s.strip_suffix('>')) {
                wit_type_name_to_wit(inner).map(|t| WitType::List(Box::new(t)))
            } else if let Some(inner) =
                other.strip_prefix("option<").and_then(|s| s.strip_suffix('>'))
            {
                wit_type_name_to_wit(inner).map(|t| WitType::Option(Box::new(t)))
            } else {
                Some(WitType::Record(other.to_string()))
            }
        }
    }
}
