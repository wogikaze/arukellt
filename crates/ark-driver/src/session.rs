use std::path::Path;

use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics};
use ark_lexer::Lexer;
use ark_mir::MirModule;
use ark_parser::parse;
use ark_resolve::ResolvedModule;
#[allow(deprecated)]
use ark_resolve::resolved_program_to_module;
use ark_target::TargetId;
use ark_typecheck::TypeChecker;

/// Result of running the full frontend pipeline (lex → parse → resolve → typecheck → MIR).
pub struct FrontendResult {
    pub resolved: ResolvedModule,
    pub checker: TypeChecker,
    pub mir: MirModule,
}

/// Result of [`Session::analyze`], exposing MIR, type checker, and resolved module
/// for downstream consumers.
pub struct AnalysisResult {
    pub resolved: ResolvedModule,
    pub checker: TypeChecker,
    pub mir: MirModule,
}

/// Compiler session holding shared state for the compilation pipeline.
pub struct Session {
    source_map: SourceMap,
    sink: DiagnosticSink,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Session {
            source_map: SourceMap::new(),
            sink: DiagnosticSink::new(),
        }
    }

    /// Add a source file to the session's source map, returning its file id.
    pub fn add_file(&mut self, name: String, source: String) -> u32 {
        self.source_map.add_file(name, source)
    }

    /// Run the full frontend pipeline: lex → parse → resolve → typecheck → MIR.
    fn run_frontend(&mut self, path: &Path) -> Result<FrontendResult, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("error: {}: {}", path.display(), e))?;

        let file_id = self.source_map.add_file(path.display().to_string(), source.clone());
        self.sink = DiagnosticSink::new();

        // Lex
        let lexer = Lexer::new(file_id, &source);
        let tokens: Vec<_> = lexer.collect();

        // Parse
        let module = parse(&tokens, &mut self.sink);
        if self.sink.has_errors() {
            return Err(render_diagnostics(self.sink.diagnostics(), &self.source_map));
        }

        // Name resolution + module loading
        let (resolved, checker, mir) =
            if let Ok(mut program) = ark_resolve::resolve_program(path, &mut self.sink) {
                ark_resolve::merge_prelude(&mut program, &mut self.sink);
                if self.sink.has_errors() {
                    return Err(render_diagnostics(
                        self.sink.diagnostics(),
                        &self.source_map,
                    ));
                }

                // Type check via program-aware API
                let mut checker = TypeChecker::new();
                checker.register_builtins();
                checker.check_program(&program, &mut self.sink);
                if self.sink.has_errors() {
                    return Err(render_diagnostics(
                        self.sink.diagnostics(),
                        &self.source_map,
                    ));
                }

                // Flatten for MIR (MIR refactoring is a later task)
                #[allow(deprecated)]
                let flat_module = resolved_program_to_module(&program);
                let mir =
                    ark_mir::lower::lower_to_mir(&flat_module, &checker, &mut self.sink);
                let resolved = ResolvedModule {
                    module: flat_module,
                    symbols: program.symbols,
                    global_scope: program.global_scope,
                };
                (resolved, checker, mir)
            } else {
                // Fallback for single-file inputs
                let resolved = ark_resolve::resolve_module(module, &mut self.sink);
                if self.sink.has_errors() {
                    return Err(render_diagnostics(
                        self.sink.diagnostics(),
                        &self.source_map,
                    ));
                }

                let mut checker = TypeChecker::new();
                checker.register_builtins();
                checker.check_module(&resolved, &mut self.sink);
                if self.sink.has_errors() {
                    return Err(render_diagnostics(
                        self.sink.diagnostics(),
                        &self.source_map,
                    ));
                }

                let mir =
                    ark_mir::lower::lower_to_mir(&resolved.module, &checker, &mut self.sink);
                (resolved, checker, mir)
            };

        Ok(FrontendResult {
            resolved,
            checker,
            mir,
        })
    }

    /// Type-check a file without compiling (lex → parse → resolve → typecheck).
    pub fn check(&mut self, path: &Path) -> Result<(), String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("error: {}: {}", path.display(), e))?;

        let file_id = self.source_map.add_file(path.display().to_string(), source.clone());
        self.sink = DiagnosticSink::new();

        let lexer = Lexer::new(file_id, &source);
        let tokens: Vec<_> = lexer.collect();

        let module = parse(&tokens, &mut self.sink);
        if self.sink.has_errors() {
            return Err(render_diagnostics(self.sink.diagnostics(), &self.source_map));
        }

        if let Ok(mut program) = ark_resolve::resolve_program(path, &mut self.sink) {
            ark_resolve::merge_prelude(&mut program, &mut self.sink);
            if self.sink.has_errors() {
                return Err(render_diagnostics(self.sink.diagnostics(), &self.source_map));
            }

            let mut checker = TypeChecker::new();
            checker.register_builtins();
            checker.check_program(&program, &mut self.sink);
            if self.sink.has_errors() {
                return Err(render_diagnostics(self.sink.diagnostics(), &self.source_map));
            }
        } else {
            // Fallback for single-file inputs
            let resolved = ark_resolve::resolve_module(module, &mut self.sink);
            if self.sink.has_errors() {
                return Err(render_diagnostics(self.sink.diagnostics(), &self.source_map));
            }

            let mut checker = TypeChecker::new();
            checker.register_builtins();
            checker.check_module(&resolved, &mut self.sink);
            if self.sink.has_errors() {
                return Err(render_diagnostics(self.sink.diagnostics(), &self.source_map));
            }
        }

        Ok(())
    }

    /// Compile a file to Wasm bytes.
    pub fn compile(&mut self, path: &Path, target: TargetId) -> Result<Vec<u8>, String> {
        let frontend = self.run_frontend(path)?;

        let wasm = ark_wasm::emit(&frontend.mir, &mut self.sink, target);

        if self.sink.has_errors() {
            return Err(render_diagnostics(self.sink.diagnostics(), &self.source_map));
        }

        // Render warnings even on successful compilation
        if self.sink.has_warnings() {
            eprint!("{}", render_diagnostics(self.sink.diagnostics(), &self.source_map));
        }

        Ok(wasm)
    }

    /// Compile a file and generate WIT text.
    pub fn compile_wit(&mut self, path: &Path) -> Result<String, String> {
        let frontend = self.run_frontend(path)?;

        let world_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
        let world = ark_wasm::component::mir_to_wit_world(&frontend.mir, world_name)
            .map_err(|e| format!("WIT generation error: {}", e))?;
        ark_wasm::component::generate_wit(&world)
            .map_err(|e| format!("WIT generation error: {}", e))
    }

    /// Run the full frontend and return analysis results for downstream use.
    pub fn analyze(&mut self, path: &Path) -> Result<AnalysisResult, String> {
        let frontend = self.run_frontend(path)?;

        Ok(AnalysisResult {
            resolved: frontend.resolved,
            checker: frontend.checker,
            mir: frontend.mir,
        })
    }

    /// Run escape analysis and return formatted profiling information.
    pub fn profile_memory(&mut self, path: &Path) -> Result<String, String> {
        let frontend = self.run_frontend(path)?;
        let escape_info = ark_mir::escape::analyze_module(&frontend.mir);
        Ok(ark_mir::escape::format_escape_info(&escape_info))
    }
}
