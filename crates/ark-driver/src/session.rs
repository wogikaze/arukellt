use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics};
use ark_hir::Program;
use ark_lexer::Lexer;
use ark_resolve::inject_wit_externs;
#[allow(deprecated)]
use ark_mir::lower_legacy_only;
use ark_mir::{
    MirModule, MirProvenance, compare_lowering_paths, dump_mir_phase, dump_phases_requested,
    eliminate_dead_functions, lower_check_output_to_mir, module_snapshot,
    optimization_pass_catalog, optimize_module, optimize_module_named, runtime_entry_name,
    set_mir_provenance, validate_backend_legal_module, validate_module,
};
use ark_parser::{ast, parse};
#[allow(deprecated)]
use ark_resolve::resolved_program_entry;
use ark_resolve::{ResolvedModule, ResolvedProgram};
use ark_target::{EmitKind, TargetId, WasiVersion, build_backend_plan};
use ark_typecheck::{CheckOutput, Type, TypeChecker};

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
    /// Deprecated: routes through the legacy AST lowerer (`lower_to_mir`).
    /// Use `CoreHir` / `OptimizedCoreHir` instead. Legacy variants will be
    /// removed once the CoreHIR lowerer (`lower_hir_to_mir`) is fully
    /// implemented (see issues/open/508-legacy-path-removal-unblocked-by.md).
    #[deprecated(since = "0.1.0", note = "use CoreHir or OptimizedCoreHir instead")]
    Legacy,
    CoreHir,
    /// Deprecated: see `Legacy`.
    #[deprecated(since = "0.1.0", note = "use OptimizedCoreHir instead")]
    OptimizedLegacy,
    OptimizedCoreHir,
}

#[allow(deprecated)] // refers to deprecated variants Legacy and OptimizedLegacy
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
#[derive(Debug, Clone, Default, Serialize)]
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
            _ => Err(format!(
                "invalid opt-level: {} (expected 0, 1, or 2)",
                level
            )),
        }
    }
}

