use std::path::{Path, PathBuf};

use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics};
use ark_hir::Program;
use ark_lexer::Lexer;
use ark_mir::{
    MirModule, MirProvenance, compare_lowering_paths, lower_check_output_to_mir, lower_legacy_only,
    module_snapshot, optimization_pass_catalog, optimize_module, optimize_module_named,
    runtime_entry_name, set_mir_provenance,
    validate_backend_legal_module, validate_module,
};
use ark_parser::{ast, parse};
#[allow(deprecated)]
use ark_resolve::resolved_program_to_module;
use ark_resolve::{ResolvedModule, ResolvedProgram};
use ark_target::{EmitKind, TargetId, build_backend_plan};
use ark_typecheck::{CheckOutput, TypeChecker};

use crate::pipeline::{
    AnalyzeArtifact, ArtifactStore, BoundArtifact, CoreHirArtifact, LoadArtifact, PhaseKey,
    PipelineConfig, ResolveArtifact,
};

pub struct FrontendResult {
    pub resolved: ResolvedModule,
    pub checker: TypeChecker,
    pub core_hir: Program,
    pub legacy_mir: MirModule,
    pub corehir_mir: MirModule,
    /// Diagnostics (warnings) collected during frontend that should be printed on success.
    pub pending_diagnostics: Vec<ark_diagnostics::Diagnostic>,
}

pub struct AnalysisResult {
    pub resolved: ResolvedModule,
    pub checker: TypeChecker,
    pub core_hir: Program,
    pub mir: MirModule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirSelection {
    Legacy,
    CoreHir,
    OptimizedLegacy,
    OptimizedCoreHir,
}

impl MirSelection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::CoreHir => "corehir",
            Self::OptimizedLegacy => "optimized-legacy",
            Self::OptimizedCoreHir => "optimized-corehir",
        }
    }
}

pub struct MirComparison {
    pub legacy: MirModule,
    pub corehir: MirModule,
    pub diff: String,
}

pub struct CompiledModule {
    pub mir: MirModule,
    pub wasm: Vec<u8>,
    pub selection: MirSelection,
}

pub struct RuntimeParityReport {
    pub selection: MirSelection,
    pub entry: Option<String>,
    pub snapshot: String,
}

/// Compilation timing report for `--time`.
#[derive(Debug, Clone, Default)]
pub struct CompileTiming {
    pub lex_ms: f64,
    pub parse_ms: f64,
    pub resolve_ms: f64,
    pub typecheck_ms: f64,
    pub lower_ms: f64,
    pub opt_ms: f64,
    pub emit_ms: f64,
    pub total_ms: f64,
    pub opt_detail: String,
}

impl std::fmt::Display for CompileTiming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[arukellt] lex:       {:>7.1}ms", self.lex_ms)?;
        writeln!(f, "[arukellt] parse:     {:>7.1}ms", self.parse_ms)?;
        writeln!(f, "[arukellt] resolve:   {:>7.1}ms", self.resolve_ms)?;
        writeln!(f, "[arukellt] typecheck: {:>7.1}ms", self.typecheck_ms)?;
        writeln!(f, "[arukellt] lower:     {:>7.1}ms", self.lower_ms)?;
        if !self.opt_detail.is_empty() {
            writeln!(
                f,
                "[arukellt] opt:       {:>7.1}ms  ({})",
                self.opt_ms, self.opt_detail
            )?;
        } else {
            writeln!(f, "[arukellt] opt:       {:>7.1}ms", self.opt_ms)?;
        }
        writeln!(f, "[arukellt] emit:      {:>7.1}ms", self.emit_ms)?;
        write!(f, "[arukellt] total:     {:>7.1}ms", self.total_ms)
    }
}

/// Optimization level controlling which MIR passes are enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    /// No optimizations (debug build).
    O0,
    /// Safe optimizations only (default).
    O1,
    /// All optimizations including aggressive passes.
    O2,
}

impl OptLevel {
    pub fn from_u8(level: u8) -> Result<Self, String> {
        match level {
            0 => Ok(Self::O0),
            1 => Ok(Self::O1),
            2 => Ok(Self::O2),
            _ => Err(format!("invalid opt-level: {} (expected 0, 1, or 2)", level)),
        }
    }
}

