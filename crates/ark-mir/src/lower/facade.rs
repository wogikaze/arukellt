//! Public lowering entrypoints, CoreHIR/legacy orchestration, path comparison, and debug helpers.

use ark_diagnostics::DiagnosticSink;
use ark_hir::{BodyId, ExprKind, ItemKind, Program, Stmt};
use ark_parser::ast;
use ark_typecheck::{CheckOutput, TypeChecker};

use crate::mir::*;
use crate::validate::validate_module;

#[allow(deprecated)]
use super::func::lower_to_mir;

#[allow(deprecated)]
fn lower_hir_fallback(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = lower_to_mir(module, checker, sink);
    set_mir_provenance(&mut mir, MirProvenance::CoreHirFallback);
    mir
}

#[allow(deprecated)]
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

/// Routes to the legacy AST lowerer. Deprecated — use `lower_check_output_to_mir`.
#[deprecated(since = "0.1.0", note = "Use lower_check_output_to_mir instead.")]
pub fn lower_any_to_mir(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    lower_hir_fallback(module, checker, sink)
}

/// Routes to the legacy AST lowerer as the CoreHIR implementation. Deprecated — use
/// `lower_check_output_to_mir` which will use the real CoreHIR lowerer when available.
#[deprecated(since = "0.1.0", note = "Use lower_check_output_to_mir instead.")]
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

/// Desugar all `Operand::IfExpr` nodes in a single MIR function into
/// `MirStmt::IfStmt` sequences with temporary locals.
///
/// This is the lowering-time counterpart of the optimisation-time desugar
/// pass.  Running it at lowering time means the MIR handed to the backend
/// is already free of `IfExpr` nodes, satisfying `is_backend_legal_module`.
pub fn lower_if_expr(func: &mut MirFunction) {
    let fn_return_types: std::collections::HashMap<String, ark_typecheck::types::Type> =
        std::collections::HashMap::new();
    crate::opt::desugar::desugar_if_exprs(func, &fn_return_types);
}

/// Apply [`lower_if_expr`] to every function in a module.
pub fn lower_if_exprs(module: &mut MirModule) {
    for func in &mut module.functions {
        lower_if_expr(func);
    }
}

/// Desugar all `Operand::LoopExpr` nodes in a single MIR function into
/// `MirStmt::WhileStmt` + temporary-local sequences.
///
/// This is the lowering-time counterpart of the optimisation-time desugar
/// pass.  Running it at lowering time means the MIR handed to the backend
/// is already free of `LoopExpr` nodes, satisfying `is_backend_legal_module`.
pub fn lower_loop_expr(func: &mut MirFunction) {
    let fn_return_types: std::collections::HashMap<String, ark_typecheck::types::Type> =
        std::collections::HashMap::new();
    crate::opt::desugar::desugar_loop_exprs(func, &fn_return_types);
}

/// Apply [`lower_loop_expr`] to every function in a module.
pub fn lower_loop_exprs(module: &mut MirModule) {
    for func in &mut module.functions {
        lower_loop_expr(func);
    }
}

/// Desugar all `Operand::TryExpr` nodes in a single MIR function into
/// match-on-Result + early-return `MirStmt` sequences.
///
/// This is the lowering-time counterpart of the optimisation-time desugar
/// pass.  Running it at lowering time means the MIR handed to the backend
/// is already free of `TryExpr` nodes, satisfying `is_backend_legal_module`.
pub fn lower_try_expr(func: &mut MirFunction) {
    let fn_return_types: std::collections::HashMap<String, ark_typecheck::types::Type> =
        std::collections::HashMap::new();
    crate::opt::desugar::desugar_try_exprs(func, &fn_return_types);
}

/// Apply [`lower_try_expr`] to every function in a module.
pub fn lower_try_exprs(module: &mut MirModule) {
    for func in &mut module.functions {
        lower_try_expr(func);
    }
}

/// Run [`lower_if_exprs`], [`lower_loop_exprs`], and [`lower_try_exprs`] until the module is
/// backend-legal or `max_rounds` is exhausted.
///
/// A single pass is not always enough: `Operand::TryExpr` hides its inner expression from the
/// earlier if/loop passes, so `IfExpr` / `LoopExpr` can remain until after `TryExpr` is expanded
/// (issue #283).
pub(crate) fn lower_backend_illegal_operands(module: &mut MirModule, max_rounds: usize) {
    for _ in 0..max_rounds {
        lower_if_exprs(module);
        lower_loop_exprs(module);
        lower_try_exprs(module);
        if is_backend_legal_module(module) {
            return;
        }
    }
}

pub fn lower_corehir_with_fallback(
    core_hir: &Program,
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> Result<MirModule, String> {
    // NOTE: lower_hir_to_mir is currently a placeholder that returns empty MIR.
    // The fallback to the legacy AST lowerer (lower_hir_fallback) is therefore
    // always taken. This will change once the CoreHIR lowerer is fully implemented.
    // See issues/open/508-legacy-path-removal-unblocked-by.md.
    let mut mir = match lower_hir_to_mir(core_hir, checker, sink) {
        Ok(mir) if !mir.functions.is_empty() => mir,
        #[allow(deprecated)]
        Ok(_) | Err(_) => lower_corehir_via_legacy(module, checker, sink),
    };
    lower_backend_illegal_operands(&mut mir, 16);
    Ok(mir)
}

#[allow(deprecated)]
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

#[deprecated(
    since = "0.1.0",
    note = "Use lower_check_output_to_mir instead. Direct legacy lowering will be removed \
            once the CoreHIR lowerer is fully implemented \
            (see issues/open/508-legacy-path-removal-unblocked-by.md)."
)]
pub fn lower_legacy_only(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    #[allow(deprecated)]
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

#[deprecated(
    since = "0.1.0",
    note = "Use lower_check_output_to_mir or lower_corehir_only instead."
)]
pub fn lower_prefer_legacy(
    module: &ast::Module,
    checker: &TypeChecker,
    sink: &mut DiagnosticSink,
) -> MirModule {
    #[allow(deprecated)]
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