pub struct Session {
    source_map: SourceMap,
    sink: DiagnosticSink,
    config: PipelineConfig,
    artifacts: ArtifactStore,
    /// mtime-based cache: path → (mtime, source, key). Lets `load_source` skip
    /// `fs::read_to_string` when the file has not changed since the last compile.
    file_mtime_cache: HashMap<PathBuf, (SystemTime, String, PhaseKey)>,
    pub timing_enabled: bool,
    pub last_timing: Option<CompileTiming>,
    pub opt_level: OptLevel,
    pub disabled_passes: Vec<String>,
    /// When true, component wrapping skips the P1 adapter (~100 KB savings).
    /// Set via `--p2-native` or `--wasi-version p2`.
    pub p2_native: bool,
    /// WASI version selected by `--wasi-version`.  Stored for diagnostics and
    /// future use when the T3 emitter gains a P2 import-table switch
    /// (see `issues/open/510-t3-p2-import-table-switch.md`).
    pub wasi_version: WasiVersion,
    /// When true, the Wasm Name Section is omitted from emission (--strip-debug).
    pub strip_debug: bool,
    /// Lint rules to suppress (allow) — diagnostics with these codes are dropped.
    pub lint_allow: Vec<String>,
    /// Lint rules to escalate to errors (deny).
    pub lint_deny: Vec<String>,
    /// The active compilation target for the current compile invocation.
    /// Set by `compile_with_entry` before the frontend runs so that
    /// `load_graph` can pass it to the module loader for target-gating.
    pub active_target: Option<TargetId>,
    /// WIT files for Component Model host imports.
    /// Each file is parsed and its exported function names are injected into
    /// the resolver before type-check, and its imports populate `MirModule.imports`.
    pub wit_files: Vec<PathBuf>,
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

#[allow(dead_code)] // Reserved for when the CoreHIR lowerer produces its own flat MIR
fn validate_backend_ready_mir(module: &MirModule) -> Result<(), String> {
    validate_backend_legal_module(module).map_err(render_mir_validation_errors)
}

#[allow(deprecated)] // handles legacy variants for backward compat during deprecation period
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

fn checker_type_from_wit_type(ty: &ark_wasm::component::WitType) -> Type {
    match ty {
        ark_wasm::component::WitType::U8 => Type::U8,
        ark_wasm::component::WitType::U16 => Type::U16,
        ark_wasm::component::WitType::U32 => Type::U32,
        ark_wasm::component::WitType::U64 => Type::U64,
        ark_wasm::component::WitType::S8 => Type::I8,
        ark_wasm::component::WitType::S16 => Type::I16,
        ark_wasm::component::WitType::S32 => Type::I32,
        ark_wasm::component::WitType::S64 => Type::I64,
        ark_wasm::component::WitType::F32 => Type::F32,
        ark_wasm::component::WitType::F64 => Type::F64,
        ark_wasm::component::WitType::Bool => Type::Bool,
        ark_wasm::component::WitType::Char => Type::Char,
        ark_wasm::component::WitType::StringType => Type::String,
        ark_wasm::component::WitType::Flags(_) => Type::I32,
        ark_wasm::component::WitType::List(inner) => {
            Type::Vec(Box::new(checker_type_from_wit_type(inner)))
        }
        ark_wasm::component::WitType::Option(inner) => {
            Type::Option(Box::new(checker_type_from_wit_type(inner)))
        }
        ark_wasm::component::WitType::Result { ok, err } => Type::Result(
            Box::new(
                ok.as_deref()
                    .map(checker_type_from_wit_type)
                    .unwrap_or(Type::Unit),
            ),
            Box::new(
                err.as_deref()
                    .map(checker_type_from_wit_type)
                    .unwrap_or(Type::Unit),
            ),
        ),
        ark_wasm::component::WitType::Tuple(items) => {
            Type::Tuple(items.iter().map(checker_type_from_wit_type).collect())
        }
        ark_wasm::component::WitType::Record(_)
        | ark_wasm::component::WitType::Enum(_)
        | ark_wasm::component::WitType::Variant(_)
        | ark_wasm::component::WitType::Resource(_)
        | ark_wasm::component::WitType::Own(_)
        | ark_wasm::component::WitType::Borrow(_) => Type::I32,
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            source_map: SourceMap::new(),
            sink: DiagnosticSink::new(),
            config: PipelineConfig::default(),
            artifacts: ArtifactStore::default(),
            file_mtime_cache: HashMap::new(),
            timing_enabled: false,
            last_timing: None,
            opt_level: OptLevel::O1,
            disabled_passes: Vec::new(),
            p2_native: false,
            wasi_version: WasiVersion::P1,
            strip_debug: false,
            lint_allow: Vec::new(),
            lint_deny: Vec::new(),
            active_target: None,
            wit_files: Vec::new(),
        }
    }

    pub fn add_file(&mut self, name: String, source: String) -> u32 {
        self.source_map.add_file(name, source)
    }