pub struct Session {
    source_map: SourceMap,
    sink: DiagnosticSink,
    config: PipelineConfig,
    artifacts: ArtifactStore,
    pub timing_enabled: bool,
    pub last_timing: Option<CompileTiming>,
    pub opt_level: OptLevel,
    pub disabled_passes: Vec<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

fn render_mir_validation_errors(errors: Vec<ark_mir::MirValidationError>) -> String {
    errors
        .into_iter()
        .map(|error| match error.block {
            Some(block) => format!(
                "internal error: MIR {} block {}: {}",
                error.function, block.0, error.message
            ),
            None => format!("internal error: MIR {}: {}", error.function, error.message),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_mir(module: &MirModule) -> Result<(), String> {
    validate_module(module).map_err(render_mir_validation_errors)
}

fn validate_backend_ready_mir(module: &MirModule) -> Result<(), String> {
    validate_backend_legal_module(module).map_err(render_mir_validation_errors)
}

fn mark_selection(module: &mut MirModule, selection: MirSelection) {
    match selection {
        MirSelection::Legacy => set_mir_provenance(module, MirProvenance::LegacyAst),
        MirSelection::CoreHir => set_mir_provenance(module, MirProvenance::CoreHir),
        MirSelection::OptimizedLegacy => set_mir_provenance(module, MirProvenance::LegacyOptimized),
        MirSelection::OptimizedCoreHir => {
            set_mir_provenance(module, MirProvenance::CoreHirOptimized)
        }
    }
}

fn ensure_runtime_entry(module: &MirModule, selection: MirSelection) -> Result<(), String> {
    if runtime_entry_name(module).is_none() {
        Err(format!(
            "internal error: MIR selection `{}` does not expose a runtime entrypoint",
            selection.as_str()
        ))
    } else {
        Ok(())
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            source_map: SourceMap::new(),
            sink: DiagnosticSink::new(),
            config: PipelineConfig::default(),
            artifacts: ArtifactStore::default(),
            timing_enabled: false,
            last_timing: None,
            opt_level: OptLevel::O1,
            disabled_passes: Vec::new(),
        }
    }

    pub fn add_file(&mut self, name: String, source: String) -> u32 {
        self.source_map.add_file(name, source)
    }

    fn load_source(&mut self, path: &Path) -> Result<(String, PhaseKey, u32), String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("error: {}: {}", path.display(), e))?;
        let key = PhaseKey::for_path(path, &source, &self.config);
        self.artifacts.remember_key(path.to_path_buf(), key.clone());
        let file_id = self
            .source_map
            .add_file(path.display().to_string(), source.clone());
        Ok((source, key, file_id))
    }

    pub fn parse(&mut self, path: &Path) -> Result<ast::Module, String> {
        let (source, key, file_id) = self.load_source(path)?;
        if let Some(module) = self.artifacts.parse.get(&key) {
            return Ok(module.clone());
        }

        self.sink = DiagnosticSink::new();
        let lexer = Lexer::new(file_id, &source);
        let tokens: Vec<_> = lexer.collect();
        let module = parse(&tokens, &mut self.sink);
        if self.sink.has_errors() {
            return Err(render_diagnostics(
                self.sink.diagnostics(),
                &self.source_map,
            ));
        }

        self.artifacts.parse.insert(key, module.clone());
        Ok(module)
    }

    pub fn bind(&mut self, path: &Path) -> Result<BoundArtifact, String> {
        let (_, key, _) = self.load_source(path)?;
        if let Some(bound) = self.artifacts.bind.get(&key) {
            return Ok(bound.clone());
        }

        let module = self.parse(path)?;
        let bound = BoundArtifact { module };
        self.artifacts.bind.insert(key, bound.clone());
        Ok(bound)
    }

    pub fn load_graph(&mut self, path: &Path) -> Result<LoadArtifact, String> {
        let (_, key, _) = self.load_source(path)?;
        if let Some(loaded) = self.artifacts.load.get(&key) {
            return Ok(loaded.clone());
        }

        self.sink = DiagnosticSink::new();
        let program = ark_resolve::resolve_program(path, &mut self.sink).ok();
        if self.sink.has_errors() {
            return Err(render_diagnostics(
                self.sink.diagnostics(),
                &self.source_map,
            ));
        }

        let artifact = LoadArtifact { program };
        self.artifacts.load.insert(key, artifact.clone());
        Ok(artifact)
    }

    pub fn analyze(&mut self, path: &Path) -> Result<AnalyzeArtifact, String> {
        let (_, key, _) = self.load_source(path)?;
        if let Some(analyzed) = self.artifacts.analyze.get(&key) {
            return Ok(analyzed.clone());
        }

        let mut loaded = self.load_graph(path)?.program;
        if let Some(program) = loaded.as_mut() {
            self.sink = DiagnosticSink::new();
            ark_resolve::merge_prelude(program, &mut self.sink);
            if self.sink.has_errors() {
                return Err(render_diagnostics(
                    self.sink.diagnostics(),
                    &self.source_map,
                ));
            }
        }

        let artifact = AnalyzeArtifact { program: loaded };
        self.artifacts.analyze.insert(key, artifact.clone());
        Ok(artifact)
    }

    pub fn resolve(&mut self, path: &Path) -> Result<ResolveArtifact, String> {
        let (_, key, _) = self.load_source(path)?;
        if let Some(resolved) = self.artifacts.resolve.get(&key) {
            return Ok(resolved.clone());
        }

        let resolved = if let Some(program) = self.analyze(path)?.program {
            #[allow(deprecated)]
            ResolveArtifact {
                resolved: ResolvedModule {
                    module: resolved_program_to_module(&program),
                    symbols: program.symbols,
                    global_scope: program.global_scope,
                },
            }
        } else {
            let module = self.bind(path)?.module;
            self.sink = DiagnosticSink::new();
            let resolved = ark_resolve::resolve_module(module, &mut self.sink);
            if self.sink.has_errors() {
                return Err(render_diagnostics(
                    self.sink.diagnostics(),
                    &self.source_map,
                ));
            }
            ResolveArtifact { resolved }
        };

        self.artifacts.resolve.insert(key, resolved.clone());
        Ok(resolved)
    }

    pub fn check_core_hir(&mut self, path: &Path) -> Result<CoreHirArtifact, String> {
        let (_, key, _) = self.load_source(path)?;
        if let Some(core_hir) = self.artifacts.core_hir.get(&key) {
            return Ok(core_hir.clone());
        }

        let resolved = self.resolve(path)?.resolved;
        self.sink = DiagnosticSink::new();
        let mut checker = TypeChecker::new();
        checker.register_builtins();
        let output = checker.check_core_hir_module(&resolved, &mut self.sink);
        if self.sink.has_errors() {
            return Err(render_diagnostics(
                self.sink.diagnostics(),
                &self.source_map,
            ));
        }

        let artifact = CoreHirArtifact { output };
        self.artifacts.core_hir.insert(key, artifact.clone());
        Ok(artifact)
    }

    fn run_frontend(&mut self, path: &Path) -> Result<FrontendResult, String> {
        let t_total = std::time::Instant::now();
        let resolved = self.resolve(path)?.resolved;
        self.sink = DiagnosticSink::new();

        let t_tc = std::time::Instant::now();
        let mut checker = TypeChecker::new();
        checker.register_builtins();
        // Run CoreHIR typecheck into an isolated sink so that E0200 CoreHIR
        // structural validation failures don't abort the legacy lowering path.
        let mut corehir_sink = DiagnosticSink::new();
        let core_hir: CheckOutput = checker.check_core_hir_module(&resolved, &mut corehir_sink);
        let corehir_valid = !corehir_sink.has_errors();
        // Promote all errors to the main sink, except CoreHIR structural validation
        // failures (E0200 "invalid CoreHIR: ..."), which should not block the legacy path.
        for diag in corehir_sink.diagnostics() {
            let is_corehir_structural = diag.code == ark_diagnostics::DiagnosticCode::E0200
                && diag.message.starts_with("invalid CoreHIR:");
            if !is_corehir_structural {
                self.sink.emit(diag.clone());
            }
        }
        if self.sink.has_errors() {
            return Err(render_diagnostics(
                self.sink.diagnostics(),
                &self.source_map,
            ));
        }
        let typecheck_ms = t_tc.elapsed().as_secs_f64() * 1000.0;

        let t_lower = std::time::Instant::now();
        let mut legacy_mir = lower_legacy_only(&resolved.module, &checker, &mut self.sink);
        mark_selection(&mut legacy_mir, MirSelection::Legacy);
        validate_mir(&legacy_mir)?;

        let corehir_mir = if corehir_valid {
            let mut mir =
                lower_check_output_to_mir(&resolved.module, &core_hir, &checker, &mut self.sink)
                    .unwrap_or_else(|_| legacy_mir.clone());
            mark_selection(&mut mir, MirSelection::CoreHir);
            if validate_mir(&mir).is_err() {
                let mut fallback = legacy_mir.clone();
                mark_selection(&mut fallback, MirSelection::CoreHir);
                fallback
            } else {
                mir
            }
        } else {
            let mut fallback = legacy_mir.clone();
            mark_selection(&mut fallback, MirSelection::CoreHir);
            fallback
        };
        let lower_ms = t_lower.elapsed().as_secs_f64() * 1000.0;

        // Record timing for resolve/typecheck/lower phases if enabled.
        // Lex/parse timing is captured from the pipeline cache overhead in resolve.
        if self.timing_enabled {
            let frontend_ms = t_total.elapsed().as_secs_f64() * 1000.0;
            let resolve_ms = frontend_ms - typecheck_ms - lower_ms;
            self.last_timing = Some(CompileTiming {
                resolve_ms,
                typecheck_ms,
                lower_ms,
                ..CompileTiming::default()
            });
        }

        // Collect warnings from frontend to pass downstream (they survive the sink reset in compile_selected).
        let pending_diagnostics: Vec<ark_diagnostics::Diagnostic> = self
            .sink
            .diagnostics()
            .iter()
            .filter(|d| d.severity() == ark_diagnostics::Severity::Warning)
            .cloned()
            .collect();

        Ok(FrontendResult {
            resolved,
            checker,
            core_hir: core_hir.program().clone(),
            legacy_mir,
            corehir_mir,
            pending_diagnostics,
        })
    }

    pub fn check(&mut self, path: &Path) -> Result<(), String> {
        self.check_core_hir(path).map(|_| ())
    }

    pub fn compare_mir_paths(&mut self, path: &Path) -> Result<MirComparison, String> {
        let frontend = self.run_frontend(path)?;
        Ok(MirComparison {
            diff: compare_lowering_paths(&frontend.legacy_mir, &frontend.corehir_mir),
            legacy: frontend.legacy_mir,
            corehir: frontend.corehir_mir,
        })
    }

    pub fn lower_mir(&mut self, path: &Path) -> Result<MirModule, String> {
        self.lower_mir_selected(path, MirSelection::CoreHir)
    }

    pub fn lower_mir_selected(
        &mut self,
        path: &Path,
        selection: MirSelection,
    ) -> Result<MirModule, String> {
        let frontend = self.run_frontend(path)?;
        let mut mir = match selection {
            MirSelection::Legacy | MirSelection::OptimizedLegacy => frontend.legacy_mir,
            MirSelection::CoreHir | MirSelection::OptimizedCoreHir => frontend.corehir_mir,
        };
        if matches!(
            selection,
            MirSelection::OptimizedLegacy | MirSelection::OptimizedCoreHir
        ) {
            optimize_module(&mut mir)
                .map_err(|message| format!("internal error: optimizer failed: {message}"))?;
            mark_selection(&mut mir, selection);
        }
        validate_mir(&mir)?;
        Ok(mir)
    }

    pub fn runtime_parity_report(
        &mut self,
        path: &Path,
        selection: MirSelection,
    ) -> Result<RuntimeParityReport, String> {
        let mir = self.lower_mir_selected(path, selection)?;
        Ok(RuntimeParityReport {
            selection,
            entry: runtime_entry_name(&mir),
            snapshot: module_snapshot(&mir),
        })
    }

    pub fn compile(&mut self, path: &Path, target: TargetId) -> Result<Vec<u8>, String> {
        self.compile_selected(path, target, MirSelection::Legacy)
            .map(|compiled| compiled.wasm)
    }

    pub fn compile_selected(
        &mut self,
        path: &Path,
        target: TargetId,
        selection: MirSelection,
    ) -> Result<CompiledModule, String> {
        let t_total = std::time::Instant::now();
        if target == TargetId::Native {
            return Err("error: native target uses the dedicated LLVM compile path".to_string());
        }

        let frontend = self.run_frontend(path)?;
        let pending_diagnostics = frontend.pending_diagnostics.clone();
        let mut mir = match selection {
            MirSelection::Legacy | MirSelection::OptimizedLegacy => frontend.legacy_mir,
            MirSelection::CoreHir | MirSelection::OptimizedCoreHir => frontend.corehir_mir,
        };
        let t_opt = std::time::Instant::now();
        let opt_detail = if matches!(
            selection,
            MirSelection::OptimizedLegacy | MirSelection::OptimizedCoreHir
        ) && self.opt_level != OptLevel::O0
        {
            let summary = if self.opt_level == OptLevel::O1 {
                let o1_passes: &[&str] = &[
                    "const_fold",
                    "branch_fold",
                    "cfg_simplify",
                    "copy_prop",
                    "const_prop",
                    "dead_local_elim",
                    "dead_block_elim",
                    "unreachable_cleanup",
                ];
                let passes: Vec<&str> = o1_passes
                    .iter()
                    .filter(|p| !self.disabled_passes.iter().any(|d| d == *p))
                    .copied()
                    .collect();
                optimize_module_named(&mut mir, &passes)
            } else {
                if self.disabled_passes.is_empty() {
                    optimize_module(&mut mir)
                } else {
                    let all_passes = optimization_pass_catalog();
                    let passes: Vec<&str> = all_passes
                        .iter()
                        .filter(|p| !self.disabled_passes.iter().any(|d| d == *p))
                        .copied()
                        .collect();
                    optimize_module_named(&mut mir, &passes)
                }
            };
            let summary = summary
                .map_err(|message| format!("internal error: optimizer failed: {message}"))?;
            mark_selection(&mut mir, selection);
            format!(
                "rounds={}, const_fold={}, dce={}",
                summary.rounds, summary.const_folded, summary.dead_locals_removed
            )
        } else {
            String::new()
        };
        let opt_ms = t_opt.elapsed().as_secs_f64() * 1000.0;

        validate_mir(&mir)?;
        ensure_runtime_entry(&mir, selection)?;
        // The legacy T1 backend handles high-level IR nodes (IfExpr, LoopExpr, TryExpr)
        // directly, so backend-legal validation only applies to the CoreHIR path.
        if matches!(
            selection,
            MirSelection::CoreHir | MirSelection::OptimizedCoreHir
        ) {
            validate_backend_ready_mir(&mir)?;
        }
        let plan = build_backend_plan(target, EmitKind::CoreWasm)?;

        self.sink = DiagnosticSink::new();
        // Re-emit frontend warnings into the current sink so they appear on success.
        for diag in pending_diagnostics {
            self.sink.emit(diag);
        }
        let t_emit = std::time::Instant::now();
        let wasm = ark_wasm::emit_with_plan(&mir, &mut self.sink, &plan);
        let emit_ms = t_emit.elapsed().as_secs_f64() * 1000.0;

        if self.sink.has_errors() {
            return Err(render_diagnostics(
                self.sink.diagnostics(),
                &self.source_map,
            ));
        }
        if self.sink.has_warnings() {
            eprint!(
                "{}",
                render_diagnostics(self.sink.diagnostics(), &self.source_map)
            );
        }

        // Finalize timing report
        if self.timing_enabled {
            if let Some(ref mut timing) = self.last_timing {
                timing.opt_ms = opt_ms;
                timing.opt_detail = opt_detail;
                timing.emit_ms = emit_ms;
                timing.total_ms = t_total.elapsed().as_secs_f64() * 1000.0;
            }
        }

        mark_selection(&mut mir, selection);
        Ok(CompiledModule {
            mir,
            wasm,
            selection,
        })
    }

    pub fn compile_wit(&mut self, path: &Path) -> Result<String, String> {
        let frontend = self.run_frontend(path)?;
        let world_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
        let world = ark_wasm::component::mir_to_wit_world(&frontend.corehir_mir, world_name)
            .map_err(|e| format!("WIT generation error: {}", e))?;
        ark_wasm::component::generate_wit(&world)
            .map_err(|e| format!("WIT generation error: {}", e))
    }

    /// Compile to a Component Model binary (.component.wasm).
    ///
    /// Pipeline: frontend → MIR → core Wasm → WIT generation → component wrapping.
    pub fn compile_component(&mut self, path: &Path, target: TargetId) -> Result<Vec<u8>, String> {
        if target == TargetId::Native {
            return Err("error: component model requires a Wasm target".to_string());
        }
        let profile = target.profile();
        if !profile.component_model {
            return Err(format!(
                "error: component model requires --target wasm32-wasi-p2 (target `{}` does not support components)",
                target
            ));
        }

        // Step 1: Compile to core Wasm
        let compiled = self.compile_selected(path, target, MirSelection::Legacy)?;

        // Step 2: Generate WIT (with warnings for non-exportable functions)
        let world_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
        let (world, export_warnings) =
            ark_wasm::component::mir_to_wit_world_with_warnings(&compiled.mir, world_name)
                .map_err(|e| format!("WIT generation error: {}", e))?;

        // Emit W0005 warnings for non-exportable functions
        for warning in &export_warnings {
            self.sink
                .emit(ark_diagnostics::non_exportable_function_diagnostic(
                    "", warning,
                ));
        }

        let wit_text = ark_wasm::component::generate_wit(&world)
            .map_err(|e| format!("WIT generation error: {}", e))?;

        // Validate exported function types for canonical ABI compatibility
        // Check 1: WIT-level validation (catches string, list, option, result, etc.)
        let type_errors = ark_wasm::component::validate_component_export_types(&world);
        if !type_errors.is_empty() {
            for (_, diag) in &type_errors {
                self.sink.emit(diag.clone());
            }
            return Err(render_diagnostics(
                self.sink.diagnostics(),
                &self.source_map,
            ));
        }

        // Check 2: Core Wasm binary validation (catches GC ref types in exports
        // that MIR types don't reflect due to monomorphization)
        let wasm_errors = ark_wasm::component::validate_core_wasm_exports(&compiled.wasm);
        if !wasm_errors.is_empty() {
            for (_, diag) in &wasm_errors {
                self.sink.emit(diag.clone());
            }
            return Err(render_diagnostics(
                self.sink.diagnostics(),
                &self.source_map,
            ));
        }

        // Step 3: Wrap into component via wasm-tools
        let component_bytes =
            ark_wasm::component::wrap::wrap_core_to_component(&compiled.wasm, &wit_text)
                .map_err(|e| format!("{}", e))?;

        Ok(component_bytes)
    }

    pub fn analyze_result(&mut self, path: &Path) -> Result<AnalysisResult, String> {
        let frontend = self.run_frontend(path)?;
        Ok(AnalysisResult {
            resolved: frontend.resolved,
            checker: frontend.checker,
            core_hir: frontend.core_hir,
            mir: frontend.corehir_mir,
        })
    }

    pub fn profile_memory(&mut self, path: &Path) -> Result<String, String> {
        let frontend = self.run_frontend(path)?;
        let escape_info = ark_mir::escape::analyze_module(&frontend.corehir_mir);
        Ok(ark_mir::escape::format_escape_info(&escape_info))
    }

    pub fn cache_key_for(&self, path: &Path) -> Option<&PhaseKey> {
        self.artifacts.key_for_path(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_cache_key_changes_with_path() {
        let session = Session::new();
        let key_a = PhaseKey::for_path(Path::new("a.ark"), "fn main() {}", &session.config);
        let key_b = PhaseKey::for_path(Path::new("b.ark"), "fn main() {}", &session.config);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn mir_selection_labels_are_stable() {
        assert_eq!(MirSelection::Legacy.as_str(), "legacy");
        assert_eq!(MirSelection::CoreHir.as_str(), "corehir");
        assert_eq!(MirSelection::OptimizedLegacy.as_str(), "optimized-legacy");
        assert_eq!(MirSelection::OptimizedCoreHir.as_str(), "optimized-corehir");
    }
}

fn _keep_pathbuf(_: &PathBuf, _: Option<ResolvedProgram>) {}
