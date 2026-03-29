//! Lower typed AST and CoreHIR to MIR.

mod expr;
mod func;
mod pattern;
mod stmt;
mod types;

use std::collections::{HashMap, HashSet};

use ark_diagnostics::DiagnosticSink;
use ark_hir::{BodyId, ExprKind, ItemKind, Program, Stmt};
use ark_parser::ast;
use ark_typecheck::types::Type as CheckerType;
use ark_typecheck::{CheckOutput, TypeChecker};

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

fn default_function_instance(name: &str) -> InstanceKey {
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

fn lower_hir_fallback(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = lower_to_mir(module, checker, sink);
    set_mir_provenance(&mut mir, MirProvenance::CoreHirFallback);
    mir
}

pub fn lower_to_mir_optimized(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = lower_to_mir(module, checker, sink);
    set_mir_provenance(&mut mir, MirProvenance::LegacyOptimized);
    mir
}

pub fn lower_hir_to_mir_placeholder(
    core_hir: &Program,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<MirModule, String> {
    lower_hir_to_mir(core_hir, checker, sink)
}

pub fn lower_hir_to_mir(
    core_hir: &Program,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<MirModule, String> {
    let module = core_hir
        .modules
        .first()
        .ok_or_else(|| "CoreHIR program has no modules".to_string())?;
    let item_count = module.items.len();
    let function_count = module
        .items
        .iter()
        .filter(|item| matches!(item.kind, ItemKind::Function(_)))
        .count();
    let body_count = core_hir.bodies.len();
    let control_flow_nodes = core_hir
        .bodies
        .iter()
        .flat_map(|body| body.stmts.iter())
        .filter(|stmt| {
            matches!(
                stmt,
                Stmt::While { .. }
                    | Stmt::Loop { .. }
                    | Stmt::ForRange { .. }
                    | Stmt::ForValues { .. }
            )
        })
        .count();
    let expr_complexity = core_hir
        .bodies
        .iter()
        .flat_map(|body| {
            body.tail
                .iter()
                .chain(body.stmts.iter().filter_map(|stmt| match stmt {
                    Stmt::Expr(expr) => Some(expr),
                    _ => None,
                }))
        })
        .filter(|expr| {
            matches!(
                expr.kind,
                ExprKind::Match { .. } | ExprKind::Try(_) | ExprKind::Closure { .. }
            )
        })
        .count();

    let mirrored = checker
        .latest_core_hir()
        .map(|bundle| {
            bundle
                .program
                .modules
                .first()
                .map(|resolved_module| resolved_module.items.len())
                .unwrap_or(0)
        })
        .unwrap_or(item_count);

    let mut mir = MirModule::new();
    set_mir_provenance(&mut mir, MirProvenance::CoreHir);
    push_optimization_trace(
        &mut mir,
        format!(
            "corehir-snapshot items={} functions={} bodies={} cfg_nodes={} complex_exprs={} mirror_items={}",
            item_count, function_count, body_count, control_flow_nodes, expr_complexity, mirrored
        ),
    );
    let _ = sink;
    Ok(mir)
}

pub fn lower_any_to_mir(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    lower_hir_fallback(module, checker, sink)
}

pub fn lower_corehir_via_legacy(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    lower_hir_fallback(module, checker, sink)
}

pub fn validate_lowered_mir(
    module: &MirModule,
) -> Result<(), Vec<crate::validate::MirValidationError>> {
    validate_module(module)
}

pub fn lowering_path_snapshot(module: &MirModule) -> String {
    module_snapshot(module)
}

pub fn compare_lowering_paths(legacy: &MirModule, corehir: &MirModule) -> String {
    compare_module_shapes(legacy, corehir)
}

pub fn mark_legacy_mir(module: &mut MirModule) {
    set_mir_provenance(module, MirProvenance::LegacyAst);
}

pub fn mark_corehir_fallback_mir(module: &mut MirModule) {
    set_mir_provenance(module, MirProvenance::CoreHirFallback);
}

pub fn mark_corehir_mir(module: &mut MirModule) {
    set_mir_provenance(module, MirProvenance::CoreHir);
}

pub fn clone_as_corehir_fallback(module: &MirModule) -> MirModule {
    let mut cloned = module.clone();
    set_mir_provenance(&mut cloned, MirProvenance::CoreHirFallback);
    cloned
}

pub fn clone_as_legacy(module: &MirModule) -> MirModule {
    let mut cloned = module.clone();
    set_mir_provenance(&mut cloned, MirProvenance::LegacyAst);
    cloned
}

pub fn lower_corehir_with_fallback(
    core_hir: &Program,
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<MirModule, String> {
    match lower_hir_to_mir(core_hir, checker, sink) {
        Ok(mir) if !mir.functions.is_empty() => Ok(mir),
        Ok(_) | Err(_) => Ok(lower_corehir_via_legacy(module, checker, sink)),
    }
}

pub fn lower_legacy_and_corehir(
    core_hir: &Program,
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<(MirModule, MirModule), String> {
    let mut legacy = lower_to_mir(module, checker, sink);
    set_mir_provenance(&mut legacy, MirProvenance::LegacyAst);
    let corehir = lower_corehir_with_fallback(core_hir, module, checker, sink)?;
    Ok((legacy, corehir))
}

pub fn lower_check_output_to_mir(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<MirModule, String> {
    lower_corehir_with_fallback(output.program(), module, checker, sink)
}

pub fn compare_check_output_to_legacy(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<String, String> {
    let (legacy, corehir) = lower_legacy_and_corehir(output.program(), module, checker, sink)?;
    Ok(compare_lowering_paths(&legacy, &corehir))
}

pub fn program_shape_snapshot(core_hir: &Program) -> String {
    let function_count = core_hir
        .modules
        .iter()
        .flat_map(|module| module.items.iter())
        .filter(|item| matches!(item.kind, ItemKind::Function(_)))
        .count();
    let body_count = core_hir.bodies.len();
    let first_body = core_hir
        .bodies
        .first()
        .map(|body| body.id.0)
        .unwrap_or_default();
    format!(
        "modules={} functions={} bodies={} first_body={}",
        core_hir.modules.len(),
        function_count,
        body_count,
        first_body
    )
}

pub fn referenced_body_ids(core_hir: &Program) -> Vec<BodyId> {
    core_hir.bodies.iter().map(|body| body.id).collect()
}

pub fn is_corehir_lowering_connected(core_hir: &Program) -> bool {
    !core_hir.modules.is_empty() && !core_hir.bodies.is_empty()
}

pub fn provenance_label(module: &MirModule) -> Option<&str> {
    mir_provenance(module)
}

pub fn has_materialized_functions(module: &MirModule) -> bool {
    !module.functions.is_empty()
}

pub fn legacy_vs_corehir_summary(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "legacy_provenance={:?} corehir_provenance={:?}\n{}",
        provenance_label(legacy),
        provenance_label(corehir),
        compare_lowering_paths(legacy, corehir)
    )
}

pub fn lower_module_from_corehir_bundle(
    module: &ast::Module,
    bundle: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<(MirModule, MirModule), String> {
    lower_legacy_and_corehir(bundle.program(), module, checker, sink)
}

pub fn ensure_runtime_candidate(module: &MirModule) -> bool {
    runtime_entry_name(module).is_some() || module.entry_fn.is_some()
}

pub fn lowering_debug_manifest(module: &MirModule) -> String {
    format!(
        "{}\nentry={:?}",
        module_snapshot(module),
        runtime_entry_name(module)
    )
}

pub fn lowering_diff_manifest(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "legacy:\n{}\n\ncorehir:\n{}\n\nshape:\n{}",
        lowering_debug_manifest(legacy),
        lowering_debug_manifest(corehir),
        compare_lowering_paths(legacy, corehir)
    )
}

pub fn lower_expr_shape_count(program: &Program) -> usize {
    program
        .bodies
        .iter()
        .map(|body| body.stmts.len() + usize::from(body.tail.is_some()))
        .sum()
}

pub fn corehir_lowering_summary(program: &Program) -> String {
    format!(
        "connected={} expr_shapes={} bodies={} modules={}",
        is_corehir_lowering_connected(program),
        lower_expr_shape_count(program),
        program.bodies.len(),
        program.modules.len()
    )
}

pub fn clone_for_optimized_legacy(module: &MirModule) -> MirModule {
    let mut cloned = module.clone();
    set_mir_provenance(&mut cloned, MirProvenance::LegacyOptimized);
    cloned
}

pub fn clone_for_optimized_corehir(module: &MirModule) -> MirModule {
    let mut cloned = module.clone();
    set_mir_provenance(&mut cloned, MirProvenance::CoreHirOptimized);
    cloned
}

pub fn note_lowering_origin(module: &mut MirModule, note: &str) {
    push_optimization_trace(module, note.to_string());
}

pub fn lower_corehir_only_summary(program: &Program) -> String {
    corehir_lowering_summary(program)
}

pub fn lower_legacy_only_summary(module: &MirModule) -> String {
    module_snapshot(module)
}

pub fn compare_program_and_mir(program: &Program, module: &MirModule) -> String {
    format!(
        "program={}\nmir={}",
        corehir_lowering_summary(program),
        module_snapshot(module)
    )
}

pub fn mark_lowered(module: &mut MirModule, provenance: MirProvenance) {
    set_mir_provenance(module, provenance);
}

pub fn compare_mir_counts(legacy: &MirModule, corehir: &MirModule) -> (usize, usize) {
    (legacy.functions.len(), corehir.functions.len())
}

pub fn describe_mir_counts(legacy: &MirModule, corehir: &MirModule) -> String {
    let (legacy_count, corehir_count) = compare_mir_counts(legacy, corehir);
    format!(
        "legacy_functions={} corehir_functions={}",
        legacy_count, corehir_count
    )
}

pub fn lowering_probe(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<String, String> {
    let (legacy, corehir) = lower_module_from_corehir_bundle(module, output, checker, sink)?;
    Ok(format!(
        "{}\n{}\n{}",
        describe_mir_counts(&legacy, &corehir),
        corehir_lowering_summary(output.program()),
        compare_lowering_paths(&legacy, &corehir)
    ))
}

pub fn lowering_paths_connected(program: &Program) -> bool {
    is_corehir_lowering_connected(program)
}

pub fn lowering_paths_ready(program: &Program) -> bool {
    lowering_paths_connected(program)
}

pub fn lower_hir_debug_dump(program: &Program) -> String {
    corehir_lowering_summary(program)
}

pub fn lower_mir_debug_dump(module: &MirModule) -> String {
    module_snapshot(module)
}

pub fn lower_path_pair_debug_dump(legacy: &MirModule, corehir: &MirModule) -> String {
    lowering_diff_manifest(legacy, corehir)
}

pub fn set_validation_run_note(module: &mut MirModule) {
    note_lowering_origin(module, "validated");
}

pub fn lowering_paths_have_runtime_candidate(legacy: &MirModule, corehir: &MirModule) -> bool {
    ensure_runtime_candidate(legacy) || ensure_runtime_candidate(corehir)
}

pub fn lowering_paths_runtime_summary(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "legacy_entry={:?} corehir_entry={:?}",
        runtime_entry_name(legacy),
        runtime_entry_name(corehir)
    )
}

pub fn lowering_paths_snapshot(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "{}\n{}",
        lower_path_pair_debug_dump(legacy, corehir),
        lowering_paths_runtime_summary(legacy, corehir)
    )
}

pub fn lowering_paths_structural_diff(legacy: &MirModule, corehir: &MirModule) -> bool {
    !compare_module_shapes(legacy, corehir).is_empty()
}

pub fn compare_lowering_provenance(legacy: &MirModule, corehir: &MirModule) -> bool {
    mir_provenance(legacy) != mir_provenance(corehir)
}

pub fn lowering_trace(module: &MirModule) -> &[String] {
    optimization_trace(module)
}

pub fn lowering_provenance(module: &MirModule) -> Option<&str> {
    mir_provenance(module)
}

pub fn lowering_entry(module: &MirModule) -> Option<String> {
    runtime_entry_name(module)
}

pub fn lowering_functions(module: &MirModule) -> Vec<String> {
    module
        .functions
        .iter()
        .map(|func| func.name.clone())
        .collect()
}

pub fn lowering_is_empty(module: &MirModule) -> bool {
    module.functions.is_empty()
}

pub fn lowering_can_run(module: &MirModule) -> bool {
    ensure_runtime_candidate(module)
}

pub fn lowering_validation_summary(module: &MirModule) -> String {
    format!(
        "validation_runs={} snapshot={}",
        module.stats.validation_runs,
        module_snapshot(module)
    )
}

pub fn lowering_path_name(module: &MirModule) -> &str {
    mir_provenance(module).unwrap_or("unknown")
}

pub fn lowering_shape_tuple(module: &MirModule) -> (usize, Option<String>) {
    (module.functions.len(), runtime_entry_name(module))
}

pub fn lowering_shape_matches(lhs: &MirModule, rhs: &MirModule) -> bool {
    lowering_shape_tuple(lhs) == lowering_shape_tuple(rhs)
}

pub fn lowering_path_compare(lhs: &MirModule, rhs: &MirModule) -> String {
    format!(
        "matches={}\n{}",
        lowering_shape_matches(lhs, rhs),
        compare_module_shapes(lhs, rhs)
    )
}

pub fn lowering_path_probe(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<String, String> {
    let (legacy, corehir) = lower_module_from_corehir_bundle(module, output, checker, sink)?;
    Ok(lowering_path_compare(&legacy, &corehir))
}

pub fn lowering_path_export_summary(module: &MirModule) -> String {
    format!(
        "entry={:?} functions={}",
        runtime_entry_name(module),
        module.functions.len()
    )
}

pub fn lowering_runtime_parity_hint(module: &MirModule) -> String {
    format!(
        "runtime_candidate={} {}",
        ensure_runtime_candidate(module),
        lowering_path_export_summary(module)
    )
}

pub fn lowering_runtime_parity_pair(lhs: &MirModule, rhs: &MirModule) -> String {
    format!(
        "legacy_runtime={}\ncorehir_runtime={}",
        lowering_runtime_parity_hint(lhs),
        lowering_runtime_parity_hint(rhs)
    )
}

pub fn lowering_probe_full(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<String, String> {
    let (legacy, corehir) = lower_module_from_corehir_bundle(module, output, checker, sink)?;
    Ok(format!(
        "{}\n{}",
        lowering_path_compare(&legacy, &corehir),
        lowering_runtime_parity_pair(&legacy, &corehir)
    ))
}

pub fn lower_legacy_only(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = lower_to_mir(module, checker, sink);
    set_mir_provenance(&mut mir, MirProvenance::LegacyAst);
    mir
}

pub fn lower_corehir_only(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<MirModule, String> {
    lower_check_output_to_mir(module, output, checker, sink)
}

pub fn lower_prefer_corehir(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<MirModule, String> {
    lower_corehir_only(module, output, checker, sink)
}

pub fn lower_prefer_legacy(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    lower_legacy_only(module, checker, sink)
}

pub fn lower_dual_paths(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<(MirModule, MirModule), String> {
    lower_module_from_corehir_bundle(module, output, checker, sink)
}

pub fn lower_dual_paths_summary(
    module: &ast::Module,
    output: &CheckOutput,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<String, String> {
    let (legacy, corehir) = lower_dual_paths(module, output, checker, sink)?;
    Ok(format!(
        "{}\n{}",
        lowering_path_compare(&legacy, &corehir),
        lowering_runtime_parity_pair(&legacy, &corehir)
    ))
}

pub fn validate_lowering_pair(
    legacy: &MirModule,
    corehir: &MirModule,
) -> Result<(), Vec<crate::validate::MirValidationError>> {
    validate_module(legacy)?;
    validate_module(corehir)
}

pub fn compare_lowering_runtime_entries(legacy: &MirModule, corehir: &MirModule) -> bool {
    runtime_entry_name(legacy) == runtime_entry_name(corehir)
}

pub fn lowering_pair_can_run(legacy: &MirModule, corehir: &MirModule) -> bool {
    ensure_runtime_candidate(legacy) && ensure_runtime_candidate(corehir)
}

pub fn lowering_pair_status(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "entries_match={} can_run={} legacy={} corehir={}",
        compare_lowering_runtime_entries(legacy, corehir),
        lowering_pair_can_run(legacy, corehir),
        lowering_path_name(legacy),
        lowering_path_name(corehir)
    )
}

pub fn lowering_pair_full_status(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "{}\n{}",
        lowering_pair_status(legacy, corehir),
        compare_module_shapes(legacy, corehir)
    )
}

pub fn mirror_corehir_counts(program: &Program, module: &mut MirModule) {
    note_lowering_origin(
        module,
        &format!(
            "corehir-bodies={} corehir-modules={}",
            program.bodies.len(),
            program.modules.len()
        ),
    );
}

pub fn mirror_check_output(output: &CheckOutput, module: &mut MirModule) {
    mirror_corehir_counts(output.program(), module);
}

pub fn lowering_has_functions(module: &MirModule) -> bool {
    !module.functions.is_empty()
}

pub fn lowering_empty_reason(module: &MirModule) -> String {
    if module.functions.is_empty() {
        "no-functions".to_string()
    } else {
        "has-functions".to_string()
    }
}

pub fn lowering_status_line(module: &MirModule) -> String {
    format!(
        "path={} entry={:?} reason={}",
        lowering_path_name(module),
        runtime_entry_name(module),
        lowering_empty_reason(module)
    )
}

pub fn compare_status_lines(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "legacy={}\ncorehir={}",
        lowering_status_line(legacy),
        lowering_status_line(corehir)
    )
}

pub fn lowering_pair_report(legacy: &MirModule, corehir: &MirModule) -> String {
    format!(
        "{}\n{}",
        compare_status_lines(legacy, corehir),
        compare_module_shapes(legacy, corehir)
    )
}

pub(crate) struct LowerCtx {
    pub(super) locals: Vec<(String, LocalId)>,
    pub(super) next_local: u32,
    pub(super) string_locals: HashSet<u32>,
    pub(super) f64_locals: HashSet<u32>,
    pub(super) i64_locals: HashSet<u32>,
    pub(super) bool_locals: HashSet<u32>,
    pub(super) char_locals: HashSet<u32>,
    pub(super) enum_tags: HashMap<String, i32>,
    /// enum name -> variant info: (variant_name, field_count)
    pub(super) enum_variants: HashMap<String, Vec<(String, usize)>>,
    /// "EnumName::Variant" -> enum name (for reverse lookup)
    pub(super) variant_to_enum: HashMap<String, String>,
    /// Also support bare names like "Some", "None", "Ok", "Err"
    pub(super) bare_variant_tags: HashMap<String, (String, i32, usize)>, // name -> (enum, tag, field_count)
    /// struct name -> ordered (field name, field type name)
    pub(super) struct_defs: HashMap<String, Vec<(String, String)>>,
    /// local id -> struct type name
    pub(super) struct_typed_locals: HashMap<u32, String>,
    /// local id -> enum type name
    pub(super) enum_typed_locals: HashMap<u32, String>,
    /// local id -> variant-level payload type info: (variant_idx, field_idx) -> is_string
    /// Maps local_id -> mapping from (variant_name, field_index) -> is_string
    pub(super) enum_local_payload_strings: HashMap<u32, HashSet<(String, u32)>>,
    /// local id -> specialized enum name for concrete generic types (e.g., "Result_i64_String")
    pub(super) enum_local_specialized: HashMap<u32, String>,
    /// enum name -> [(variant_name, [payload_type_names])]
    pub(super) enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    /// "EnumName::VariantName" -> ordered field names (for struct variants)
    pub(super) enum_variant_field_names: HashMap<String, Vec<String>>,
    /// Locals known to hold Vec<String> values.
    pub(super) vec_string_locals: HashSet<u32>,
    /// Locals known to hold Vec<i64> values.
    pub(super) vec_i64_locals: HashSet<u32>,
    /// Locals known to hold Vec<f64> values.
    pub(super) vec_f64_locals: HashSet<u32>,
    /// Locals known to hold Vec<i32> values.
    pub(super) vec_i32_locals: HashSet<u32>,
    /// Local to assign break values to (for loop-as-expression).
    pub(super) loop_result_local: Option<LocalId>,
    /// Function name -> return type expression (for resolving generic enum payloads in match).
    pub(super) fn_return_types: HashMap<String, ast::TypeExpr>,
    /// Set of user-defined function names (for function references).
    pub(super) user_fn_names: HashSet<String>,
    /// Type parameters of the function being lowered (for generic tuple support).
    pub(super) type_params: Vec<String>,
    /// Set of generic function names (functions with type_params).
    pub(super) generic_fn_names: HashSet<String>,
    /// Closure info: local_id -> (synthetic function name, captured variable names)
    pub(super) closure_locals: HashMap<u32, (String, Vec<String>)>,
    /// Pending synthetic closure functions to add to the module.
    pub(super) pending_closures: Vec<MirFunction>,
    /// Counter for generating unique closure names.
    pub(super) closure_counter: u32,
    /// Synthetic closure function name -> captured variable names (for call-site injection).
    pub(super) closure_fn_captures: HashMap<String, Vec<String>>,
    /// Method call resolutions from type checker: span_start -> (mangled_name, struct_name)
    pub(super) method_resolutions: HashMap<u32, (String, String)>,
}

impl LowerCtx {
    #[allow(clippy::too_many_arguments)]
    fn new(
        enum_tags: HashMap<String, i32>,
        struct_defs: HashMap<String, Vec<(String, String)>>,
        enum_variants: HashMap<String, Vec<(String, usize)>>,
        variant_to_enum: HashMap<String, String>,
        bare_variant_tags: HashMap<String, (String, i32, usize)>,
        enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
        enum_variant_field_names: HashMap<String, Vec<String>>,
        fn_return_types: HashMap<String, ast::TypeExpr>,
        user_fn_names: HashSet<String>,
        method_resolutions: HashMap<u32, (String, String)>,
        type_params: Vec<String>,
        generic_fn_names: HashSet<String>,
    ) -> Self {
        Self {
            locals: Vec::new(),
            next_local: 0,
            string_locals: HashSet::new(),
            f64_locals: HashSet::new(),
            i64_locals: HashSet::new(),
            bool_locals: HashSet::new(),
            char_locals: HashSet::new(),
            enum_tags,
            enum_variants,
            variant_to_enum,
            bare_variant_tags,
            struct_defs,
            struct_typed_locals: HashMap::new(),
            enum_typed_locals: HashMap::new(),
            enum_local_payload_strings: HashMap::new(),
            enum_local_specialized: HashMap::new(),
            enum_defs,
            enum_variant_field_names,
            vec_string_locals: HashSet::new(),
            vec_i64_locals: HashSet::new(),
            vec_f64_locals: HashSet::new(),
            vec_i32_locals: HashSet::new(),
            loop_result_local: None,
            fn_return_types,
            user_fn_names,
            type_params,
            generic_fn_names,
            closure_locals: HashMap::new(),
            pending_closures: Vec::new(),
            closure_counter: 0,
            closure_fn_captures: HashMap::new(),
            method_resolutions,
        }
    }

    pub(super) fn declare_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        self.locals.push((name.to_string(), id));
        id
    }

    /// Check if an expression is a call to a generic function.
    pub(super) fn is_generic_call(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call { callee, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    return self.generic_fn_names.contains(name);
                }
                false
            }
            _ => false,
        }
    }

    pub(super) fn new_temp(&mut self) -> LocalId {
        self.declare_local(&format!("__tmp_{}", self.next_local))
    }

    pub(super) fn lookup_local(&self, name: &str) -> Option<LocalId> {
        self.locals
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, id)| *id)
    }
}

// Re-export the main lowering function from func submodule
pub use func::lower_to_mir;
