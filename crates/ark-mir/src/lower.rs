//! Lower typed AST and CoreHIR to MIR.

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
fn type_to_sig_name(
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
        Type::Vec(elem) => format!("Vec<{}>", type_to_sig_name(elem, struct_id_to_name, enum_id_to_name)),
        Type::Option(inner) => format!("Option<{}>", type_to_sig_name(inner, struct_id_to_name, enum_id_to_name)),
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

fn finalize_function(function: &mut MirFunction) {
    if function.instance.item.is_empty() {
        function.instance = default_function_instance(&function.name);
    }
}

fn finalize_block(block: &mut BasicBlock) {
    if block.source.span.is_none() {
        block.source = default_block_source();
    }
}

fn finalize_function_blocks(function: &mut MirFunction) {
    for block in &mut function.blocks {
        finalize_block(block);
    }
}

fn finalize_function_metadata(function: &mut MirFunction) {
    finalize_function(function);
    finalize_function_blocks(function);
    if function.source.span.is_none() {
        function.source = default_function_source();
    }
}

fn push_function(mir: &mut MirModule, mut function: MirFunction) {
    finalize_function_metadata(&mut function);
    register_function_metadata(mir, &function);
    mir.functions.push(function);
}

fn infer_fn_id(name: &str, next_fn_id: u32) -> FnId {
    let inferred = legacy_fn_id(name);
    if inferred.0 == 0 {
        FnId(next_fn_id)
    } else {
        inferred
    }
}

fn fallback_block(id: BlockId, stmts: Vec<MirStmt>, terminator: Terminator) -> BasicBlock {
    BasicBlock {
        id,
        stmts,
        terminator,
        source: default_block_source(),
    }
}

#[allow(clippy::too_many_arguments)]
fn fallback_function(
    id: FnId,
    name: String,
    params: Vec<MirLocal>,
    return_ty: ark_typecheck::types::Type,
    locals: Vec<MirLocal>,
    blocks: Vec<BasicBlock>,
    entry: BlockId,
    struct_typed_locals: std::collections::HashMap<u32, String>,
    enum_typed_locals: std::collections::HashMap<u32, String>,
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
        source: default_function_source(),
    }
}

