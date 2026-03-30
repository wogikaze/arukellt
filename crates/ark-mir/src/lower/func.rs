//! Function lowering and free variable collection for MIR.

use std::collections::{HashMap, HashSet};

use ark_diagnostics::DiagnosticSink;
use ark_parser::ast;
use ark_typecheck::TypeChecker;
use ark_typecheck::types::Type as CheckerType;

use crate::mir::*;

use super::LowerCtx;
use super::types::{is_string_type, is_void_expr, type_expr_name};
use super::{
    fallback_block, fallback_function, finalize_lowered_module, push_function, type_to_sig_name,
};

#[allow(dead_code)]
fn fn_id_name(func: FnId) -> String {
    format!("fn#{}", func.0)
}

/// Convert a type-checker `Type` to an AST `TypeExpr` for Result/Option propagation.
/// Returns `None` for types that don't need propagation (primitives, etc.).
fn type_to_type_expr(ty: &CheckerType) -> Option<ast::TypeExpr> {
    let dummy = ark_diagnostics::Span::new(0, 0, 0);
    match ty {
        CheckerType::I32 => Some(ast::TypeExpr::Named {
            name: "i32".into(),
            span: dummy,
        }),
        CheckerType::I64 => Some(ast::TypeExpr::Named {
            name: "i64".into(),
            span: dummy,
        }),
        CheckerType::F32 => Some(ast::TypeExpr::Named {
            name: "f32".into(),
            span: dummy,
        }),
        CheckerType::F64 => Some(ast::TypeExpr::Named {
            name: "f64".into(),
            span: dummy,
        }),
        CheckerType::Bool => Some(ast::TypeExpr::Named {
            name: "bool".into(),
            span: dummy,
        }),
        CheckerType::String => Some(ast::TypeExpr::Named {
            name: "String".into(),
            span: dummy,
        }),
        CheckerType::Unit => Some(ast::TypeExpr::Unit(dummy)),
        CheckerType::Result(ok, err) => {
            let ok_te = type_to_type_expr(ok)?;
            let err_te = type_to_type_expr(err)?;
            Some(ast::TypeExpr::Generic {
                name: "Result".into(),
                args: vec![ok_te, err_te],
                span: dummy,
            })
        }
        CheckerType::Option(inner) => {
            let inner_te = type_to_type_expr(inner)?;
            Some(ast::TypeExpr::Generic {
                name: "Option".into(),
                args: vec![inner_te],
                span: dummy,
            })
        }
        CheckerType::Vec(elem) => {
            let elem_te = type_to_type_expr(elem)?;
            Some(ast::TypeExpr::Generic {
                name: "Vec".into(),
                args: vec![elem_te],
                span: dummy,
            })
        }
        _ => None,
    }
}

