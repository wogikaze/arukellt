//! Finalize helpers and MIR construction utilities for the lowering pipeline.

use std::collections::HashMap;

use ark_typecheck::types::Type as CheckerType;

use crate::mir::*;
use crate::validate::validate_module;

/// Format a checker Type to a string using struct/enum name maps rather than
/// the default Display which produces "struct#N"/"enum#N".
pub(crate) fn type_to_sig_name(
    ty: &CheckerType,
    struct_id_to_name: &HashMap<u32, String>,
    enum_id_to_name: &HashMap<u32, String>,
) -> String {
    use ark_typecheck::types::Type;
    match ty {
        Type::Struct(id) => struct_id_to_name
            .get(&id.0)
            .cloned()
            .unwrap_or_else(|| format!("{}", ty)),
        Type::Enum(id) => enum_id_to_name
            .get(&id.0)
            .cloned()
            .unwrap_or_else(|| format!("{}", ty)),
        Type::Vec(elem) => format!(
            "Vec<{}>",
            type_to_sig_name(elem, struct_id_to_name, enum_id_to_name)
        ),
        Type::Option(inner) => format!(
            "Option<{}>",
            type_to_sig_name(inner, struct_id_to_name, enum_id_to_name)
        ),
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            type_to_sig_name(ok, struct_id_to_name, enum_id_to_name),
            type_to_sig_name(err, struct_id_to_name, enum_id_to_name),
        ),
        Type::Tuple(elems) => format!("__tuple{}", elems.len()),
        _ => format!("{}", ty),
    }
}

fn finalize_module_metadata(mir: &mut MirModule) {
    sync_module_metadata(mir);
    let _ = validate_module(mir);
}

#[allow(dead_code)]
fn clone_with_provenance(module: &MirModule, provenance: MirProvenance) -> MirModule {
    let mut cloned = module.clone();
    set_mir_provenance(&mut cloned, provenance);
    cloned
}

#[allow(dead_code)]
fn function_names(module: &MirModule) -> Vec<String> {
    module
        .functions
        .iter()
        .map(|func| func.name.clone())
        .collect()
}

#[allow(dead_code)]
fn validation_runs(module: &MirModule) -> u32 {
    module.stats.validation_runs
}

pub(crate) fn default_function_instance(name: &str) -> InstanceKey {
    InstanceKey::simple(name.to_string())
}

fn default_function_source() -> SourceInfo {
    SourceInfo::unknown()
}

fn default_block_source() -> SourceInfo {
    SourceInfo::unknown()
}

pub(crate) fn finalize_function(function: &mut MirFunction) {
    if function.instance.item.is_empty() {
        function.instance = default_function_instance(&function.name);
    }
}

pub(crate) fn finalize_block(block: &mut BasicBlock) {
    if block.source.span.is_none() {
        block.source = default_block_source();
    }
}

pub(crate) fn finalize_function_blocks(function: &mut MirFunction) {
    for block in &mut function.blocks {
        finalize_block(block);
    }
}

pub(crate) fn finalize_function_metadata(function: &mut MirFunction) {
    finalize_function(function);
    finalize_function_blocks(function);
    if function.source.span.is_none() {
        function.source = default_function_source();
    }
}

pub(crate) fn push_function(mir: &mut MirModule, mut function: MirFunction) {
    finalize_function_metadata(&mut function);
    register_function_metadata(mir, &function);
    mir.functions.push(function);
}

pub(crate) fn infer_fn_id(name: &str, next_fn_id: u32) -> FnId {
    let inferred = legacy_fn_id(name);
    if inferred.0 == 0 {
        FnId(next_fn_id)
    } else {
        inferred
    }
}

pub(crate) fn fallback_block(
    id: BlockId,
    stmts: Vec<MirStmt>,
    terminator: Terminator,
) -> BasicBlock {
    BasicBlock {
        id,
        stmts,
        terminator,
        source: default_block_source(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn fallback_function(
    id: FnId,
    name: String,
    params: Vec<MirLocal>,
    return_ty: ark_typecheck::types::Type,
    locals: Vec<MirLocal>,
    blocks: Vec<BasicBlock>,
    entry: BlockId,
    struct_typed_locals: std::collections::HashMap<u32, String>,
    enum_typed_locals: std::collections::HashMap<u32, String>,
    type_params: Vec<String>,
    is_exported: bool,
) -> MirFunction {
    MirFunction {
        id,
        name: name.clone(),
        instance: default_function_instance(&name),
        params,
        return_ty,
        locals,
        blocks,
        entry,
        struct_typed_locals,
        enum_typed_locals,
        type_params,
        source: default_function_source(),
        is_exported,
    }
}

pub(crate) fn finalize_lowered_module(mir: &mut MirModule) {
    finalize_module_metadata(mir);
}