fn finalize_lowered_module(mir: &mut MirModule) {
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
                if let ast::TypeExpr::Generic { name: tname, .. } = &param.ty {
                    if ctx.enum_variants.contains_key(tname.as_str()) {
                        ctx.enum_typed_locals.insert(pid.0, tname.clone());
                    }
                }
            }

            let entry = BlockId(0);
            let mut stmts = ctx.lower_block(&f.body);

            // Handle tail expression: if it's a void call (println etc.),
            // lower it as a statement. Otherwise, it's the return value.
            let tail_op = if let Some(tail) = &f.body.tail_expr {
                if is_void_expr(tail) {
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
                    if let ast::TypeExpr::Generic { name: tname, .. } = &param.ty {
                        if ctx.enum_variants.contains_key(tname.as_str()) {
                            ctx.enum_typed_locals.insert(pid.0, tname.clone());
                        }
                    }
                }

                let entry = BlockId(0);
                let mut stmts = ctx.lower_block(&method.body);

                let tail_op = if let Some(tail) = &method.body.tail_expr {
                    if is_void_expr(tail) {
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
                params: sig.params.iter().map(|t| type_to_sig_name(t, &struct_id_to_name, &enum_id_to_name)).collect(),
                ret: type_to_sig_name(&sig.ret, &struct_id_to_name, &enum_id_to_name),
            },
        );
    }
    // Fill in remaining from MIR functions (synthetic functions not in checker).
    for func in &mir.functions {
        fn_sigs_table
            .entry(func.name.clone())
            .or_insert_with(|| {
                let params: Vec<String> = func.params.iter().map(|p| {
                    // Use struct/enum typed_locals for accurate param types
                    if let Some(sname) = func.struct_typed_locals.get(&p.id.0) {
                        sname.clone()
                    } else if let Some(ename) = func.enum_typed_locals.get(&p.id.0) {
                        ename.clone()
                    } else {
                        format!("{}", p.ty)
                    }
                }).collect();
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

struct LowerCtx {
    locals: Vec<(String, LocalId)>,
    next_local: u32,
    string_locals: HashSet<u32>,
    f64_locals: HashSet<u32>,
    i64_locals: HashSet<u32>,
    bool_locals: HashSet<u32>,
    enum_tags: HashMap<String, i32>,
    /// enum name -> variant info: (variant_name, field_count)
    enum_variants: HashMap<String, Vec<(String, usize)>>,
    /// "EnumName::Variant" -> enum name (for reverse lookup)
    variant_to_enum: HashMap<String, String>,
    /// Also support bare names like "Some", "None", "Ok", "Err"
    bare_variant_tags: HashMap<String, (String, i32, usize)>, // name -> (enum, tag, field_count)
    /// struct name -> ordered (field name, field type name)
    struct_defs: HashMap<String, Vec<(String, String)>>,
    /// local id -> struct type name
    struct_typed_locals: HashMap<u32, String>,
    /// local id -> enum type name
    enum_typed_locals: HashMap<u32, String>,
    /// local id -> variant-level payload type info: (variant_idx, field_idx) -> is_string
    /// Maps local_id -> mapping from (variant_name, field_index) -> is_string
    enum_local_payload_strings: HashMap<u32, HashSet<(String, u32)>>,
    /// local id -> specialized enum name for concrete generic types (e.g., "Result_i64_String")
    enum_local_specialized: HashMap<u32, String>,
    /// enum name -> [(variant_name, [payload_type_names])]
    enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    /// "EnumName::VariantName" -> ordered field names (for struct variants)
    enum_variant_field_names: HashMap<String, Vec<String>>,
    /// Locals known to hold Vec<String> values.
    vec_string_locals: HashSet<u32>,
    /// Locals known to hold Vec<i64> values.
    vec_i64_locals: HashSet<u32>,
    /// Locals known to hold Vec<f64> values.
    vec_f64_locals: HashSet<u32>,
    /// Locals known to hold Vec<i32> values.
    vec_i32_locals: HashSet<u32>,
    /// Local to assign break values to (for loop-as-expression).
    loop_result_local: Option<LocalId>,
    /// Function name -> return type expression (for resolving generic enum payloads in match).
    fn_return_types: HashMap<String, ast::TypeExpr>,
    /// Set of user-defined function names (for function references).
    user_fn_names: HashSet<String>,
    /// Closure info: local_id -> (synthetic function name, captured variable names)
    closure_locals: HashMap<u32, (String, Vec<String>)>,
    /// Pending synthetic closure functions to add to the module.
    pending_closures: Vec<MirFunction>,
    /// Counter for generating unique closure names.
    closure_counter: u32,
    /// Synthetic closure function name -> captured variable names (for call-site injection).
    closure_fn_captures: HashMap<String, Vec<String>>,
    /// Method call resolutions from type checker: span_start -> (mangled_name, struct_name)
    method_resolutions: HashMap<u32, (String, String)>,
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
    ) -> Self {
        Self {
            locals: Vec::new(),
            next_local: 0,
            string_locals: HashSet::new(),
            f64_locals: HashSet::new(),
            i64_locals: HashSet::new(),
            bool_locals: HashSet::new(),
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
            closure_locals: HashMap::new(),
            pending_closures: Vec::new(),
            closure_counter: 0,
            closure_fn_captures: HashMap::new(),
            method_resolutions,
        }
    }

    fn declare_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        self.locals.push((name.to_string(), id));
        id
    }

    fn new_temp(&mut self) -> LocalId {
        self.declare_local(&format!("__tmp_{}", self.next_local))
    }

    fn lookup_local(&self, name: &str) -> Option<LocalId> {
        self.locals
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, id)| *id)
    }

    /// Infer the struct type name of an expression (for field access).
    fn infer_struct_type(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Ident { name, .. } => {
                let local_id = self.lookup_local(name)?;
                self.struct_typed_locals.get(&local_id.0).cloned()
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                // Chained field access: get parent struct, look up field type
                let parent_struct = self.infer_struct_type(object)?;
                let fields = self.struct_defs.get(&parent_struct)?;
                let field_type = fields
                    .iter()
                    .find(|(fname, _)| fname == field)
                    .map(|(_, ftype)| ftype.clone())?;
                // The field type is the struct name for the nested struct
                if self.struct_defs.contains_key(&field_type) {
                    Some(field_type)
                } else {
                    None
                }
            }
            ast::Expr::Call { callee, .. } => {
                // For method calls returning struct, check return type
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    let struct_name = self.infer_struct_type(object)?;
                    let mangled = format!("{}__{}", struct_name, field);
                    if let Some(ast::TypeExpr::Named { name, .. }) =
                        self.fn_return_types.get(&mangled)
                    {
                        if self.struct_defs.contains_key(name.as_str()) {
                            return Some(name.clone());
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Infer struct name from an init expression (e.g., StructInit or function call returning struct)
    fn infer_struct_from_init(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::StructInit { name, .. } => {
                if self.struct_defs.contains_key(name.as_str()) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            ast::Expr::Call { callee, .. } => {
                // Check if function returns a struct type
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    if let Some(ast::TypeExpr::Named { name: tname, .. }) =
                        self.fn_return_types.get(name)
                    {
                        if self.struct_defs.contains_key(tname.as_str()) {
                            return Some(tname.clone());
                        }
                    }
                }
                // Check method call returning struct
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    if let Some(struct_name) = self.infer_struct_type(object) {
                        let mangled = format!("{}__{}", struct_name, field);
                        if let Some(ast::TypeExpr::Named { name: tname, .. }) =
                            self.fn_return_types.get(&mangled)
                        {
                            if self.struct_defs.contains_key(tname.as_str()) {
                                return Some(tname.clone());
                            }
                        }
                    }
                }
                None
            }
            ast::Expr::Binary { span, .. } => {
                // Check if operator overloading returns a struct
                if let Some((mangled, _)) = self.method_resolutions.get(&span.start) {
                    if let Some(ast::TypeExpr::Named { name: tname, .. }) =
                        self.fn_return_types.get(mangled)
                    {
                        if self.struct_defs.contains_key(tname.as_str()) {
                            return Some(tname.clone());
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Infer the return TypeExpr for a call expression by looking up fn_return_types.
    fn infer_return_type_expr(&self, expr: &ast::Expr) -> Option<ast::TypeExpr> {
        match expr {
            ast::Expr::Call { callee, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    return self.fn_return_types.get(name).cloned();
                }
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    if let Some(struct_name) = self.infer_struct_type(object) {
                        let mangled = format!("{}__{}", struct_name, field);
                        return self.fn_return_types.get(&mangled).cloned();
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if an identifier is a known enum variant constructor.
    #[allow(dead_code)]
    fn is_enum_variant_call(&self, name: &str) -> bool {
        self.bare_variant_tags.contains_key(name)
    }

    /// Collect free variables in an expression that are not in the given bound set.
    fn collect_free_vars(&self, expr: &ast::Expr, bound: &HashSet<&str>, out: &mut Vec<String>) {
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

    /// Detect which elements of a tuple-returning expression are strings.
    /// For a call like `pair(42, String_from("hello"))`, returns {1} since arg[1] is String.
    fn detect_string_tuple_elements(
        &self,
        init_expr: &ast::Expr,
        op: &Operand,
        _arity: usize,
    ) -> HashSet<usize> {
        let mut result = HashSet::new();
        // If the init expression is a direct tuple, check each element
        if let ast::Expr::Tuple { elements, .. } = init_expr {
            for (i, elem) in elements.iter().enumerate() {
                if self.is_string_ast_expr(elem) {
                    result.insert(i);
                }
            }
            return result;
        }
        // If the init expression is a call to a generic function, check the arguments
        if let ast::Expr::Call { callee, args, .. } = init_expr {
            #[allow(clippy::collapsible_match)]
            if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                // Look up the function's return type
                if let Some(ret_ty) = self.fn_return_types.get(name) {
                    // If the return type is a tuple, map args to tuple elements
                    if let ast::TypeExpr::Tuple(tuple_types, _) = ret_ty {
                        // Check if the function maps args directly to tuple elements
                        // (common for pair-like functions)
                        if tuple_types.len() == args.len() {
                            for (i, arg) in args.iter().enumerate() {
                                if self.is_string_ast_expr(arg) {
                                    result.insert(i);
                                }
                            }
                        }
                    }
                }
            }
        }
        // If the operand itself is a StructInit (lowered tuple), check fields
        if let Operand::StructInit { fields, .. } = op {
            for (i, (_, field_op)) in fields.iter().enumerate() {
                if self.is_string_operand_mir(field_op) {
                    result.insert(i);
                }
            }
        }
        result
    }

    /// Check if an AST expression produces a String value.
    fn is_string_ast_expr(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::StringLit { .. } => true,
            ast::Expr::Call { callee, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    matches!(
                        name.as_str(),
                        "String_from"
                            | "String_new"
                            | "concat"
                            | "slice"
                            | "join"
                            | "i32_to_string"
                            | "i64_to_string"
                            | "f64_to_string"
                            | "f32_to_string"
                            | "bool_to_string"
                            | "char_to_string"
                            | "to_lower"
                            | "to_upper"
                            | "clone"
                            | "to_string"
                    )
                } else {
                    false
                }
            }
            ast::Expr::Ident { name, .. } => {
                if let Some(lid) = self.lookup_local(name) {
                    self.string_locals.contains(&lid.0)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a MIR operand represents a String value.
    fn is_string_operand_mir(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstString(_) => true,
            Operand::Call(name, _) => {
                matches!(
                    name.as_str(),
                    "String_from"
                        | "String_new"
                        | "concat"
                        | "slice"
                        | "join"
                        | "i32_to_string"
                        | "i64_to_string"
                        | "f64_to_string"
                        | "f32_to_string"
                        | "bool_to_string"
                        | "char_to_string"
                        | "to_lower"
                        | "to_upper"
                        | "clone"
                        | "to_string"
                ) || name.ends_with("__to_string")
            }
            Operand::Place(Place::Local(lid)) => self.string_locals.contains(&lid.0),
            Operand::IfExpr { then_result, else_result, .. } => {
                then_result.as_ref().is_some_and(|r| self.is_string_operand_mir(r))
                    || else_result.as_ref().is_some_and(|r| self.is_string_operand_mir(r))
            }
            _ => false,
        }
    }

    /// Check if a MIR operand produces an f64 value.
    fn is_f64_operand_mir(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstF64(_) => true,
            Operand::Call(name, _) => matches!(name.as_str(), "sqrt"),
            Operand::BinOp(_, l, r) => self.is_f64_operand_mir(l) || self.is_f64_operand_mir(r),
            Operand::Place(Place::Local(lid)) => self.f64_locals.contains(&lid.0),
            Operand::IfExpr { then_result, else_result, .. } => {
                then_result.as_ref().is_some_and(|r| self.is_f64_operand_mir(r))
                    || else_result.as_ref().is_some_and(|r| self.is_f64_operand_mir(r))
            }
            _ => false,
        }
    }

    fn is_i64_operand_mir(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstI64(_) => true,
            Operand::Call(name, _) => matches!(name.as_str(), "clock_now"),
            Operand::BinOp(_, l, r) => self.is_i64_operand_mir(l) || self.is_i64_operand_mir(r),
            Operand::Place(Place::Local(lid)) => self.i64_locals.contains(&lid.0),
            Operand::IfExpr { then_result, else_result, .. } => {
                then_result.as_ref().is_some_and(|r| self.is_i64_operand_mir(r))
                    || else_result.as_ref().is_some_and(|r| self.is_i64_operand_mir(r))
            }
            _ => false,
        }
    }

    fn lower_block(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            self.lower_stmt(stmt, &mut stmts);
        }
        stmts
    }

    /// Lower a block including its tail expression as a statement.
    fn lower_block_all(&mut self, block: &ast::Block) -> Vec<MirStmt> {
        let mut stmts = self.lower_block(block);
        if let Some(tail) = &block.tail_expr {
            self.lower_expr_stmt(tail, &mut stmts);
        }
        stmts
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt, out: &mut Vec<MirStmt>) {
        match stmt {
            ast::Stmt::Let {
                name,
                init,
                ty,
                pattern,
                ..
            } => {
                // Handle tuple destructuring: let (a, b) = expr
                if let Some(ast::Pattern::Tuple { elements, .. }) = pattern {
                    let tuple_name = format!("__tuple{}", elements.len());
                    let local_id = self.declare_local(name);
                    let op = self.lower_expr(init);
                    out.push(MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(op.clone()),
                    ));
                    // Detect which tuple elements are strings from the init expression
                    let string_element_indices =
                        self.detect_string_tuple_elements(init, &op, elements.len());
                    // Destructure each element
                    for (i, elem) in elements.iter().enumerate() {
                        if let ast::Pattern::Ident {
                            name: elem_name, ..
                        } = elem
                        {
                            let elem_id = self.declare_local(elem_name);
                            if string_element_indices.contains(&i) {
                                self.string_locals.insert(elem_id.0);
                            }
                            let access = Operand::FieldAccess {
                                object: Box::new(Operand::Place(Place::Local(local_id))),
                                struct_name: tuple_name.clone(),
                                field: i.to_string(),
                            };
                            out.push(MirStmt::Assign(Place::Local(elem_id), Rvalue::Use(access)));
                        }
                    }
                    return;
                }
                if name == "_" {
                    // Wildcard binding: evaluate for side effects only
                    self.lower_expr_stmt(init, out);
                    return;
                }
                // Evaluate init BEFORE declaring the local so that shadowed
                // names (e.g. `let list = prepend(list, 3)`) resolve to the
                // previous binding.
                let op = self.lower_expr(init);
                let local_id = self.declare_local(name);
                if let Some(type_expr) = ty {
                    if is_string_type(type_expr) {
                        self.string_locals.insert(local_id.0);
                    }
                    // Track f64-typed locals
                    if let ast::TypeExpr::Named { name: tname, .. } = type_expr {
                        if tname == "f64" {
                            self.f64_locals.insert(local_id.0);
                        }
                        if tname == "i64" {
                            self.i64_locals.insert(local_id.0);
                        }
                        if tname == "bool" {
                            self.bool_locals.insert(local_id.0);
                        }
                    }
                    // Track struct-typed locals
                    if let ast::TypeExpr::Named { name: tname, .. } = type_expr {
                        if self.struct_defs.contains_key(tname.as_str()) {
                            self.struct_typed_locals.insert(local_id.0, tname.clone());
                        }
                        if self.enum_variants.contains_key(tname.as_str()) {
                            self.enum_typed_locals.insert(local_id.0, tname.clone());
                        }
                    }
                    // Track generic enum types: Option<i32>, Result<i32, String>
                    if let ast::TypeExpr::Generic {
                        name: tname, args, ..
                    } = type_expr
                    {
                        if tname == "Vec" && args.first().is_some_and(is_string_type) {
                            self.vec_string_locals.insert(local_id.0);
                        }
                        if tname == "Vec" {
                            if let Some(ast::TypeExpr::Named { name: inner, .. }) = args.first() {
                                if inner == "i64" {
                                    self.vec_i64_locals.insert(local_id.0);
                                } else if inner == "f64" {
                                    self.vec_f64_locals.insert(local_id.0);
                                } else if inner == "i32" {
                                    self.vec_i32_locals.insert(local_id.0);
                                }
                            }
                        }
                        if self.enum_variants.contains_key(tname.as_str()) {
                            self.enum_typed_locals.insert(local_id.0, tname.clone());
                            // Map generic args to variant payload types
                            // For Option<T>: Some has payload 0 = T
                            // For Result<T, E>: Ok has payload 0 = T, Err has payload 0 = E
                            let mut payload_strings = HashSet::new();
                            if tname == "Option" && args.first().is_some_and(is_string_type) {
                                payload_strings.insert(("Some".to_string(), 0u32));
                            } else if tname == "Result" {
                                if args.first().is_some_and(is_string_type) {
                                    payload_strings.insert(("Ok".to_string(), 0u32));
                                }
                                if args.get(1).is_some_and(is_string_type) {
                                    payload_strings.insert(("Err".to_string(), 0u32));
                                }
                            }
                            if !payload_strings.is_empty() {
                                self.enum_local_payload_strings
                                    .insert(local_id.0, payload_strings);
                            }
                            // Track specialized enum types for i64/f64 payloads
                            if let Some(spec) = detect_specialized_result(type_expr) {
                                self.enum_local_specialized.insert(local_id.0, spec);
                            }
                        }
                    }
                }
                // Infer f64 from initializer when there's no explicit type annotation
                if !self.f64_locals.contains(&local_id.0) && self.is_f64_operand_mir(&op) {
                    self.f64_locals.insert(local_id.0);
                }
                // Infer i64 from initializer when there's no explicit type annotation
                if !self.i64_locals.contains(&local_id.0) && self.is_i64_operand_mir(&op) {
                    self.i64_locals.insert(local_id.0);
                }
                // Infer String from initializer when there's no explicit type annotation
                if !self.string_locals.contains(&local_id.0) && self.is_string_operand_mir(&op) {
                    self.string_locals.insert(local_id.0);
                }
                // Infer struct type from StructInit initializer when there's no type annotation
                #[allow(clippy::map_entry)]
                if !self.struct_typed_locals.contains_key(&local_id.0) {
                    if let Some(sname) = self.infer_struct_from_init(init) {
                        self.struct_typed_locals.insert(local_id.0, sname);
                    }
                }
                // Infer enum type from call return type when there's no explicit annotation
                #[allow(clippy::map_entry)]
                if !self.enum_typed_locals.contains_key(&local_id.0) {
                    if let Some(ret_te) = self.infer_return_type_expr(init) {
                        let is_result = matches!(&ret_te, ast::TypeExpr::Generic { name, .. } if name == "Result");
                        let is_option = matches!(&ret_te, ast::TypeExpr::Generic { name, .. } if name == "Option");
                        if is_result || is_option {
                            let enum_name = if is_result { "Result" } else { "Option" };
                            self.enum_typed_locals
                                .insert(local_id.0, enum_name.to_string());
                            // Compute payload strings for the inferred type
                            let mut payload_strings = HashSet::new();
                            if let ast::TypeExpr::Generic { args, .. } = &ret_te {
                                if enum_name == "Option" {
                                    if args.first().is_some_and(is_string_type) {
                                        payload_strings.insert(("Some".to_string(), 0u32));
                                    }
                                } else if enum_name == "Result" {
                                    if args.first().is_some_and(is_string_type) {
                                        payload_strings.insert(("Ok".to_string(), 0u32));
                                    }
                                    if args.get(1).is_some_and(is_string_type) {
                                        payload_strings.insert(("Err".to_string(), 0u32));
                                    }
                                }
                            }
                            if !payload_strings.is_empty() {
                                self.enum_local_payload_strings
                                    .insert(local_id.0, payload_strings);
                            }
                            if let Some(spec) = detect_specialized_result(&ret_te) {
                                self.enum_local_specialized.insert(local_id.0, spec);
                            }
                        }
                    }
                }
                // Track closure locals: if the init expression was a closure, record captures
                if let Operand::FnRef(ref fn_name) = op {
                    if let Some(caps) = self.closure_fn_captures.get(fn_name).cloned() {
                        self.closure_locals
                            .insert(local_id.0, (fn_name.clone(), caps));
                    }
                }
                // Promote integer literals to i64 when type annotation is i64
                let op = if self.i64_locals.contains(&local_id.0) {
                    match op {
                        Operand::ConstI32(v) => Operand::ConstI64(v as i64),
                        other => other,
                    }
                } else {
                    op
                };
                out.push(MirStmt::Assign(Place::Local(local_id), Rvalue::Use(op)));
            }
            ast::Stmt::Expr(expr) => {
                self.lower_expr_stmt(expr, out);
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond_op = self.lower_expr(cond);
                out.push(MirStmt::WhileStmt {
                    cond: cond_op,
                    body: self.lower_block_all(body),
                });
            }
            ast::Stmt::Loop { body, .. } => {
                out.push(MirStmt::WhileStmt {
                    cond: Operand::ConstBool(true),
                    body: self.lower_block_all(body),
                });
            }
            ast::Stmt::For {
                target, iter, body, ..
            } => {
                // Desugar for to while
                match iter {
                    ast::ForIter::Range { start, end } => {
                        // for i in start..end { body }
                        // → let mut __i = start; while __i < end { let i = __i; body; __i = __i + 1; }
                        let start_op = self.lower_expr(start);
                        let end_op = self.lower_expr(end);

                        let idx_local = self.declare_local(target);

                        // Assign start value
                        out.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(start_op),
                        ));

                        // Build cond: idx < end
                        let end_local = self.new_temp();
                        out.push(MirStmt::Assign(
                            Place::Local(end_local),
                            Rvalue::Use(end_op),
                        ));

                        let cond_local = self.new_temp();

                        // Build body: original body + increment
                        let mut while_body = self.lower_block_all(body);

                        // idx = idx + 1
                        let inc_tmp = self.new_temp();
                        while_body.push(MirStmt::Assign(
                            Place::Local(inc_tmp),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Add,
                                Box::new(Operand::Place(Place::Local(idx_local))),
                                Box::new(Operand::ConstI32(1)),
                            )),
                        ));
                        while_body.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(Operand::Place(Place::Local(inc_tmp))),
                        ));

                        // cond = idx < end
                        let mut full_body = vec![MirStmt::Assign(
                            Place::Local(cond_local),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Lt,
                                Box::new(Operand::Place(Place::Local(idx_local))),
                                Box::new(Operand::Place(Place::Local(end_local))),
                            )),
                        )];

                        out.push(MirStmt::WhileStmt {
                            cond: Operand::ConstBool(true),
                            body: {
                                full_body.push(MirStmt::IfStmt {
                                    cond: Operand::Place(Place::Local(cond_local)),
                                    then_body: while_body,
                                    else_body: vec![MirStmt::Break],
                                });
                                full_body
                            },
                        });
                    }
                    ast::ForIter::Values(vec_expr) => {
                        // for x in values(v) { body }
                        // → let mut __i = 0; while __i < len(v) { let x = get(v, __i); body; __i = __i + 1; }
                        let vec_op = self.lower_expr(vec_expr);

                        let idx_local = self.new_temp();
                        let vec_local = self.new_temp();
                        let target_local = self.declare_local(target);

                        // __i = 0
                        out.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(Operand::ConstI32(0)),
                        ));
                        // __vec = vec_expr
                        out.push(MirStmt::Assign(
                            Place::Local(vec_local),
                            Rvalue::Use(vec_op),
                        ));

                        // Build loop body
                        let len_tmp = self.new_temp();
                        let cond_tmp = self.new_temp();

                        // cond: __i < len(__vec)
                        let mut loop_body = vec![
                            MirStmt::Assign(
                                Place::Local(len_tmp),
                                Rvalue::Use(Operand::Call(
                                    "len".to_string(),
                                    vec![Operand::Place(Place::Local(vec_local))],
                                )),
                            ),
                            MirStmt::Assign(
                                Place::Local(cond_tmp),
                                Rvalue::Use(Operand::BinOp(
                                    BinOp::Lt,
                                    Box::new(Operand::Place(Place::Local(idx_local))),
                                    Box::new(Operand::Place(Place::Local(len_tmp))),
                                )),
                            ),
                        ];

                        // x = get_unchecked(__vec, __i) — safe because __i < len(__vec) is checked
                        let mut inner_body = vec![MirStmt::Assign(
                            Place::Local(target_local),
                            Rvalue::Use(Operand::Call(
                                "get_unchecked".to_string(),
                                vec![
                                    Operand::Place(Place::Local(vec_local)),
                                    Operand::Place(Place::Local(idx_local)),
                                ],
                            )),
                        )];

                        // original body
                        inner_body.extend(self.lower_block_all(body));

                        // __i = __i + 1
                        let inc_tmp = self.new_temp();
                        inner_body.push(MirStmt::Assign(
                            Place::Local(inc_tmp),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Add,
                                Box::new(Operand::Place(Place::Local(idx_local))),
                                Box::new(Operand::ConstI32(1)),
                            )),
                        ));
                        inner_body.push(MirStmt::Assign(
                            Place::Local(idx_local),
                            Rvalue::Use(Operand::Place(Place::Local(inc_tmp))),
                        ));

                        loop_body.push(MirStmt::IfStmt {
                            cond: Operand::Place(Place::Local(cond_tmp)),
                            then_body: inner_body,
                            else_body: vec![MirStmt::Break],
                        });

                        out.push(MirStmt::WhileStmt {
                            cond: Operand::ConstBool(true),
                            body: loop_body,
                        });
                    }
                    ast::ForIter::Iter(iter_expr) => {
                        // for x in iter_expr { body }
                        // → let __iter = iter_expr
                        //   loop {
                        //     let __next = StructName__next(__iter)
                        //     // __next is Option<T>: [tag(4)][payload(4)]
                        //     // tag==0 → Some(x): let x = payload; body
                        //     // tag==1 → None: break
                        //   }
                        let struct_name = self.infer_struct_type(iter_expr);
                        let iter_op = self.lower_expr(iter_expr);

                        let iter_local = self.new_temp();
                        let next_local = self.new_temp();
                        let tag_local = self.new_temp();
                        let target_local = self.declare_local(target);

                        // Track struct type for the iterator local
                        if let Some(ref sname) = struct_name {
                            self.struct_typed_locals.insert(iter_local.0, sname.clone());
                        }
                        // next_local holds Option<T> enum ref from __next() call
                        self.enum_typed_locals.insert(next_local.0, "Option".to_string());

                        // __iter = iter_expr
                        out.push(MirStmt::Assign(
                            Place::Local(iter_local),
                            Rvalue::Use(iter_op),
                        ));

                        let method_name = if let Some(ref sname) = struct_name {
                            format!("{}__next", sname)
                        } else {
                            "__next".to_string()
                        };

                        // Build loop body:
                        // __next = StructName__next(__iter)
                        let mut loop_body = vec![MirStmt::Assign(
                            Place::Local(next_local),
                            Rvalue::Use(Operand::Call(
                                method_name,
                                vec![Operand::Place(Place::Local(iter_local))],
                            )),
                        )];

                        // tag = __next.tag (EnumTag)
                        loop_body.push(MirStmt::Assign(
                            Place::Local(tag_local),
                            Rvalue::Use(Operand::EnumTag(Box::new(Operand::Place(Place::Local(
                                next_local,
                            ))))),
                        ));

                        // Build inner body: extract payload and run user body
                        let mut some_body = vec![MirStmt::Assign(
                            Place::Local(target_local),
                            Rvalue::Use(Operand::EnumPayload {
                                object: Box::new(Operand::Place(Place::Local(next_local))),
                                index: 0,
                                enum_name: "Option".to_string(),
                                variant_name: "Some".to_string(),
                            }),
                        )];
                        some_body.extend(self.lower_block_all(body));

                        // if tag == 0 (Some) → some_body; else → break
                        let cond_local = self.new_temp();
                        loop_body.push(MirStmt::Assign(
                            Place::Local(cond_local),
                            Rvalue::Use(Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::Place(Place::Local(tag_local))),
                                Box::new(Operand::ConstI32(0)),
                            )),
                        ));
                        loop_body.push(MirStmt::IfStmt {
                            cond: Operand::Place(Place::Local(cond_local)),
                            then_body: some_body,
                            else_body: vec![MirStmt::Break],
                        });

                        out.push(MirStmt::WhileStmt {
                            cond: Operand::ConstBool(true),
                            body: loop_body,
                        });
                    }
                }
            }
        }
    }

    fn lower_expr_stmt(&mut self, expr: &ast::Expr, out: &mut Vec<MirStmt>) {
        match expr {
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    out.push(MirStmt::CallBuiltin {
                        dest: None,
                        name: name.clone(),
                        args: mir_args,
                    });
                } else if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    // Method call as statement: x.method(args) → discard result
                    if let Some((mangled, _)) = self.method_resolutions.get(&span.start).cloned() {
                        let self_arg = self.lower_expr(object);
                        let mut all_args = vec![self_arg];
                        all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                        out.push(MirStmt::CallBuiltin {
                            dest: None,
                            name: mangled,
                            args: all_args,
                        });
                    } else if let Some(struct_name) = self.infer_struct_type(object) {
                        let mangled = format!("{}__{}", struct_name, field);
                        if self.user_fn_names.contains(&mangled) {
                            let self_arg = self.lower_expr(object);
                            let mut all_args = vec![self_arg];
                            all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                            out.push(MirStmt::CallBuiltin {
                                dest: None,
                                name: mangled,
                                args: all_args,
                            });
                        }
                    }
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                if let ast::Expr::Ident { name, .. } = target.as_ref() {
                    if let Some(local_id) = self.lookup_local(name) {
                        let op = self.lower_expr(value);
                        out.push(MirStmt::Assign(Place::Local(local_id), Rvalue::Use(op)));
                    }
                } else if let ast::Expr::FieldAccess { object, field, .. } = target.as_ref() {
                    // self.field = value → FieldStore
                    if let ast::Expr::Ident { name, .. } = object.as_ref() {
                        if let Some(local_id) = self.lookup_local(name) {
                            let struct_name = self.struct_typed_locals.get(&local_id.0).cloned();
                            let val_op = self.lower_expr(value);
                            out.push(MirStmt::Assign(
                                Place::Field(Box::new(Place::Local(local_id)), field.clone()),
                                Rvalue::Use(val_op),
                            ));
                            // Track struct type for the field access
                            if let Some(sname) = struct_name {
                                // No-op: struct type already tracked
                                let _ = sname;
                            }
                        }
                    }
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block_all(then_block);
                let else_stmts = else_block
                    .as_ref()
                    .map(|b| self.lower_block_all(b))
                    .unwrap_or_default();
                out.push(MirStmt::IfStmt {
                    cond: c,
                    then_body: then_stmts,
                    else_body: else_stmts,
                });
            }
            ast::Expr::Break { value, .. } => {
                if let Some(val) = value {
                    if let Some(result_id) = self.loop_result_local {
                        let op = self.lower_expr(val);
                        out.push(MirStmt::Assign(Place::Local(result_id), Rvalue::Use(op)));
                    }
                }
                out.push(MirStmt::Break);
            }
            ast::Expr::Continue { .. } => {
                out.push(MirStmt::Continue);
            }
            ast::Expr::Return { value, .. } => {
                let op = value.as_ref().map(|v| self.lower_expr(v));
                out.push(MirStmt::Return(op));
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                self.lower_match_stmt(scrutinee, arms, out);
            }
            ast::Expr::Block(block) => {
                out.extend(self.lower_block(block));
                if let Some(tail) = &block.tail_expr {
                    self.lower_expr_stmt(tail, out);
                }
            }
            _ => {}
        }
    }

    /// Lower a match expression used as a statement (result discarded).
    /// Converts to nested if-else chains.
    fn lower_match_stmt(
        &mut self,
        scrutinee: &ast::Expr,
        arms: &[ast::MatchArm],
        out: &mut Vec<MirStmt>,
    ) {
        let scrut_val = self.lower_expr(scrutinee);
        // Store complex scrutinees in a temp local to avoid re-evaluation
        let scrut = match &scrut_val {
            Operand::Place(_)
            | Operand::ConstI32(_)
            | Operand::ConstBool(_)
            | Operand::ConstString(_)
            | Operand::Unit => scrut_val,
            _ => {
                let tmp = self.declare_local("__match_scrut");
                out.push(MirStmt::Assign(Place::Local(tmp), Rvalue::Use(scrut_val)));
                // If scrutinee is a function call, resolve generic enum payload types
                if let ast::Expr::Call { callee, .. } = scrutinee {
                    if let ast::Expr::Ident { name: fn_name, .. } = callee.as_ref() {
                        self.resolve_enum_payload_types_for_local(tmp, fn_name);
                    }
                }
                Operand::Place(Place::Local(tmp))
            }
        };
        let stmt = self.build_match_if_chain(&scrut, arms, 0, true);
        if let Some(s) = stmt {
            out.push(s);
        }
    }

    /// Resolve generic enum payload types for a local holding a function return value.
    /// E.g., if fn returns Result<i32, String>, mark the Err variant's payload as String.
    fn resolve_enum_payload_types_for_local(&mut self, local: LocalId, fn_name: &str) {
        let ret_ty = if let Some(ty) = self.fn_return_types.get(fn_name) {
            ty.clone()
        } else {
            return;
        };
        match &ret_ty {
            ast::TypeExpr::Generic { name, args, .. } if name == "Result" || name == "Option" => {
                self.enum_typed_locals.insert(local.0, name.clone());
                let mut payload_strings = HashSet::new();
                if name == "Result" && args.len() == 2 {
                    // Result<T, E>: Ok payload is args[0], Err payload is args[1]
                    if is_string_type(&args[0]) {
                        payload_strings.insert(("Ok".to_string(), 0u32));
                    }
                    if is_string_type(&args[1]) {
                        payload_strings.insert(("Err".to_string(), 0u32));
                    }
                } else if name == "Option" && args.len() == 1 {
                    // Option<T>: Some payload is args[0]
                    if is_string_type(&args[0]) {
                        payload_strings.insert(("Some".to_string(), 0u32));
                    }
                }
                if !payload_strings.is_empty() {
                    self.enum_local_payload_strings
                        .insert(local.0, payload_strings);
                }
            }
            _ => {}
        }
    }

    /// Build a nested if-else chain from match arms starting at `idx`.
    /// `as_stmt` indicates whether arm bodies should be lowered as statements.
    #[allow(clippy::only_used_in_recursion)]
    fn build_match_if_chain(
        &mut self,
        scrut: &Operand,
        arms: &[ast::MatchArm],
        idx: usize,
        as_stmt: bool,
    ) -> Option<MirStmt> {
        if idx >= arms.len() {
            return None;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) => {
                // Default arm — check guard if any, otherwise just emit
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    let mut then_body = Vec::new();
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) =
                        self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                    {
                        vec![next]
                    } else {
                        vec![]
                    };
                    Some(MirStmt::IfStmt {
                        cond: guard_cond,
                        then_body,
                        else_body,
                    })
                } else {
                    let mut body = Vec::new();
                    self.lower_expr_stmt(&arm.body, &mut body);
                    if body.len() == 1 {
                        Some(body.remove(0))
                    } else {
                        Some(MirStmt::IfStmt {
                            cond: Operand::ConstBool(true),
                            then_body: body,
                            else_body: vec![],
                        })
                    }
                }
            }
            ast::Pattern::IntLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::BoolLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::StringLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstString(value.clone())),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::Ident { name, .. } => {
                // Binding pattern — bind the scrutinee to the name
                let local_id = self.declare_local(name);
                if let Some(guard) = &arm.guard {
                    // Bind first, then check guard
                    let mut outer_body = vec![MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(scrut.clone()),
                    )];
                    let guard_cond = self.lower_expr(guard);
                    let mut then_body = Vec::new();
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) =
                        self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                    {
                        vec![next]
                    } else {
                        vec![]
                    };
                    outer_body.push(MirStmt::IfStmt {
                        cond: guard_cond,
                        then_body,
                        else_body,
                    });
                    Some(MirStmt::IfStmt {
                        cond: Operand::ConstBool(true),
                        then_body: outer_body,
                        else_body: vec![],
                    })
                } else {
                    let mut then_body = vec![MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(scrut.clone()),
                    )];
                    self.lower_expr_stmt(&arm.body, &mut then_body);
                    let else_body = if let Some(next) =
                        self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                    {
                        vec![next]
                    } else {
                        vec![]
                    };
                    Some(MirStmt::IfStmt {
                        cond: Operand::ConstBool(true),
                        then_body,
                        else_body,
                    })
                }
            }
            ast::Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    // Compare tag: enum ptr -> i32.load at offset 0
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    let mut then_body = Vec::new();
                    // Determine if scrutinee has known payload string types
                    let payload_strings = if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_local_payload_strings.get(&lid.0).cloned()
                    } else {
                        None
                    };
                    // Determine specialized enum name for i64/f64 payloads
                    let effective_enum_name = if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_local_specialized
                            .get(&lid.0)
                            .cloned()
                            .unwrap_or_else(|| path.clone())
                    } else {
                        path.clone()
                    };
                    // Bind payload fields to local variables
                    for (i, field_pat) in fields.iter().enumerate() {
                        if let ast::Pattern::Ident { name: binding, .. } = field_pat {
                            let local_id = self.declare_local(binding);
                            // Check if this payload field is a string
                            if let Some(ref ps) = payload_strings {
                                if ps.contains(&(variant.clone(), i as u32)) {
                                    self.string_locals.insert(local_id.0);
                                }
                            }
                            // Check if this payload field is f64, i64, or String
                            if let Some(variants) = self.enum_defs.get(effective_enum_name.as_str())
                            {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant)
                                {
                                    if let Some(t) = types.get(i) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "i64" {
                                            self.i64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: i as u32,
                                enum_name: effective_enum_name.clone(),
                                variant_name: variant.clone(),
                            };
                            then_body.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                    }
                    if let Some(guard) = &arm.guard {
                        // Guard references pattern bindings, which are in then_body.
                        // Wrap: if(tag_match) { bind; if(guard) { body } else { next } }
                        let guard_cond = self.lower_expr(guard);
                        let mut inner_then = Vec::new();
                        self.lower_expr_stmt(&arm.body, &mut inner_then);
                        let else_body = if let Some(next) =
                            self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                        {
                            vec![next]
                        } else {
                            vec![]
                        };
                        then_body.push(MirStmt::IfStmt {
                            cond: guard_cond,
                            then_body: inner_then,
                            else_body: else_body.clone(),
                        });
                        Some(MirStmt::IfStmt {
                            cond,
                            then_body,
                            else_body,
                        })
                    } else {
                        self.lower_expr_stmt(&arm.body, &mut then_body);
                        let else_body = if let Some(next) =
                            self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                        {
                            vec![next]
                        } else {
                            vec![]
                        };
                        Some(MirStmt::IfStmt {
                            cond,
                            then_body,
                            else_body,
                        })
                    }
                } else {
                    self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                }
            }
            ast::Pattern::Or { patterns, .. } => {
                // Or-pattern: try each sub-pattern, share the body
                // Build: if pat1_cond || pat2_cond || ... { body } else { next }
                let mut combined_cond: Option<Operand> = None;
                for pat in patterns {
                    let sub_cond = self.pattern_to_condition(scrut, pat);
                    combined_cond = Some(match combined_cond {
                        Some(prev) => Operand::BinOp(BinOp::Or, Box::new(prev), Box::new(sub_cond)),
                        None => sub_cond,
                    });
                }
                let mut cond = combined_cond.unwrap_or(Operand::ConstBool(false));
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let mut then_body = Vec::new();
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            ast::Pattern::Struct { name, fields, .. } => {
                // Check if this is an enum struct variant pattern: "EnumName::VariantName"
                if let Some((enum_name, variant_name)) = name.split_once("::") {
                    let key = format!("{}::{}", enum_name, variant_name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let cond = Operand::BinOp(
                            BinOp::Eq,
                            Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                            Box::new(Operand::ConstI32(tag)),
                        );
                        let mut then_body = Vec::new();
                        let def_field_names = self
                            .enum_variant_field_names
                            .get(&key)
                            .cloned()
                            .unwrap_or_default();
                        for (fname, fpat) in fields {
                            let binding_name = match fpat {
                                Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                                None => fname.clone(),
                                _ => fname.clone(),
                            };
                            let local_id = self.declare_local(&binding_name);
                            // Determine field index from definition order
                            let field_idx =
                                def_field_names.iter().position(|n| n == fname).unwrap_or(0);
                            // Track f64/i64/String types
                            if let Some(variants) = self.enum_defs.get(enum_name) {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant_name)
                                {
                                    if let Some(t) = types.get(field_idx) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "i64" {
                                            self.i64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: field_idx as u32,
                                enum_name: enum_name.to_string(),
                                variant_name: variant_name.to_string(),
                            };
                            then_body.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                        self.lower_expr_stmt(&arm.body, &mut then_body);
                        let else_body = if let Some(next) =
                            self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
                        {
                            vec![next]
                        } else {
                            vec![]
                        };
                        return Some(MirStmt::IfStmt {
                            cond,
                            then_body,
                            else_body,
                        });
                    }
                }
                // Regular struct pattern: bind fields from struct
                let mut then_body = Vec::new();
                for (fname, fpat) in fields {
                    let binding_name = match fpat {
                        Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                        None => fname.clone(),
                        _ => fname.clone(),
                    };
                    let local_id = self.declare_local(&binding_name);
                    // Detect f64/String fields from struct_defs
                    if let Some(sdef) = self.struct_defs.get(name.as_str()) {
                        if let Some((_, ftype)) = sdef.iter().find(|(n, _)| n == fname) {
                            if ftype == "f64" {
                                self.f64_locals.insert(local_id.0);
                            }
                            if ftype == "String" {
                                self.string_locals.insert(local_id.0);
                            }
                        }
                    }
                    let field_access = Operand::FieldAccess {
                        object: Box::new(scrut.clone()),
                        struct_name: name.clone(),
                        field: fname.clone(),
                    };
                    then_body.push(MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(field_access),
                    ));
                }
                let cond = if let Some(guard) = &arm.guard {
                    self.lower_expr(guard)
                } else {
                    Operand::ConstBool(true)
                };
                self.lower_expr_stmt(&arm.body, &mut then_body);
                let else_body =
                    if let Some(next) = self.build_match_if_chain(scrut, arms, idx + 1, as_stmt) {
                        vec![next]
                    } else {
                        vec![]
                    };
                Some(MirStmt::IfStmt {
                    cond,
                    then_body,
                    else_body,
                })
            }
            _ => {
                // Skip unsupported patterns, try next arm
                self.build_match_if_chain(scrut, arms, idx + 1, as_stmt)
            }
        }
    }

    /// Convert a single pattern to a condition operand (for or-patterns).
    fn pattern_to_condition(&self, scrut: &Operand, pattern: &ast::Pattern) -> Operand {
        match pattern {
            ast::Pattern::Wildcard(_) | ast::Pattern::Ident { .. } => Operand::ConstBool(true),
            ast::Pattern::IntLit { value, .. } => Operand::BinOp(
                BinOp::Eq,
                Box::new(scrut.clone()),
                Box::new(Operand::ConstI32(*value as i32)),
            ),
            ast::Pattern::BoolLit { value, .. } => Operand::BinOp(
                BinOp::Eq,
                Box::new(scrut.clone()),
                Box::new(Operand::ConstBool(*value)),
            ),
            ast::Pattern::StringLit { value, .. } => Operand::BinOp(
                BinOp::Eq,
                Box::new(scrut.clone()),
                Box::new(Operand::ConstString(value.clone())),
            ),
            ast::Pattern::Enum { path, variant, .. } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    )
                } else {
                    Operand::ConstBool(false)
                }
            }
            _ => Operand::ConstBool(true),
        }
    }

    /// Build a nested IfExpr from match arms for value-returning match.
    fn build_match_if_expr(
        &mut self,
        scrut: &Operand,
        arms: &[ast::MatchArm],
        idx: usize,
    ) -> Operand {
        if idx >= arms.len() {
            return Operand::Unit;
        }
        let arm = &arms[idx];
        match &arm.pattern {
            ast::Pattern::Wildcard(_) => {
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    let then_result = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    Operand::IfExpr {
                        cond: Box::new(guard_cond),
                        then_body: vec![],
                        then_result: Some(Box::new(then_result)),
                        else_body: vec![],
                        else_result: Some(Box::new(else_result)),
                    }
                } else {
                    self.lower_expr(&arm.body)
                }
            }
            ast::Pattern::Ident { name, .. } => {
                let local_id = self.declare_local(name);
                let assign_stmt =
                    MirStmt::Assign(Place::Local(local_id), Rvalue::Use(scrut.clone()));
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    let body_val = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    // Outer: assign binding, then inner guard check
                    Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body: vec![assign_stmt],
                        then_result: Some(Box::new(Operand::IfExpr {
                            cond: Box::new(guard_cond),
                            then_body: vec![],
                            then_result: Some(Box::new(body_val)),
                            else_body: vec![],
                            else_result: Some(Box::new(else_result)),
                        })),
                        else_body: vec![],
                        else_result: Some(Box::new(Operand::Unit)),
                    }
                } else {
                    let body_val = self.lower_expr(&arm.body);
                    let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                    Operand::IfExpr {
                        cond: Box::new(Operand::ConstBool(true)),
                        then_body: vec![assign_stmt],
                        then_result: Some(Box::new(body_val)),
                        else_body: vec![],
                        else_result: Some(Box::new(else_result)),
                    }
                }
            }
            ast::Pattern::IntLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstI32(*value as i32)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::BoolLit { value, .. } => {
                let mut cond = Operand::BinOp(
                    BinOp::Eq,
                    Box::new(scrut.clone()),
                    Box::new(Operand::ConstBool(*value)),
                );
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => {
                let key = format!("{}::{}", path, variant);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    let cond = Operand::BinOp(
                        BinOp::Eq,
                        Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                        Box::new(Operand::ConstI32(tag)),
                    );
                    let payload_strings = if let Operand::Place(Place::Local(lid)) = scrut {
                        self.enum_local_payload_strings.get(&lid.0).cloned()
                    } else {
                        None
                    };
                    let mut setup_stmts = Vec::new();
                    for (i, field_pat) in fields.iter().enumerate() {
                        if let ast::Pattern::Ident { name: binding, .. } = field_pat {
                            let local_id = self.declare_local(binding);
                            if let Some(ref ps) = payload_strings {
                                if ps.contains(&(variant.clone(), i as u32)) {
                                    self.string_locals.insert(local_id.0);
                                }
                            }
                            if let Some(variants) = self.enum_defs.get(path.as_str()) {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant)
                                {
                                    if let Some(t) = types.get(i) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: i as u32,
                                enum_name: path.clone(),
                                variant_name: variant.clone(),
                            };
                            setup_stmts.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                    }
                    if let Some(guard) = &arm.guard {
                        let guard_cond = self.lower_expr(guard);
                        let then_result = self.lower_expr(&arm.body);
                        let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                        // Outer: tag check → bind fields → inner guard check
                        Operand::IfExpr {
                            cond: Box::new(cond),
                            then_body: setup_stmts,
                            then_result: Some(Box::new(Operand::IfExpr {
                                cond: Box::new(guard_cond),
                                then_body: vec![],
                                then_result: Some(Box::new(then_result)),
                                else_body: vec![],
                                else_result: Some(Box::new(else_result)),
                            })),
                            else_body: vec![],
                            else_result: Some(Box::new(self.build_match_if_expr(
                                scrut,
                                arms,
                                idx + 1,
                            ))),
                        }
                    } else {
                        let then_result = self.lower_expr(&arm.body);
                        let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                        Operand::IfExpr {
                            cond: Box::new(cond),
                            then_body: setup_stmts,
                            then_result: Some(Box::new(then_result)),
                            else_body: vec![],
                            else_result: Some(Box::new(else_result)),
                        }
                    }
                } else {
                    self.build_match_if_expr(scrut, arms, idx + 1)
                }
            }
            ast::Pattern::Or { patterns, .. } => {
                let mut combined_cond: Option<Operand> = None;
                for pat in patterns {
                    let sub_cond = self.pattern_to_condition(scrut, pat);
                    combined_cond = Some(match combined_cond {
                        Some(prev) => Operand::BinOp(BinOp::Or, Box::new(prev), Box::new(sub_cond)),
                        None => sub_cond,
                    });
                }
                let mut cond = combined_cond.unwrap_or(Operand::ConstBool(false));
                if let Some(guard) = &arm.guard {
                    let guard_cond = self.lower_expr(guard);
                    cond = Operand::BinOp(BinOp::And, Box::new(cond), Box::new(guard_cond));
                }
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: vec![],
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            ast::Pattern::Struct { name, fields, .. } => {
                // Check if this is an enum struct variant pattern: "EnumName::VariantName"
                if let Some((enum_name, variant_name)) = name.split_once("::") {
                    let key = format!("{}::{}", enum_name, variant_name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let cond = Operand::BinOp(
                            BinOp::Eq,
                            Box::new(Operand::EnumTag(Box::new(scrut.clone()))),
                            Box::new(Operand::ConstI32(tag)),
                        );
                        let mut setup_stmts = Vec::new();
                        let def_field_names = self
                            .enum_variant_field_names
                            .get(&key)
                            .cloned()
                            .unwrap_or_default();
                        for (fname, fpat) in fields {
                            let binding_name = match fpat {
                                Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                                None => fname.clone(),
                                _ => fname.clone(),
                            };
                            let local_id = self.declare_local(&binding_name);
                            let field_idx =
                                def_field_names.iter().position(|n| n == fname).unwrap_or(0);
                            if let Some(variants) = self.enum_defs.get(enum_name) {
                                if let Some((_, types)) =
                                    variants.iter().find(|(vn, _)| vn == variant_name)
                                {
                                    if let Some(t) = types.get(field_idx) {
                                        if t == "f64" {
                                            self.f64_locals.insert(local_id.0);
                                        }
                                        if t == "String" {
                                            self.string_locals.insert(local_id.0);
                                        }
                                    }
                                }
                            }
                            let payload = Operand::EnumPayload {
                                object: Box::new(scrut.clone()),
                                index: field_idx as u32,
                                enum_name: enum_name.to_string(),
                                variant_name: variant_name.to_string(),
                            };
                            setup_stmts.push(MirStmt::Assign(
                                Place::Local(local_id),
                                Rvalue::Use(payload),
                            ));
                        }
                        let then_result = self.lower_expr(&arm.body);
                        let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                        return Operand::IfExpr {
                            cond: Box::new(cond),
                            then_body: setup_stmts,
                            then_result: Some(Box::new(then_result)),
                            else_body: vec![],
                            else_result: Some(Box::new(else_result)),
                        };
                    }
                }
                // Regular struct pattern
                let mut setup_stmts = Vec::new();
                for (fname, fpat) in fields {
                    let binding_name = match fpat {
                        Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                        None => fname.clone(),
                        _ => fname.clone(),
                    };
                    let local_id = self.declare_local(&binding_name);
                    if let Some(sdef) = self.struct_defs.get(name.as_str()) {
                        if let Some((_, ftype)) = sdef.iter().find(|(n, _)| n == fname) {
                            if ftype == "f64" {
                                self.f64_locals.insert(local_id.0);
                            }
                            if ftype == "String" {
                                self.string_locals.insert(local_id.0);
                            }
                        }
                    }
                    let field_access = Operand::FieldAccess {
                        object: Box::new(scrut.clone()),
                        struct_name: name.clone(),
                        field: fname.clone(),
                    };
                    setup_stmts.push(MirStmt::Assign(
                        Place::Local(local_id),
                        Rvalue::Use(field_access),
                    ));
                }
                let cond = if let Some(guard) = &arm.guard {
                    self.lower_expr(guard)
                } else {
                    Operand::ConstBool(true)
                };
                let then_result = self.lower_expr(&arm.body);
                let else_result = self.build_match_if_expr(scrut, arms, idx + 1);
                Operand::IfExpr {
                    cond: Box::new(cond),
                    then_body: setup_stmts,
                    then_result: Some(Box::new(then_result)),
                    else_body: vec![],
                    else_result: Some(Box::new(else_result)),
                }
            }
            _ => {
                // Skip unsupported patterns
                self.build_match_if_expr(scrut, arms, idx + 1)
            }
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr) -> Operand {
        match expr {
            ast::Expr::StringLit { value, .. } => Operand::ConstString(value.clone()),
            ast::Expr::IntLit { value, .. } => {
                // Keep full i64 precision; will be promoted in let binding if needed
                if *value > i32::MAX as i64 || *value < i32::MIN as i64 {
                    Operand::ConstI64(*value)
                } else {
                    Operand::ConstI32(*value as i32)
                }
            }
            ast::Expr::FloatLit { value, .. } => Operand::ConstF64(*value),
            ast::Expr::BoolLit { value, .. } => Operand::ConstBool(*value),
            ast::Expr::CharLit { value, .. } => Operand::ConstChar(*value),
            ast::Expr::Ident { name, .. } => {
                // Check if this is a bare enum variant (e.g., None)
                if let Some((enum_name, tag, field_count)) = self.bare_variant_tags.get(name) {
                    if *field_count == 0 {
                        return Operand::EnumInit {
                            enum_name: enum_name.clone(),
                            variant: name.clone(),
                            tag: *tag,
                            payload: vec![],
                        };
                    }
                }
                if let Some(local_id) = self.lookup_local(name) {
                    Operand::Place(Place::Local(local_id))
                } else if self.user_fn_names.contains(name) {
                    Operand::FnRef(name.clone())
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Binary {
                op,
                left,
                right,
                span,
                ..
            } => {
                // Check for operator overloading (struct + struct → method call)
                if let Some((mangled, _struct_name)) =
                    self.method_resolutions.get(&span.start).cloned()
                {
                    let l = self.lower_expr(left);
                    let r = self.lower_expr(right);
                    let result = Operand::Call(mangled, vec![l, r]);
                    // For Ne, wrap eq result with negation
                    return match op {
                        ast::BinOp::Ne => {
                            Operand::UnaryOp(crate::mir::UnaryOp::Not, Box::new(result))
                        }
                        ast::BinOp::Gt => {
                            // a > b → b.cmp(a) (swap args)
                            // Actually, just return the call result; cmp returns bool
                            result
                        }
                        _ => result,
                    };
                }
                match op {
                    // Short-circuit: a && b  =>  if a { b } else { false }
                    ast::BinOp::And => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::IfExpr {
                            cond: Box::new(l),
                            then_body: vec![],
                            then_result: Some(Box::new(r)),
                            else_body: vec![],
                            else_result: Some(Box::new(Operand::ConstBool(false))),
                        }
                    }
                    // Short-circuit: a || b  =>  if a { true } else { b }
                    ast::BinOp::Or => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::IfExpr {
                            cond: Box::new(l),
                            then_body: vec![],
                            then_result: Some(Box::new(Operand::ConstBool(true))),
                            else_body: vec![],
                            else_result: Some(Box::new(r)),
                        }
                    }
                    _ => {
                        let l = self.lower_expr(left);
                        let r = self.lower_expr(right);
                        Operand::BinOp(lower_binop(op), Box::new(l), Box::new(r))
                    }
                }
            }
            ast::Expr::Unary { op, operand, .. } => {
                let inner = self.lower_expr(operand);
                Operand::UnaryOp(lower_unaryop(op), Box::new(inner))
            }
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    // Check if this is a bare enum variant constructor (e.g., Some(42), Ok(100))
                    if let Some((enum_name, tag, _field_count)) =
                        self.bare_variant_tags.get(name).cloned()
                    {
                        let payload: Vec<Operand> =
                            args.iter().map(|a| self.lower_expr(a)).collect();
                        return Operand::EnumInit {
                            enum_name,
                            variant: name.clone(),
                            tag,
                            payload,
                        };
                    }
                    // Builtin Option/Result operations
                    match name.as_str() {
                        "unwrap" => {
                            let arg = self.lower_expr(&args[0]);
                            // unwrap: extract payload[0] (tag 0 = Some/Ok)
                            return Operand::EnumPayload {
                                object: Box::new(arg),
                                index: 0,
                                enum_name: "Option".to_string(),
                                variant_name: "Some".to_string(),
                            };
                        }
                        "unwrap_or" => {
                            let arg = self.lower_expr(&args[0]);
                            let default = self.lower_expr(&args[1]);
                            // if is_some(arg) then payload[0] else default
                            let cond = Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg.clone()))),
                                Box::new(Operand::ConstI32(0)), // Some/Ok tag
                            );
                            return Operand::IfExpr {
                                cond: Box::new(cond),
                                then_body: vec![],
                                then_result: Some(Box::new(Operand::EnumPayload {
                                    object: Box::new(arg),
                                    index: 0,
                                    enum_name: "Option".to_string(),
                                    variant_name: "Some".to_string(),
                                })),
                                else_body: vec![],
                                else_result: Some(Box::new(default)),
                            };
                        }
                        "is_some" | "is_ok" => {
                            let arg = self.lower_expr(&args[0]);
                            // Some/Ok tag = 0
                            return Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg))),
                                Box::new(Operand::ConstI32(0)),
                            );
                        }
                        "is_none" | "is_err" => {
                            let arg = self.lower_expr(&args[0]);
                            // None/Err tag != 0 (i.e., tag == 1)
                            return Operand::BinOp(
                                BinOp::Eq,
                                Box::new(Operand::EnumTag(Box::new(arg))),
                                Box::new(Operand::ConstI32(1)),
                            );
                        }
                        "to_string" if args.len() == 1 => {
                            // Display trait dispatch: if arg is a struct with Display impl,
                            // rewrite to StructName__to_string(arg)
                            if let Some(struct_name) = self.infer_struct_type(&args[0]) {
                                let mangled = format!("{}__{}", struct_name, "to_string");
                                if self.user_fn_names.contains(&mangled) {
                                    let lowered_arg = self.lower_expr(&args[0]);
                                    return Operand::Call(mangled, vec![lowered_arg]);
                                }
                            }
                        }
                        _ => {}
                    }
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    // Check if callee is a local (function pointer parameter) → indirect call
                    if let Some(local_id) = self.lookup_local(name) {
                        // Check if this is a closure with captures → direct call with injected args
                        if let Some((synth_fn, cap_names)) =
                            self.closure_locals.get(&local_id.0).cloned()
                        {
                            let mut all_args = mir_args;
                            for cap_name in &cap_names {
                                if let Some(cap_lid) = self.lookup_local(cap_name) {
                                    all_args.push(Operand::Place(Place::Local(cap_lid)));
                                } else {
                                    all_args.push(Operand::ConstI32(0));
                                }
                            }
                            Operand::Call(synth_fn, all_args)
                        } else {
                            let callee_op = self.lower_expr(callee);
                            Operand::CallIndirect {
                                callee: Box::new(callee_op),
                                args: mir_args,
                            }
                        }
                    } else {
                        Operand::Call(name.clone(), mir_args)
                    }
                } else if let ast::Expr::QualifiedIdent { module, name, .. } = callee.as_ref() {
                    // Qualified enum variant constructor: Shape::Circle(5.0)
                    let key = format!("{}::{}", module, name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let payload: Vec<Operand> =
                            args.iter().map(|a| self.lower_expr(a)).collect();
                        return Operand::EnumInit {
                            enum_name: module.clone(),
                            variant: name.clone(),
                            tag,
                            payload,
                        };
                    }
                    // Qualified module function call: module::func(args)
                    // Loaded modules are flattened into the merged module, so codegen resolves by item name.
                    let mir_args: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                    Operand::Call(name.clone(), mir_args)
                } else if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    // Method call: x.method(args) → TypeName__method(x, args)
                    if let Some((mangled, _struct_name)) =
                        self.method_resolutions.get(&span.start).cloned()
                    {
                        let self_arg = self.lower_expr(object);
                        let mut all_args = vec![self_arg];
                        all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                        Operand::Call(mangled, all_args)
                    } else {
                        // Fallback: try to infer struct type and look up method
                        if let Some(struct_name) = self.infer_struct_type(object) {
                            let mangled = format!("{}__{}", struct_name, field);
                            if self.user_fn_names.contains(&mangled) {
                                let self_arg = self.lower_expr(object);
                                let mut all_args = vec![self_arg];
                                all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                                return Operand::Call(mangled, all_args);
                            }
                        }
                        Operand::Unit
                    }
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let c = self.lower_expr(cond);
                let then_stmts = self.lower_block(then_block);
                let then_tail = then_block.tail_expr.as_ref().map(|e| self.lower_expr(e));
                let else_stmts = else_block
                    .as_ref()
                    .map(|b| self.lower_block(b))
                    .unwrap_or_default();
                let else_tail = else_block
                    .as_ref()
                    .and_then(|b| b.tail_expr.as_ref().map(|e| self.lower_expr(e)));
                Operand::IfExpr {
                    cond: Box::new(c),
                    then_body: then_stmts,
                    then_result: then_tail.map(Box::new),
                    else_body: else_stmts,
                    else_result: else_tail.map(Box::new),
                }
            }
            ast::Expr::Block(block) => {
                if let Some(tail) = &block.tail_expr {
                    self.lower_expr(tail)
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Match {
                scrutinee, arms, ..
            } => {
                let scrut = self.lower_expr(scrutinee);
                self.build_match_if_expr(&scrut, arms, 0)
            }
            ast::Expr::Loop { body, .. } => {
                let result_id = self.declare_local("__loop_result");
                let prev = self.loop_result_local;
                self.loop_result_local = Some(result_id);
                let mut body_stmts = Vec::new();
                for stmt in &body.stmts {
                    self.lower_stmt(stmt, &mut body_stmts);
                }
                if let Some(tail) = &body.tail_expr {
                    self.lower_expr_stmt(tail, &mut body_stmts);
                }
                self.loop_result_local = prev;
                // Emit as a while(true) loop
                let outer = vec![
                    MirStmt::Assign(Place::Local(result_id), Rvalue::Use(Operand::ConstI32(0))),
                    MirStmt::WhileStmt {
                        cond: Operand::ConstBool(true),
                        body: body_stmts,
                    },
                ];
                // Return the stmts as side effects and the result local as value
                // We need a way to emit statements before returning an operand.
                // Use the Block operand approach: lower stmts as a sequence, then return the local
                Operand::LoopExpr {
                    init: Box::new(Operand::ConstI32(0)),
                    body: outer,
                    result: Box::new(Operand::Place(Place::Local(result_id))),
                }
            }
            ast::Expr::QualifiedIdent { module, name, .. } => {
                // Enum variant reference: Direction::South -> EnumInit with no payload
                let key = format!("{}::{}", module, name);
                if let Some(&tag) = self.enum_tags.get(&key) {
                    // Check if this variant has fields
                    let has_fields = self
                        .enum_variants
                        .get(module.as_str())
                        .and_then(|vs| vs.iter().find(|(vn, _)| vn == name))
                        .is_some_and(|(_, fc)| *fc > 0);
                    if has_fields {
                        // Variant with payload but called without args — shouldn't happen
                        Operand::ConstI32(tag)
                    } else {
                        // Unit variant — allocate in memory like other enum variants for consistency
                        Operand::EnumInit {
                            enum_name: module.clone(),
                            variant: name.clone(),
                            tag,
                            payload: vec![],
                        }
                    }
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Tuple { elements, .. } => {
                // Lower tuple as a struct with numbered fields
                let tuple_name = format!("__tuple{}", elements.len());
                let lowered_fields: Vec<(String, Operand)> = elements
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (i.to_string(), self.lower_expr(e)))
                    .collect();
                Operand::StructInit {
                    name: tuple_name,
                    fields: lowered_fields,
                }
            }
            ast::Expr::StructInit {
                name, fields, base, ..
            } => {
                // Check if this is an enum struct variant: "EnumName::VariantName"
                if let Some((enum_name, variant_name)) = name.split_once("::") {
                    let key = format!("{}::{}", enum_name, variant_name);
                    if let Some(&tag) = self.enum_tags.get(&key) {
                        let lowered: HashMap<String, Operand> = fields
                            .iter()
                            .map(|(fname, fexpr)| (fname.clone(), self.lower_expr(fexpr)))
                            .collect();
                        // Order payload fields by definition order
                        let def_field_names = self
                            .enum_variant_field_names
                            .get(&key)
                            .cloned()
                            .unwrap_or_default();
                        let payload: Vec<Operand> = def_field_names
                            .iter()
                            .map(|fname| {
                                lowered.get(fname).cloned().unwrap_or(Operand::ConstI32(0))
                            })
                            .collect();
                        return Operand::EnumInit {
                            enum_name: enum_name.to_string(),
                            variant: variant_name.to_string(),
                            tag,
                            payload,
                        };
                    }
                }
                let mut lowered_fields: Vec<(String, Operand)> = fields
                    .iter()
                    .map(|(fname, fexpr)| (fname.clone(), self.lower_expr(fexpr)))
                    .collect();
                // Handle struct field update: fill missing fields from base
                if let Some(base_expr) = base {
                    let base_op = self.lower_expr(base_expr);
                    if let Some(sdef) = self.struct_defs.get(name.as_str()).cloned() {
                        let explicit: std::collections::HashSet<String> =
                            lowered_fields.iter().map(|(n, _)| n.clone()).collect();
                        for (fname, _) in &sdef {
                            if !explicit.contains(fname) {
                                lowered_fields.push((
                                    fname.clone(),
                                    Operand::FieldAccess {
                                        object: Box::new(base_op.clone()),
                                        struct_name: name.clone(),
                                        field: fname.clone(),
                                    },
                                ));
                            }
                        }
                    }
                }
                Operand::StructInit {
                    name: name.clone(),
                    fields: lowered_fields,
                }
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                // Try to determine the struct type from the object
                let struct_name = self.infer_struct_type(object);
                let obj = self.lower_expr(object);
                Operand::FieldAccess {
                    object: Box::new(obj),
                    struct_name: struct_name.unwrap_or_default(),
                    field: field.clone(),
                }
            }
            ast::Expr::Try { expr, span } => {
                let inner = self.lower_expr(expr);
                // Check if the typechecker recorded a From conversion for this ?
                let from_fn = self
                    .method_resolutions
                    .get(&span.start)
                    .map(|(f, _)| f.clone());
                Operand::TryExpr {
                    expr: Box::new(inner),
                    from_fn,
                }
            }
            ast::Expr::Closure { params, body, return_type, .. } => {
                // Lambda-lift: create a synthetic function
                let synth_name = format!("__closure_{}", self.closure_counter);
                self.closure_counter += 1;

                // Identify free variables (captured from enclosing scope)
                let param_names: HashSet<&str> = params.iter().map(|p| p.name.as_str()).collect();
                let mut captures: Vec<String> = Vec::new();
                self.collect_free_vars(body, &param_names, &mut captures);
                captures.dedup();

                // Build params for the synthetic function: closure params first, then captures
                let mut mir_params: Vec<MirLocal> = Vec::new();
                let mut param_idx = 0u32;
                for p in params {
                    let ty = match &p.ty {
                        Some(te) if is_string_type(te) => ark_typecheck::types::Type::String,
                        Some(ast::TypeExpr::Named { name, .. }) if name == "i64" => {
                            ark_typecheck::types::Type::I64
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "f64" => {
                            ark_typecheck::types::Type::F64
                        }
                        Some(ast::TypeExpr::Named { name, .. }) if name == "bool" => {
                            ark_typecheck::types::Type::Bool
                        }
                        _ => ark_typecheck::types::Type::I32,
                    };
                    mir_params.push(MirLocal {
                        id: LocalId(param_idx),
                        name: Some(p.name.clone()),
                        ty,
                    });
                    param_idx += 1;
                }
                for cap in &captures {
                    let ty = if let Some(lid) = self.lookup_local(cap) {
                        if self.string_locals.contains(&lid.0) {
                            ark_typecheck::types::Type::String
                        } else if self.f64_locals.contains(&lid.0) {
                            ark_typecheck::types::Type::F64
                        } else if self.i64_locals.contains(&lid.0) {
                            ark_typecheck::types::Type::I64
                        } else {
                            ark_typecheck::types::Type::I32
                        }
                    } else {
                        ark_typecheck::types::Type::I32
                    };
                    mir_params.push(MirLocal {
                        id: LocalId(param_idx),
                        name: Some(cap.clone()),
                        ty,
                    });
                    param_idx += 1;
                }

                // Lower closure body in a fresh sub-context
                let mut sub_ctx = LowerCtx::new(
                    self.enum_tags.clone(),
                    self.struct_defs.clone(),
                    self.enum_variants.clone(),
                    self.variant_to_enum.clone(),
                    self.bare_variant_tags.clone(),
                    self.enum_defs.clone(),
                    self.enum_variant_field_names.clone(),
                    self.fn_return_types.clone(),
                    self.user_fn_names.clone(),
                    self.method_resolutions.clone(),
                );
                for p in &mir_params {
                    let lid = sub_ctx.declare_local(p.name.as_deref().unwrap_or("_"));
                    match &p.ty {
                        ark_typecheck::types::Type::String => {
                            sub_ctx.string_locals.insert(lid.0);
                        }
                        ark_typecheck::types::Type::F64 => {
                            sub_ctx.f64_locals.insert(lid.0);
                        }
                        ark_typecheck::types::Type::I64 => {
                            sub_ctx.i64_locals.insert(lid.0);
                        }
                        ark_typecheck::types::Type::Bool => {
                            sub_ctx.bool_locals.insert(lid.0);
                        }
                        _ => {
                            // Propagate struct type info for captured variables
                            if let Some(pname) = &p.name {
                                if let Some(parent_lid) = self.lookup_local(pname) {
                                    if let Some(sname) = self.struct_typed_locals.get(&parent_lid.0)
                                    {
                                        sub_ctx.struct_typed_locals.insert(lid.0, sname.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                // Lower body
                let (body_stmts, tail_op) = match body.as_ref() {
                    ast::Expr::Block(block) => {
                        let stmts = sub_ctx.lower_block(block);
                        let tail = block.tail_expr.as_ref().map(|e| sub_ctx.lower_expr(e));
                        (stmts, tail)
                    }
                    other => {
                        let op = sub_ctx.lower_expr(other);
                        (vec![], Some(op))
                    }
                };

                let return_ty = if let Some(rt) = return_type {
                    if is_string_type(rt) {
                        ark_typecheck::types::Type::String
                    } else {
                        match rt {
                            ast::TypeExpr::Named { name, .. } if name == "i64" => ark_typecheck::types::Type::I64,
                            ast::TypeExpr::Named { name, .. } if name == "f64" => ark_typecheck::types::Type::F64,
                            ast::TypeExpr::Named { name, .. } if name == "bool" => ark_typecheck::types::Type::Bool,
                            _ => ark_typecheck::types::Type::I32,
                        }
                    }
                } else if let Some(ref op) = tail_op {
                    if sub_ctx.is_string_operand_mir(op) {
                        ark_typecheck::types::Type::String
                    } else if sub_ctx.is_f64_operand_mir(op) {
                        ark_typecheck::types::Type::F64
                    } else if sub_ctx.is_i64_operand_mir(op) {
                        ark_typecheck::types::Type::I64
                    } else {
                        ark_typecheck::types::Type::I32
                    }
                } else {
                    ark_typecheck::types::Type::I32
                };
                let num_locals = sub_ctx.next_local;
                let entry = BlockId(0);
                let locals: Vec<MirLocal> = (0..num_locals)
                    .map(|i| {
                        let ty = if sub_ctx.string_locals.contains(&i) {
                            ark_typecheck::types::Type::String
                        } else if sub_ctx.f64_locals.contains(&i) {
                            ark_typecheck::types::Type::F64
                        } else if sub_ctx.i64_locals.contains(&i) {
                            ark_typecheck::types::Type::I64
                        } else {
                            ark_typecheck::types::Type::I32
                        };
                        MirLocal {
                            id: LocalId(i),
                            name: Some(format!("_l{}", i)),
                            ty,
                        }
                    })
                    .collect();
                let mir_fn = fallback_function(
                    FnId(0), // will be reassigned in lower_module
                    synth_name.clone(),
                    mir_params,
                    return_ty,
                    locals,
                    vec![fallback_block(
                        entry,
                        body_stmts,
                        if let Some(op) = tail_op {
                            Terminator::Return(Some(op))
                        } else {
                            Terminator::Return(None)
                        },
                    )],
                    entry,
                    sub_ctx.struct_typed_locals.clone(),
                    sub_ctx.enum_typed_locals.clone(),
                );

                let mir_fn = MirFunction {
                    id: infer_fn_id(&synth_name, self.closure_counter),
                    ..mir_fn
                };

                let mir_fn = MirFunction {
                    instance: default_function_instance(&synth_name),
                    ..mir_fn
                };

                let mut mir_fn = mir_fn;
                finalize_function_metadata(&mut mir_fn);
                self.pending_closures.push(mir_fn);
                self.user_fn_names.insert(synth_name.clone());

                // Store captures for call-site injection
                if !captures.is_empty() {
                    self.closure_fn_captures
                        .insert(synth_name.clone(), captures);
                }

                Operand::FnRef(synth_name)
            }
            ast::Expr::Array { elements, .. } => {
                let lowered: Vec<Operand> = elements.iter().map(|e| self.lower_expr(e)).collect();
                Operand::ArrayInit { elements: lowered }
            }
            ast::Expr::ArrayRepeat { value, count, .. } => {
                let val = self.lower_expr(value);
                if let ast::Expr::IntLit { value: n, .. } = count.as_ref() {
                    let n = *n as usize;
                    let elements: Vec<Operand> = (0..n).map(|_| val.clone()).collect();
                    Operand::ArrayInit { elements }
                } else {
                    Operand::Unit
                }
            }
            ast::Expr::Index { object, index, .. } => {
                let obj = self.lower_expr(object);
                let idx = self.lower_expr(index);
                Operand::IndexAccess {
                    object: Box::new(obj),
                    index: Box::new(idx),
                }
            }
            other => {
                eprintln!(
                    "ICE: unhandled expression in lower_expr: {:?}",
                    std::mem::discriminant(other)
                );
                Operand::Unit
            }
        }
    }
}

fn lower_binop(op: &ast::BinOp) -> BinOp {
    match op {
        ast::BinOp::Add => BinOp::Add,
        ast::BinOp::Sub => BinOp::Sub,
        ast::BinOp::Mul => BinOp::Mul,
        ast::BinOp::Div => BinOp::Div,
        ast::BinOp::Mod => BinOp::Mod,
        ast::BinOp::Eq => BinOp::Eq,
        ast::BinOp::Ne => BinOp::Ne,
        ast::BinOp::Lt => BinOp::Lt,
        ast::BinOp::Gt => BinOp::Gt,
        ast::BinOp::Le => BinOp::Le,
        ast::BinOp::Ge => BinOp::Ge,
        ast::BinOp::And => BinOp::And,
        ast::BinOp::Or => BinOp::Or,
        ast::BinOp::BitAnd => BinOp::BitAnd,
        ast::BinOp::BitOr => BinOp::BitOr,
        ast::BinOp::BitXor => BinOp::BitXor,
        ast::BinOp::Shl => BinOp::Shl,
        ast::BinOp::Shr => BinOp::Shr,
    }
}

fn lower_unaryop(op: &ast::UnaryOp) -> UnaryOp {
    match op {
        ast::UnaryOp::Neg => UnaryOp::Neg,
        ast::UnaryOp::Not => UnaryOp::Not,
        ast::UnaryOp::BitNot => UnaryOp::BitNot,
    }
}

/// Check if an expression is void (should be emitted as statement, not value).
fn is_void_expr(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Call { callee, .. } => {
            if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                matches!(name.as_str(), "println" | "print" | "eprintln")
            } else {
                false
            }
        }
        ast::Expr::Assign { .. } => true,
        ast::Expr::Block(block) => match &block.tail_expr {
            None => true,
            Some(tail) => is_void_expr(tail),
        },
        ast::Expr::If { then_block, .. } => match &then_block.tail_expr {
            None => true,
            Some(tail) => is_void_expr(tail),
        },
        ast::Expr::Match { arms, .. } => {
            // Match is void if its first arm body is void
            arms.first().is_none_or(|arm| is_void_expr(&arm.body))
        }
        _ => false,
    }
}

fn is_string_type(ty: &ast::TypeExpr) -> bool {
    matches!(ty, ast::TypeExpr::Named { name, .. } if name == "String")
}

fn type_expr_name(ty: &ast::TypeExpr) -> String {
    match ty {
        ast::TypeExpr::Named { name, .. } => name.clone(),
        ast::TypeExpr::Generic { name, .. } => name.clone(),
        ast::TypeExpr::Unit(_) => "()".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Detect specialized Result enum name for concrete i64/f64 payloads.
/// Returns Some("Result_i64_String") for Result<i64, String>, etc.
fn detect_specialized_result(type_expr: &ast::TypeExpr) -> Option<String> {
    if let ast::TypeExpr::Generic { name, args, .. } = type_expr {
        if name == "Result" {
            if let Some(ast::TypeExpr::Named { name: ok_name, .. }) = args.first() {
                if ok_name == "i64" {
                    return Some("Result_i64_String".to_string());
                }
                if ok_name == "f64" {
                    return Some("Result_f64_String".to_string());
                }
                if ok_name == "String" {
                    return Some("Result_String_String".to_string());
                }
            }
        }
    }
    None
}