    fn load_source(&mut self, path: &Path) -> Result<(String, PhaseKey, u32), String> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Fast path: if mtime is unchanged, reuse the cached source without re-reading.
        if let Ok(meta) = std::fs::metadata(&canonical)
            && let Ok(mtime) = meta.modified()
            && let Some((cached_mtime, cached_src, cached_key)) =
                self.file_mtime_cache.get(&canonical)
            && *cached_mtime == mtime
        {
            self.artifacts
                .remember_key(canonical.clone(), cached_key.clone());
            let file_id = self
                .source_map
                .add_file(canonical.display().to_string(), cached_src.clone());
            return Ok((cached_src.clone(), cached_key.clone(), file_id));
        }

        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("error: {}: {}", path.display(), e))?;
        let key = PhaseKey::for_path(path, &source, &self.config);
        self.artifacts.remember_key(canonical.clone(), key.clone());

        // Update mtime cache.
        if let Ok(meta) = std::fs::metadata(&canonical)
            && let Ok(mtime) = meta.modified()
        {
            self.file_mtime_cache
                .insert(canonical.clone(), (mtime, source.clone(), key.clone()));
        }

        let file_id = self
            .source_map
            .add_file(path.display().to_string(), source.clone());
        Ok((source, key, file_id))
    }

    /// Evict the incremental cache for `path`. Call this in watch mode when a
    /// file-change event is detected so the next compile re-reads from disk.
    pub fn invalidate_file(&mut self, path: &Path) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.file_mtime_cache.remove(&canonical);
        // Also clear the downstream artifact cache so the whole pipeline re-runs.
        self.artifacts = ArtifactStore::default();
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
        let program =
            ark_resolve::resolve_program_with_target(path, &mut self.sink, self.active_target).ok();
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
            // Check unused imports on the entry module
            ark_resolve::check_unused_imports(&program.entry_module, &mut self.sink);
            ark_resolve::check_unused_bindings(&program.entry_module, &mut self.sink);
            #[allow(deprecated)]
            ResolveArtifact {
                resolved: resolved_program_entry(program),
            }
        } else {
            let module = self.bind(path)?.module;
            self.sink = DiagnosticSink::new();
            let resolved = ark_resolve::resolve_module(module, &mut self.sink);
            // Check unused imports
            ark_resolve::check_unused_imports(&resolved.module, &mut self.sink);
            ark_resolve::check_unused_bindings(&resolved.module, &mut self.sink);
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
        // Preserve warnings from resolution (e.g., unused imports)
        let resolve_warnings: Vec<ark_diagnostics::Diagnostic> = self
            .sink
            .diagnostics()
            .iter()
            .filter(|d| d.severity() == ark_diagnostics::Severity::Warning)
            .cloned()
            .collect();
        self.sink = DiagnosticSink::new();
        for w in resolve_warnings {
            self.sink.emit(w);
        }
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
        self.run_frontend_for(path, None)
    }

    /// Run frontend with optional selection hint to avoid double lowering.
    /// When `hint` is `Some(Legacy)`, only legacy lowering runs [DEPRECATED].
    /// When `hint` is `Some(CoreHir)`, CoreHIR lowering runs; currently this
    /// falls back to the legacy AST lowerer because `lower_hir_to_mir` is still
    /// a placeholder (see issues/open/508-legacy-path-removal-unblocked-by.md).
    /// When `hint` is `None`, both paths are lowered (needed for `compare_mir_paths`).
    #[allow(deprecated)] // accepts deprecated Legacy/OptimizedLegacy during deprecation period
    fn run_frontend_for(
        &mut self,
        path: &Path,
        hint: Option<MirSelection>,
    ) -> Result<FrontendResult, String> {
        let t_total = std::time::Instant::now();
        let mut resolved = self.resolve(path)?.resolved;

        // Inject WIT-imported function names into the symbol table so the
        // type-checker does not reject calls to them as undefined symbols.
        if !self.wit_files.is_empty() {
            let wit_functions = self.collect_wit_extern_functions();
            inject_wit_externs(
                &mut resolved.symbols,
                resolved.global_scope,
                &wit_functions
                    .iter()
                    .map(|(name, _, _)| name.as_str())
                    .collect::<Vec<_>>(),
            );
        }

        // Preserve warnings from resolution (e.g., unused imports)
        let resolve_warnings: Vec<ark_diagnostics::Diagnostic> = self
            .sink
            .diagnostics()
            .iter()
            .filter(|d| d.severity() == ark_diagnostics::Severity::Warning)
            .cloned()
            .collect();
        self.sink = DiagnosticSink::new();
        for w in resolve_warnings {
            self.sink.emit(w);
        }

        let t_tc = std::time::Instant::now();
        let mut checker = TypeChecker::new();
        checker.register_builtins();
        if !self.wit_files.is_empty() {
            for (name, params, ret) in self.collect_wit_extern_functions() {
                checker.register_extern_function(name, params, ret);
            }
        }
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

        let need_legacy = !matches!(
            hint,
            Some(MirSelection::CoreHir | MirSelection::OptimizedCoreHir)
        );
        let need_corehir = !matches!(
            hint,
            Some(MirSelection::Legacy | MirSelection::OptimizedLegacy)
        );

        let legacy_mir = if need_legacy {
            let mut mir = lower_legacy_only(&resolved.module, &checker, &mut self.sink);
            mark_selection(&mut mir, MirSelection::Legacy);
            validate_mir(&mir)?;
            mir
        } else {
            MirModule::new()
        };

        let corehir_mir = if need_corehir {
            if corehir_valid {
                let mut mir = lower_check_output_to_mir(
                    &resolved.module,
                    &core_hir,
                    &checker,
                    &mut self.sink,
                )
                .unwrap_or_else(|_| {
                    // Fallback: need legacy MIR
                    let mut fb = if need_legacy {
                        legacy_mir.clone()
                    } else {
                        let mut m = lower_legacy_only(&resolved.module, &checker, &mut self.sink);
                        mark_selection(&mut m, MirSelection::Legacy);
                        m
                    };
                    mark_selection(&mut fb, MirSelection::CoreHir);
                    fb
                });
                mark_selection(&mut mir, MirSelection::CoreHir);
                if validate_mir(&mir).is_err() {
                    let mut fallback = if need_legacy {
                        legacy_mir.clone()
                    } else {
                        let mut m = lower_legacy_only(&resolved.module, &checker, &mut self.sink);
                        mark_selection(&mut m, MirSelection::Legacy);
                        m
                    };
                    mark_selection(&mut fallback, MirSelection::CoreHir);
                    fallback
                } else {
                    mir
                }
            } else {
                let mut fallback = if need_legacy {
                    legacy_mir.clone()
                } else {
                    let mut m = lower_legacy_only(&resolved.module, &checker, &mut self.sink);
                    mark_selection(&mut m, MirSelection::Legacy);
                    m
                };
                mark_selection(&mut fallback, MirSelection::CoreHir);
                fallback
            }
        } else {
            MirModule::new()
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

    /// Collect WIT-imported functions declared by the configured WIT files.
    ///
    /// Silent on IO / parse errors — malformed WIT files are rejected earlier
    /// by the CLI preflight before the driver runs.
    fn collect_wit_extern_functions(&self) -> Vec<(String, Vec<Type>, Type)> {
        let mut functions = Vec::new();
        for wit_path in &self.wit_files {
            if let Ok(text) = std::fs::read_to_string(wit_path) {
                if let Ok(doc) = ark_wasm::component::parse_wit(&text) {
                    for iface in &doc.interfaces {
                        for func in &iface.functions {
                            functions.push((
                                func.name.clone(),
                                func.params
                                    .iter()
                                    .map(|(_, ty)| checker_type_from_wit_type(ty))
                                    .collect(),
                                func.result
                                    .as_ref()
                                    .map(checker_type_from_wit_type)
                                    .unwrap_or(Type::Unit),
                            ));
                        }
                    }
                }
            }
        }
        functions
    }

    pub fn check(&mut self, path: &Path) -> Result<(), String> {
        let result = self.check_core_hir(path);
        // Print any warnings (including unused imports) even on success,
        // filtering by lint_allow/lint_deny configuration
        if self.sink.has_warnings() {
            let filtered: Vec<_> = self
                .sink
                .diagnostics()
                .iter()
                .filter(|d| !self.lint_allow.iter().any(|code| d.code.as_str() == code))
                .cloned()
                .collect();
            // Check for denied lint rules escalated to errors
            let has_denied = filtered.iter().any(|d| {
                d.severity() == ark_diagnostics::Severity::Warning
                    && self.lint_deny.iter().any(|code| d.code.as_str() == code)
            });
            if !filtered.is_empty() {
                eprint!("{}", render_diagnostics(&filtered, &self.source_map));
            }
            if has_denied {
                return Err("lint rules denied by configuration".to_string());
            }
        }
        result.map(|_| ())
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

    #[allow(deprecated)] // handles deprecated Legacy/OptimizedLegacy during deprecation period
    pub fn lower_mir_selected(
        &mut self,
        path: &Path,
        selection: MirSelection,
    ) -> Result<MirModule, String> {
        let frontend = self.run_frontend_for(path, Some(selection))?;
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
        self.compile_selected(path, target, MirSelection::CoreHir)
            .map(|compiled| compiled.wasm)
    }

    pub fn compile_selected(
        &mut self,
        path: &Path,
        target: TargetId,
        selection: MirSelection,
    ) -> Result<CompiledModule, String> {
        self.compile_with_entry(path, target, selection, None)
    }

    #[allow(deprecated)] // handles deprecated Legacy/OptimizedLegacy during deprecation period
    pub fn compile_with_entry(
        &mut self,
        path: &Path,
        target: TargetId,
        selection: MirSelection,
        entry_override: Option<&str>,
    ) -> Result<CompiledModule, String> {
        let t_total = std::time::Instant::now();
        if target == TargetId::Native {
            return Err("error: native target uses the dedicated LLVM compile path".to_string());
        }

        // Store the active target so that load_graph can pass it to the module
        // loader for compile-time target-gating (e.g. E0500 for T3-only imports on T1).
        self.active_target = Some(target);

        let frontend = self.run_frontend_for(path, Some(selection))?;
        let pending_diagnostics = frontend.pending_diagnostics.clone();
        let mut mir = match selection {
            MirSelection::Legacy | MirSelection::OptimizedLegacy => frontend.legacy_mir,
            MirSelection::CoreHir | MirSelection::OptimizedCoreHir => frontend.corehir_mir,
        };

        if let Some(entry_name) = entry_override {
            if let Some(id) = ark_mir::mir::function_id_by_name(&mir, entry_name) {
                mir.entry_fn = Some(id);
            } else {
                return Err(format!("error: entry function `{}` not found", entry_name));
            }
        }

        // Validate MIR immediately after lowering, before any optimization pass.
        // This ensures structural invariants are caught regardless of MirSelection.
        validate_mir(&mir)?;

        // Populate WIT-derived imports for Component Model support.
        // Each WIT file is parsed and its function signatures are appended to
        // mir.imports so backend consumers can emit correct component import stubs.
        if !self.wit_files.is_empty() {
            for wit_path in self.wit_files.clone() {
                if let Ok(text) = std::fs::read_to_string(&wit_path) {
                    if let Ok(doc) = ark_wasm::component::parse_wit(&text) {
                        for iface in &doc.interfaces {
                            let imports =
                                ark_wasm::component::wit_interface_to_mir_imports(iface);
                            mir.imports.extend(imports);
                        }
                    }
                }
            }
        }

        // T3 MIR opt-level: blanket O0 removed 2026-04-15 (issue #486).
        // All passes in crates/ark-mir/src/passes/ are safe for T3 because they
        // operate on pure scalars / CFG structure and bypass desugar_exprs.
        // The opt/pipeline.rs batch path (optimize_module_named etc.) always runs
        // desugar_exprs first, which converts high-level MIR nodes (IfExpr, TryExpr)
        // that T3 relies on into statement form — that conversion breaks T3 Wasm
        // type emission.  T3 therefore uses the passes/ standalone path.
        // For non-T3, the existing batch pipeline runs as before.
        // See crates/ark-mir/src/passes/README.md §T3 safety classification.
        let effective_mir_opt_level = self.opt_level;

        // T3-safe passes live in the passes/ directory and bypass desugar.
        // All O2 passes in opt/pipeline.rs remain gated (they go through the batch
        // pipeline which triggers desugar).  Unlock conditions documented in README.
        let t3_standalone_only = target == TargetId::Wasm32WasiP2;

        let t_opt = std::time::Instant::now();
        let opt_detail = if matches!(
            selection,
            MirSelection::OptimizedLegacy | MirSelection::OptimizedCoreHir
        ) && effective_mir_opt_level != OptLevel::O0
        {
            if t3_standalone_only {
                // T3 safe path: call passes/ standalone functions directly, bypassing
                // optimize_module_with_passes which always invokes desugar_exprs.
                // desugar_exprs converts IfExpr/TryExpr high-level nodes that T3 needs;
                // running it produces Wasm type-mismatch errors at the T3 emitter.
                // Only passes registered in passes/ (const_fold, dead_block_elim) run.
                // Other O1/O2 passes remain gated until passes/ is extended.
                let passes_level = match effective_mir_opt_level {
                    OptLevel::O1 => ark_mir::OptLevel::O1,
                    OptLevel::O2 => ark_mir::OptLevel::O2,
                    OptLevel::O0 => unreachable!("gated above"),
                };
                let stats = ark_mir::passes::run_all(&mut mir, passes_level);
                mark_selection(&mut mir, selection);
                // T3 path: emit optimized-MIR dump if requested (bypasses pipeline dump).
                if dump_phases_requested()
                    .as_deref()
                    .is_some_and(|d| d == "optimized-mir" || d == "all")
                {
                    dump_mir_phase(&mir, "post-opt");
                }
                let total: usize = stats.iter().map(|s| s.changed).sum();
                format!("t3_passes={}", total)
            } else {
                let summary = if effective_mir_opt_level == OptLevel::O1 {
                    let o1_passes: &[&str] = &[
                        "const_fold",
                        "branch_fold",
                        "cfg_simplify",
                        "copy_prop",
                        "const_prop",
                        "dead_local_elim",
                        "dead_block_elim",
                        "unreachable_cleanup",
                        "cse",
                    ];
                    let passes: Vec<&str> = o1_passes
                        .iter()
                        .filter(|p| !self.disabled_passes.iter().any(|d| d == *p))
                        .copied()
                        .collect();
                    optimize_module_named(&mut mir, &passes)
                } else if self.disabled_passes.is_empty() {
                    optimize_module(&mut mir)
                } else {
                    let all_passes = optimization_pass_catalog();
                    let passes: Vec<&str> = all_passes
                        .iter()
                        .filter(|p| !self.disabled_passes.iter().any(|d| d == *p))
                        .copied()
                        .collect();
                    optimize_module_named(&mut mir, &passes)
                };
                let summary = summary
                    .map_err(|message| format!("internal error: optimizer failed: {message}"))?;
                mark_selection(&mut mir, selection);
                format!(
                    "rounds={}, const_fold={}, dce={}",
                    summary.rounds, summary.const_folded, summary.dead_locals_removed
                )
            }
        } else {
            String::new()
        };
        let opt_ms = t_opt.elapsed().as_secs_f64() * 1000.0;

        ensure_runtime_entry(&mir, selection)?;
        // Both T1 and T3 backends handle high-level IR nodes (IfExpr, LoopExpr, TryExpr)
        // directly. The stricter backend-legal check is deferred until the CoreHIR lowerer
        // produces its own flat basic-block MIR (currently it falls back to legacy lowering).

        // Dead function elimination: remove stdlib functions not reachable from main.
        // Skipped for T3 until entry-point reachability includes all WASI exports.
        if effective_mir_opt_level != OptLevel::O0
            && !t3_standalone_only
            && std::env::var("ARUKELLT_NO_DEAD_FN").is_err()
        {
            eliminate_dead_functions(&mut mir);
        }

        let plan = build_backend_plan(target, EmitKind::CoreWasm)?;

        self.sink = DiagnosticSink::new();
        // Re-emit frontend warnings into the current sink so they appear on success.
        for diag in pending_diagnostics {
            self.sink.emit(diag);
        }
        let t_emit = std::time::Instant::now();
        let opt_u8 = match self.opt_level {
            OptLevel::O0 => 0u8,
            OptLevel::O1 => 1u8,
            OptLevel::O2 => 2u8,
        };
        let wasm = ark_wasm::emit_with_plan(&mir, &mut self.sink, &plan, opt_u8, self.strip_debug);
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
        if self.timing_enabled
            && let Some(ref mut timing) = self.last_timing
        {
            timing.opt_ms = opt_ms;
            timing.opt_detail = opt_detail;
            timing.emit_ms = emit_ms;
            timing.total_ms = t_total.elapsed().as_secs_f64() * 1000.0;
        }

        mark_selection(&mut mir, selection);
        Ok(CompiledModule {
            mir,
            wasm,
            selection,
        })
    }

    pub fn compile_wit(&mut self, path: &Path) -> Result<String, String> {
        self.compile_wit_with_world(path, None)
    }

    /// Compile to WIT text, optionally targeting a standard WASI world.
    pub fn compile_wit_with_world(
        &mut self,
        path: &Path,
        world_spec: Option<&str>,
    ) -> Result<String, String> {
        let frontend = self.run_frontend(path)?;
        let world_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
        let world = ark_wasm::component::mir_to_wit_world_with_warnings(
            &frontend.corehir_mir,
            world_name,
            world_spec,
        )
        .map(|(w, _)| w)
        .map_err(|e| format!("WIT generation error: {}", e))?;
        ark_wasm::component::generate_wit(&world)
            .map_err(|e| format!("WIT generation error: {}", e))
    }

    /// Compile to a Component Model binary (.component.wasm).
    ///
    /// Pipeline: frontend → MIR → core Wasm → WIT generation → component wrapping.
    pub fn compile_component(&mut self, path: &Path, target: TargetId) -> Result<Vec<u8>, String> {
        self.compile_component_with_world(path, target, None)
    }

    /// Like `compile_component` but optionally targeting a standard WASI world.
    pub fn compile_component_with_world(
        &mut self,
        path: &Path,
        target: TargetId,
        world_spec: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        if target == TargetId::Native {
            return Err("error: component model requires a Wasm target".to_string());
        }
        // Store target for module loader gating (same as compile_with_entry).
        self.active_target = Some(target);
        let profile = target.profile();
        if !profile.component_model {
            return Err(format!(
                "error: component model requires --target wasm32-wasi-p2 (target `{}` does not support components)",
                target
            ));
        }

        // Step 1: Compile to core Wasm
        let compiled = self.compile_selected(path, target, MirSelection::CoreHir)?;

        // Step 2: Generate WIT (with warnings for non-exportable functions)
        let world_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
        let (world, export_warnings) = ark_wasm::component::mir_to_wit_world_with_warnings(
            &compiled.mir,
            world_name,
            world_spec,
        )
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
        let component_bytes = ark_wasm::component::wrap::wrap_core_to_component(
            &compiled.wasm,
            &wit_text,
            self.p2_native,
        )
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

    pub fn find_tests(&mut self, path: &Path) -> Result<Vec<String>, String> {
        let parsed = self.parse(path)?;
        let mut tests = Vec::new();
        for item in &parsed.items {
            if let ast::Item::FnDef(f) = item
                && f.name.starts_with("test_")
                && f.params.is_empty()
            {
                tests.push(f.name.clone());
            }
        }
        Ok(tests)
    }

    pub fn profile_memory(&mut self, path: &Path) -> Result<String, String> {
        let frontend = self.run_frontend(path)?;
        let escape_info = ark_mir::escape::analyze_module(&frontend.corehir_mir);
        Ok(ark_mir::escape::format_escape_info(&escape_info))
    }

    pub fn cache_key_for(&self, path: &Path) -> Option<&PhaseKey> {
        self.artifacts.key_for_path(path)
    }

    /// Read the compiler process's own RSS from `/proc/self/status` (Linux only).
    ///
    /// Returns a formatted summary of VmRSS (current resident size) and
    /// VmHWM (peak resident size, aka high-water mark).
    pub fn profile_rss() -> String {
        #[cfg(target_os = "linux")]
        {
            match std::fs::read_to_string("/proc/self/status") {
                Ok(contents) => {
                    let mut rss_kb: Option<u64> = None;
                    let mut hwm_kb: Option<u64> = None;
                    for line in contents.lines() {
                        if let Some(rest) = line.strip_prefix("VmRSS:") {
                            rss_kb = rest
                                .trim()
                                .strip_suffix("kB")
                                .and_then(|v| v.trim().parse().ok());
                        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
                            hwm_kb = rest
                                .trim()
                                .strip_suffix("kB")
                                .and_then(|v| v.trim().parse().ok());
                        }
                    }
                    let rss = rss_kb.map_or("unknown".to_string(), |v| format!("{} KB", v));
                    let hwm = hwm_kb.map_or("unknown".to_string(), |v| format!("{} KB", v));
                    format!("[memory] VmRSS: {}, VmHWM: {}", rss, hwm)
                }
                Err(e) => {
                    format!("[memory] warning: failed to read /proc/self/status: {}", e)
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            "[memory] warning: RSS profiling is only supported on Linux".to_string()
        }
    }
}

fn _keep_pathbuf(_: &PathBuf, _: Option<ResolvedProgram>) {}

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
    #[allow(deprecated)]
    fn mir_selection_labels_are_stable() {
        assert_eq!(MirSelection::Legacy.as_str(), "legacy");
        assert_eq!(MirSelection::CoreHir.as_str(), "corehir");
        assert_eq!(MirSelection::OptimizedLegacy.as_str(), "optimized-legacy");
        assert_eq!(MirSelection::OptimizedCoreHir.as_str(), "optimized-corehir");
    }

    #[test]
    fn profile_rss_returns_memory_line() {
        let output = Session::profile_rss();
        assert!(
            output.starts_with("[memory]"),
            "expected [memory] prefix, got: {}",
            output
        );
    }
}