/// Lower a type-checked module to MIR.
pub fn lower_to_mir(
    module: &ast::Module,
    checker: &TypeChecker,
    _sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = MirModule::new();
    let mut next_fn_id = 0u32;

    // Collect enum variant tags: "EnumName::Variant" -> tag index
    let mut enum_tags: HashMap<String, i32> = HashMap::new();
    // Collect enum variant info: enum_name -> [(variant_name, field_count)]
    let mut enum_variants: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    // Reverse lookup: "EnumName::Variant" -> enum_name
    let mut variant_to_enum: HashMap<String, String> = HashMap::new();
    // Bare variant names (for prelude types like Option/Result): name -> (enum, tag, field_count)
    let mut bare_variant_tags: HashMap<String, (String, i32, usize)> = HashMap::new();
    // Collect struct definitions: "StructName" -> field names (ordered)
    let mut struct_defs: HashMap<String, Vec<(String, String)>> = HashMap::new();
    // Collect enum definitions: "EnumName" -> [(variant_name, [payload_type_names])]
    let mut enum_defs: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();
    // Collect enum struct variant field names: "EnumName::VariantName" -> [field_names]
    let mut enum_variant_field_names: HashMap<String, Vec<String>> = HashMap::new();

    // Inject builtin enum types: Option<T> and Result<T, E>
    #[allow(clippy::type_complexity)]
    let builtin_enums: &[(&str, &[(&str, &[&str])])] = &[
        ("Option", &[("Some", &["i32"]), ("None", &[])]),
        ("Result", &[("Ok", &["i32"]), ("Err", &["String"])]),
    ];
    for &(enum_name, variants) in builtin_enums {
        let mut variants_info = Vec::new();
        let mut variants_defs = Vec::new();
        for (i, &(vname, field_types)) in variants.iter().enumerate() {
            let key = format!("{}::{}", enum_name, vname);
            enum_tags.insert(key.clone(), i as i32);
            variant_to_enum.insert(key, enum_name.to_string());
            variants_info.push((vname.to_string(), field_types.len()));
            bare_variant_tags.insert(
                vname.to_string(),
                (enum_name.to_string(), i as i32, field_types.len()),
            );
            let payload_types: Vec<String> = field_types.iter().map(|t| t.to_string()).collect();
            variants_defs.push((vname.to_string(), payload_types));
        }
        enum_variants.insert(enum_name.to_string(), variants_info);
        enum_defs.insert(enum_name.to_string(), variants_defs);
    }

    // Specialized Result enums for i64/f64 payloads
    enum_defs.insert(
        "Result_i64_String".to_string(),
        vec![
            ("Ok".to_string(), vec!["i64".to_string()]),
            ("Err".to_string(), vec!["String".to_string()]),
        ],
    );
    enum_defs.insert(
        "Result_f64_String".to_string(),
        vec![
            ("Ok".to_string(), vec!["f64".to_string()]),
            ("Err".to_string(), vec!["String".to_string()]),
        ],
    );
    enum_defs.insert(
        "Result_String_String".to_string(),
        vec![
            ("Ok".to_string(), vec!["String".to_string()]),
            ("Err".to_string(), vec!["String".to_string()]),
        ],
    );
    enum_defs.insert(
        "Option_String".to_string(),
        vec![
            ("Some".to_string(), vec!["String".to_string()]),
            ("None".to_string(), vec![]),
        ],
    );

    for item in &module.items {
        if let ast::Item::EnumDef(e) = item {
            let mut variants_info = Vec::new();
            let mut variants_defs = Vec::new();
            for (i, variant) in e.variants.iter().enumerate() {
                let (vname, field_count, payload_types) = match variant {
                    ast::Variant::Unit { name, .. } => (name.clone(), 0, vec![]),
                    ast::Variant::Tuple { name, fields, .. } => {
                        let types: Vec<String> = fields.iter().map(type_expr_name).collect();
                        (name.clone(), fields.len(), types)
                    }
                    ast::Variant::Struct { name, fields, .. } => {
                        let types: Vec<String> =
                            fields.iter().map(|f| type_expr_name(&f.ty)).collect();
                        let fnames: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
                        let key = format!("{}::{}", e.name, name);
                        enum_variant_field_names.insert(key, fnames);
                        (name.clone(), fields.len(), types)
                    }
                };
                let key = format!("{}::{}", e.name, vname);
                enum_tags.insert(key.clone(), i as i32);
                variant_to_enum.insert(key, e.name.clone());
                variants_info.push((vname.clone(), field_count));
                variants_defs.push((vname.clone(), payload_types));
                // Register bare variant name for common prelude types
                bare_variant_tags.insert(vname.clone(), (e.name.clone(), i as i32, field_count));
            }
            enum_variants.insert(e.name.clone(), variants_info);
            enum_defs.insert(e.name.clone(), variants_defs);
        }
        if let ast::Item::StructDef(s) = item {
            let fields: Vec<(String, String)> = s
                .fields
                .iter()
                .map(|f| {
                    let type_name = type_expr_name(&f.ty);
                    (f.name.clone(), type_name)
                })
                .collect();
            struct_defs.insert(s.name.clone(), fields);
        }
    }

    // Register tuple struct layouts for common arities
    for arity in 2..=4u32 {
        let name = format!("__tuple{}", arity);
        let fields: Vec<(String, String)> = (0..arity)
            .map(|i| (i.to_string(), "i32".to_string()))
            .collect();
        struct_defs.insert(name, fields);
    }
    // Register anyref-field tuple structs for generic function tuple returns
    for arity in 2..=4u32 {
        let name = format!("__tuple{}_any", arity);
        let fields: Vec<(String, String)> = (0..arity)
            .map(|i| (i.to_string(), "anyref".to_string()))
            .collect();
        struct_defs.insert(name, fields);
    }

    // Build fn_return_types map for resolving generic enum payloads in match
    let mut fn_return_types: HashMap<String, ast::TypeExpr> = HashMap::new();
    let mut user_fn_names: HashSet<String> = HashSet::new();
    for item in &module.items {
        if let ast::Item::FnDef(f) = item {
            user_fn_names.insert(f.name.clone());
            if let Some(ret_ty) = &f.return_type {
                fn_return_types.insert(f.name.clone(), ret_ty.clone());
            }
        }
        // Register impl method names with mangled format
        if let ast::Item::ImplBlock(ib) = item {
            for method in &ib.methods {
                let mangled = format!("{}__{}", ib.target_type, method.name);
                user_fn_names.insert(mangled.clone());
                if let Some(ret_ty) = &method.return_type {
                    fn_return_types.insert(mangled, ret_ty.clone());
                }
            }
        }
    }

    // Populate fn_return_types from checker's builtin FnSigs so that
    // Result-returning builtins (fs_read_file, parse_i32, …) work without
    // explicit type annotations on let bindings.
    for (name, sig) in checker.fn_sigs_iter() {
        if fn_return_types.contains_key(name) {
            continue; // user-defined function takes precedence
        }
        if let Some(te) = type_to_type_expr(&sig.ret) {
            fn_return_types.insert(name.clone(), te);
        }
    }

    // Get method resolutions from the type checker
    let method_resolutions = checker.method_resolutions_snapshot();

    // Build set of generic function names
    let mut generic_fn_names: HashSet<String> = HashSet::new();
    for item in &module.items {
        if let ast::Item::FnDef(f) = item
            && !f.type_params.is_empty()
        {
            generic_fn_names.insert(f.name.clone());
        }
        if let ast::Item::ImplBlock(ib) = item {
            for method in &ib.methods {
                if !method.type_params.is_empty() {
                    let mangled = format!("{}__{}", ib.target_type, method.name);
                    generic_fn_names.insert(mangled);
                }
            }
        }
    }

    for item in &module.items {
        if let ast::Item::FnDef(f) = item {
            let fn_id = FnId(next_fn_id);
            next_fn_id += 1;

            let mut ctx = LowerCtx::new(
                enum_tags.clone(),
                struct_defs.clone(),
                enum_variants.clone(),
                variant_to_enum.clone(),
                bare_variant_tags.clone(),
                enum_defs.clone(),
                enum_variant_field_names.clone(),
                fn_return_types.clone(),
                user_fn_names.clone(),
                method_resolutions.clone(),
                f.type_params.clone(),
                generic_fn_names.clone(),
            );

            for param in &f.params {
                let pid = ctx.declare_local(&param.name);
                if is_string_type(&param.ty) {
                    ctx.string_locals.insert(pid.0);
                }
                // Track i64/f64-typed parameters
                if let ast::TypeExpr::Named { name: tname, .. } = &param.ty {
                    if tname == "i64" {
                        ctx.i64_locals.insert(pid.0);
                    }
                    if tname == "f64" {
                        ctx.f64_locals.insert(pid.0);
                    }
                }
                // Track struct-typed parameters
                if let ast::TypeExpr::Named { name: tname, .. } = &param.ty {
                    if ctx.struct_defs.contains_key(tname.as_str()) {
                        ctx.struct_typed_locals.insert(pid.0, tname.clone());
                    }
                    if ctx.enum_variants.contains_key(tname.as_str()) {
                        ctx.enum_typed_locals.insert(pid.0, tname.clone());
                    }
                }
                if let ast::TypeExpr::Generic { name: tname, .. } = &param.ty
                    && ctx.enum_variants.contains_key(tname.as_str())
                {
                    ctx.enum_typed_locals.insert(pid.0, tname.clone());
                }
            }

            let entry = BlockId(0);
            let mut stmts = ctx.lower_block(&f.body);

            // Handle tail expression: lower as a statement (void) when the
            // function declares no return type (unit), or when the expression
            // is a known void call. Otherwise treat it as the return value.
            let fn_returns_unit =
                f.return_type.is_none() || matches!(&f.return_type, Some(ast::TypeExpr::Unit(_)));
            let tail_op = if let Some(tail) = &f.body.tail_expr {
                if fn_returns_unit || is_void_expr(tail) {
                    ctx.lower_expr_stmt(tail, &mut stmts);
                    None
                } else {
                    Some(ctx.lower_expr(tail))
                }
            } else {
                None
            };

            let mir_fn = fallback_function(
                fn_id,
                f.name.clone(),
                f.params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| MirLocal {
                        id: LocalId(i as u32),
                        name: Some(p.name.clone()),
                        ty: match &p.ty {
                            ty if is_string_type(ty) => ark_typecheck::types::Type::String,
                            ast::TypeExpr::Named { name, .. } if name == "f64" => {
                                ark_typecheck::types::Type::F64
                            }
                            ast::TypeExpr::Named { name, .. } if name == "f32" => {
                                ark_typecheck::types::Type::F32
                            }
                            ast::TypeExpr::Named { name, .. } if name == "i64" => {
                                ark_typecheck::types::Type::I64
                            }
                            ast::TypeExpr::Named { name, .. } if name == "bool" => {
                                ark_typecheck::types::Type::Bool
                            }
                            ast::TypeExpr::Named { name, .. } if name == "char" => {
                                ark_typecheck::types::Type::Char
                            }
                            ast::TypeExpr::Named { name, .. } if f.type_params.contains(name) => {
                                ark_typecheck::types::Type::Any
                            }
                            _ => ark_typecheck::types::Type::I32,
                        },
                    })
                    .collect(),
                match &f.return_type {
                    Some(ty) if is_string_type(ty) => ark_typecheck::types::Type::String,
                    Some(ast::TypeExpr::Named { name, .. }) if name == "f64" => {
                        ark_typecheck::types::Type::F64
                    }
                    Some(ast::TypeExpr::Named { name, .. }) if name == "f32" => {
                        ark_typecheck::types::Type::F32
                    }
                    Some(ast::TypeExpr::Named { name, .. }) if name == "i64" => {
                        ark_typecheck::types::Type::I64
                    }
                    Some(ast::TypeExpr::Named { name, .. }) if name == "bool" => {
                        ark_typecheck::types::Type::Bool
                    }
                    Some(ast::TypeExpr::Named { name, .. }) if name == "char" => {
                        ark_typecheck::types::Type::Char
                    }
                    Some(ast::TypeExpr::Named { name, .. }) if f.type_params.contains(name) => {
                        ark_typecheck::types::Type::Any
                    }
                    Some(_) => ark_typecheck::types::Type::I32,
                    None => ark_typecheck::types::Type::Unit,
                },
                ctx.locals
                    .iter()
                    .map(|(name, id)| MirLocal {
                        id: *id,
                        name: Some(name.clone()),
                        ty: if ctx.string_locals.contains(&id.0) {
                            ark_typecheck::types::Type::String
                        } else if ctx.f64_locals.contains(&id.0) {
                            ark_typecheck::types::Type::F64
                        } else if ctx.i64_locals.contains(&id.0) {
                            ark_typecheck::types::Type::I64
                        } else if ctx.bool_locals.contains(&id.0) {
                            ark_typecheck::types::Type::Bool
                        } else if ctx.char_locals.contains(&id.0) {
                            ark_typecheck::types::Type::Char
                        } else if ctx.vec_string_locals.contains(&id.0) {
                            ark_typecheck::types::Type::Vec(Box::new(
                                ark_typecheck::types::Type::String,
                            ))
                        } else if ctx.vec_i64_locals.contains(&id.0) {
                            ark_typecheck::types::Type::Vec(Box::new(
                                ark_typecheck::types::Type::I64,
                            ))
                        } else if ctx.vec_f64_locals.contains(&id.0) {
                            ark_typecheck::types::Type::Vec(Box::new(
                                ark_typecheck::types::Type::F64,
                            ))
                        } else if ctx.vec_i32_locals.contains(&id.0) {
                            ark_typecheck::types::Type::Vec(Box::new(
                                ark_typecheck::types::Type::I32,
                            ))
                        } else {
                            ark_typecheck::types::Type::I32
                        },
                    })
                    .collect(),
                vec![fallback_block(
                    entry,
                    stmts,
                    if let Some(op) = tail_op {
                        Terminator::Return(Some(op))
                    } else {
                        Terminator::Return(None)
                    },
                )],
                entry,
                ctx.struct_typed_locals.clone(),
                ctx.enum_typed_locals.clone(),
                f.type_params.clone(),
                f.is_pub,
            );

            if f.name == "main" {
                mir.entry_fn = Some(fn_id);
            }

            push_function(&mut mir, mir_fn);

            // Collect any synthetic closure functions generated during this function's lowering
            for mut closure_fn in ctx.pending_closures.drain(..) {
                let closure_id = FnId(mir.functions.len() as u32);
                closure_fn.id = closure_id;
                push_function(&mut mir, closure_fn);
            }
            // Keep next_fn_id ahead of all pushed functions (including closures).
            next_fn_id = next_fn_id.max(mir.functions.len() as u32);
        }
        // Lower impl method bodies as regular functions with mangled names
        if let ast::Item::ImplBlock(ib) = item {
            for method in &ib.methods {
                let fn_id = FnId(next_fn_id);
                next_fn_id += 1;
                let mangled = format!("{}__{}", ib.target_type, method.name);

                let mut ctx = LowerCtx::new(
                    enum_tags.clone(),
                    struct_defs.clone(),
                    enum_variants.clone(),
                    variant_to_enum.clone(),
                    bare_variant_tags.clone(),
                    enum_defs.clone(),
                    enum_variant_field_names.clone(),
                    fn_return_types.clone(),
                    user_fn_names.clone(),
                    method_resolutions.clone(),
                    method.type_params.clone(),
                    generic_fn_names.clone(),
                );

                for param in &method.params {
                    let pid = ctx.declare_local(&param.name);
                    if is_string_type(&param.ty) {
                        ctx.string_locals.insert(pid.0);
                    }
                    if let ast::TypeExpr::Named { name: tname, .. } = &param.ty {
                        if tname == "f64" {
                            ctx.f64_locals.insert(pid.0);
                        }
                        if ctx.struct_defs.contains_key(tname.as_str()) {
                            ctx.struct_typed_locals.insert(pid.0, tname.clone());
                        }
                        if ctx.enum_variants.contains_key(tname.as_str()) {
                            ctx.enum_typed_locals.insert(pid.0, tname.clone());
                        }
                    }
                    if let ast::TypeExpr::Generic { name: tname, .. } = &param.ty
                        && ctx.enum_variants.contains_key(tname.as_str())
                    {
                        ctx.enum_typed_locals.insert(pid.0, tname.clone());
                    }
                }

                let entry = BlockId(0);
                let mut stmts = ctx.lower_block(&method.body);

                let method_returns_unit = method.return_type.is_none()
                    || matches!(&method.return_type, Some(ast::TypeExpr::Unit(_)));
                let tail_op = if let Some(tail) = &method.body.tail_expr {
                    if method_returns_unit || is_void_expr(tail) {
                        ctx.lower_expr_stmt(tail, &mut stmts);
                        None
                    } else {
                        Some(ctx.lower_expr(tail))
                    }
                } else {
                    None
                };

                let mir_fn = fallback_function(
                    fn_id,
                    mangled,
                    method
                        .params
                        .iter()
                        .enumerate()
                        .map(|(i, p)| MirLocal {
                            id: LocalId(i as u32),
                            name: Some(p.name.clone()),
                            ty: match &p.ty {
                                ty if is_string_type(ty) => ark_typecheck::types::Type::String,
                                ast::TypeExpr::Named { name, .. } if name == "f64" => {
                                    ark_typecheck::types::Type::F64
                                }
                                ast::TypeExpr::Named { name, .. } if name == "f32" => {
                                    ark_typecheck::types::Type::F32
                                }
                                ast::TypeExpr::Named { name, .. } if name == "i64" => {
                                    ark_typecheck::types::Type::I64
                                }
                                ast::TypeExpr::Named { name, .. } if name == "bool" => {
                                    ark_typecheck::types::Type::Bool
                                }
                                ast::TypeExpr::Named { name, .. } if name == "char" => {
                                    ark_typecheck::types::Type::Char
                                }
                                _ => ark_typecheck::types::Type::I32,
                            },
                        })
                        .collect(),
                    match &method.return_type {
                        Some(ty) if is_string_type(ty) => ark_typecheck::types::Type::String,
                        Some(ast::TypeExpr::Named { name, .. }) if name == "f64" => {
                            ark_typecheck::types::Type::F64
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "f32" => {
                            ark_typecheck::types::Type::F32
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "i64" => {
                            ark_typecheck::types::Type::I64
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "bool" => {
                            ark_typecheck::types::Type::Bool
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "char" => {
                            ark_typecheck::types::Type::Char
                        }
                        Some(_) => ark_typecheck::types::Type::I32,
                        None => ark_typecheck::types::Type::Unit,
                    },
                    ctx.locals
                        .iter()
                        .map(|(name, id)| MirLocal {
                            id: *id,
                            name: Some(name.clone()),
                            ty: if ctx.string_locals.contains(&id.0) {
                                ark_typecheck::types::Type::String
                            } else if ctx.f64_locals.contains(&id.0) {
                                ark_typecheck::types::Type::F64
                            } else if ctx.i64_locals.contains(&id.0) {
                                ark_typecheck::types::Type::I64
                            } else if ctx.bool_locals.contains(&id.0) {
                                ark_typecheck::types::Type::Bool
                            } else if ctx.char_locals.contains(&id.0) {
                                ark_typecheck::types::Type::Char
                            } else if ctx.vec_string_locals.contains(&id.0) {
                                ark_typecheck::types::Type::Vec(Box::new(
                                    ark_typecheck::types::Type::String,
                                ))
                            } else if ctx.vec_i64_locals.contains(&id.0) {
                                ark_typecheck::types::Type::Vec(Box::new(
                                    ark_typecheck::types::Type::I64,
                                ))
                            } else if ctx.vec_f64_locals.contains(&id.0) {
                                ark_typecheck::types::Type::Vec(Box::new(
                                    ark_typecheck::types::Type::F64,
                                ))
                            } else if ctx.vec_i32_locals.contains(&id.0) {
                                ark_typecheck::types::Type::Vec(Box::new(
                                    ark_typecheck::types::Type::I32,
                                ))
                            } else {
                                ark_typecheck::types::Type::I32
                            },
                        })
                        .collect(),
                    vec![fallback_block(
                        entry,
                        stmts,
                        if let Some(op) = tail_op {
                            Terminator::Return(Some(op))
                        } else {
                            Terminator::Return(None)
                        },
                    )],
                    entry,
                    ctx.struct_typed_locals.clone(),
                    ctx.enum_typed_locals.clone(),
                    method.type_params.clone(),
                    false, // impl methods are not component exports
                );

                push_function(&mut mir, mir_fn);

                for mut closure_fn in ctx.pending_closures.drain(..) {
                    let closure_id = FnId(mir.functions.len() as u32);
                    closure_fn.id = closure_id;
                    push_function(&mut mir, closure_fn);
                }

                next_fn_id = next_fn_id.max(mir.functions.len() as u32);

                continue;
            }
        }
    }

    // Build reverse maps from TypeId → name for struct/enum types
    let struct_id_to_name: HashMap<u32, String> = checker
        .struct_defs_iter()
        .map(|(name, info)| (info.type_id.0, name.clone()))
        .collect();
    let enum_id_to_name: HashMap<u32, String> = checker
        .enum_defs_iter()
        .map(|(name, info)| (info.type_id.0, name.clone()))
        .collect();

    // Build the nominal type table for backend consumers.
    let mut fn_sigs_table = HashMap::new();
    // Include checker fn_sigs FIRST — they have accurate return types for user functions.
    for (name, sig) in checker.fn_sigs_iter() {
        fn_sigs_table.insert(
            name.clone(),
            MirFnSig {
                name: name.clone(),
                params: sig
                    .params
                    .iter()
                    .map(|t| type_to_sig_name(t, &struct_id_to_name, &enum_id_to_name))
                    .collect(),
                ret: type_to_sig_name(&sig.ret, &struct_id_to_name, &enum_id_to_name),
            },
        );
    }
    // Fill in remaining from MIR functions (synthetic functions not in checker).
    for func in &mir.functions {
        fn_sigs_table.entry(func.name.clone()).or_insert_with(|| {
            let params: Vec<String> = func
                .params
                .iter()
                .map(|p| {
                    // Use struct/enum typed_locals for accurate param types
                    if let Some(sname) = func.struct_typed_locals.get(&p.id.0) {
                        sname.clone()
                    } else if let Some(ename) = func.enum_typed_locals.get(&p.id.0) {
                        ename.clone()
                    } else {
                        format!("{}", p.ty)
                    }
                })
                .collect();
            let ret = if let Some(sname) = func.struct_typed_locals.get(&u32::MAX) {
                sname.clone()
            } else {
                format!("{}", func.return_ty)
            };
            MirFnSig {
                name: func.name.clone(),
                params,
                ret,
            }
        });
    }
    mir.type_table = TypeTable {
        struct_defs: struct_defs.clone(),
        enum_defs: enum_defs.clone(),
        fn_sigs: fn_sigs_table,
    };

    mir.struct_defs = struct_defs;
    mir.enum_defs = enum_defs;
    finalize_lowered_module(&mut mir);
    mir
}

impl LowerCtx {
    /// Collect free variables in an expression that are not in the given bound set.
    pub(super) fn collect_free_vars(
        &self,
        expr: &ast::Expr,
        bound: &HashSet<&str>,
        out: &mut Vec<String>,
    ) {
        match expr {
            ast::Expr::Ident { name, .. } => {
                if !bound.contains(name.as_str()) {
                    // Check if it's a local in the enclosing scope (not a function or builtin)
                    if self.lookup_local(name).is_some() && !out.contains(name) {
                        out.push(name.clone());
                    }
                }
            }
            ast::Expr::Binary { left, right, .. } => {
                self.collect_free_vars(left, bound, out);
                self.collect_free_vars(right, bound, out);
            }
            ast::Expr::Unary { operand, .. } => {
                self.collect_free_vars(operand, bound, out);
            }
            ast::Expr::Call { callee, args, .. } => {
                self.collect_free_vars(callee, bound, out);
                for a in args {
                    self.collect_free_vars(a, bound, out);
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.collect_free_vars(cond, bound, out);
                for s in &then_block.stmts {
                    self.collect_free_vars_stmt(s, bound, out);
                }
                if let Some(t) = &then_block.tail_expr {
                    self.collect_free_vars(t, bound, out);
                }
                if let Some(b) = else_block {
                    for s in &b.stmts {
                        self.collect_free_vars_stmt(s, bound, out);
                    }
                    if let Some(t) = &b.tail_expr {
                        self.collect_free_vars(t, bound, out);
                    }
                }
            }
            ast::Expr::Block(block) => {
                for s in &block.stmts {
                    self.collect_free_vars_stmt(s, bound, out);
                }
                if let Some(t) = &block.tail_expr {
                    self.collect_free_vars(t, bound, out);
                }
            }
            ast::Expr::FieldAccess { object, .. } => {
                self.collect_free_vars(object, bound, out);
            }
            ast::Expr::Index { object, index, .. } => {
                self.collect_free_vars(object, bound, out);
                self.collect_free_vars(index, bound, out);
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                self.collect_free_vars(scrutinee, bound, out);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.collect_free_vars(guard, bound, out);
                    }
                    self.collect_free_vars(&arm.body, bound, out);
                }
            }
            ast::Expr::Try { expr, .. } => {
                self.collect_free_vars(expr, bound, out);
            }
            ast::Expr::StructInit { fields, .. } => {
                for (_, fexpr) in fields {
                    self.collect_free_vars(fexpr, bound, out);
                }
            }
            _ => {}
        }
    }

    fn collect_free_vars_stmt(
        &self,
        stmt: &ast::Stmt,
        bound: &HashSet<&str>,
        out: &mut Vec<String>,
    ) {
        match stmt {
            ast::Stmt::Let { init, .. } => {
                self.collect_free_vars(init, bound, out);
            }
            ast::Stmt::Expr(e) => {
                self.collect_free_vars(e, bound, out);
            }
            _ => {}
        }
    }
}
