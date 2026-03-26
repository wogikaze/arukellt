//! Arukellt compiler CLI.
//!
//! Subcommands: compile, run, check

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics};
use ark_lexer::Lexer;
use ark_parser::parse;
use ark_target::{EmitKind, TargetId, parse_target};

#[derive(Parser)]
#[command(name = "arukellt", version, about = "The Arukellt compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an .ark file to Wasm
    Compile {
        /// Input .ark file
        file: PathBuf,
        /// Output file (default: <input>.wasm for T1, <input>.component.wasm for T3)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Emit kind (core-wasm, component, wit, all)
        #[arg(long)]
        emit: Option<EmitKind>,
    },
    /// Compile and run an .ark file
    Run {
        /// Input .ark file
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
    },
    /// Type-check an .ark file without compiling
    Check {
        /// Input .ark file
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
    },
    /// List available compile targets
    Targets,
}

fn main() {
    let cli = Cli::parse();

    // Check for alias warnings in raw args (clap already parsed TargetId,
    // but we want to warn on deprecated aliases)
    check_target_alias_warning();

    match cli.command {
        Commands::Compile {
            file,
            output,
            target,
            emit: emit_kind,
        } => {
            let profile = target.profile();
            let emit_kind = emit_kind.unwrap_or(profile.default_emit_kind);
            if !profile.implemented {
                eprintln!(
                    "error: target `{}` ({}) is not yet implemented [{}]",
                    target,
                    target.tier(),
                    profile.status_label()
                );
                process::exit(1);
            }
            if let Err(e) = ark_wasm::emit::validate_emit_kind(target, emit_kind) {
                eprintln!("error: {}", e);
                process::exit(1);
            }
            let output = output.unwrap_or_else(|| file.with_extension("wasm"));
            match compile_file(&file, target) {
                Ok(wasm) => {
                    std::fs::write(&output, &wasm).unwrap_or_else(|e| {
                        eprintln!("error: failed to write {}: {}", output.display(), e);
                        process::exit(1);
                    });
                    eprintln!(
                        "Compiled {} -> {} ({} bytes, target: {})",
                        file.display(),
                        output.display(),
                        wasm.len(),
                        target,
                    );
                }
                Err(errors) => {
                    eprint!("{}", errors);
                    process::exit(1);
                }
            }
        }
        Commands::Run { file, target } => {
            let profile = target.profile();
            if !profile.run_supported {
                if !profile.implemented {
                    eprintln!(
                        "error: target `{}` ({}) is not yet implemented",
                        target,
                        target.tier()
                    );
                } else {
                    eprintln!(
                        "error: target `{}` ({}) does not support `run` (compile only)",
                        target,
                        target.tier()
                    );
                }
                process::exit(1);
            }
            match compile_file(&file, target) {
                Ok(wasm) => {
                    if let Err(e) = run_wasm(&wasm) {
                        eprintln!("error: runtime: {}", e);
                        process::exit(1);
                    }
                }
                Err(errors) => {
                    eprint!("{}", errors);
                    process::exit(1);
                }
            }
        }
        Commands::Check { file, target } => {
            let profile = target.profile();
            if !profile.implemented {
                eprintln!(
                    "error: target `{}` ({}) is not yet implemented",
                    target,
                    target.tier()
                );
                process::exit(1);
            }
            match check_file(&file) {
                Ok(()) => {
                    eprintln!("OK: {}", file.display());
                }
                Err(errors) => {
                    eprint!("{}", errors);
                    process::exit(1);
                }
            }
        }
        Commands::Targets => {
            print!("{}", ark_target::targets_help());
        }
    }
}

/// Check raw CLI args for deprecated target aliases and emit warnings.
fn check_target_alias_warning() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--target") {
        if let Some(value) = args.get(pos + 1) {
            if let Ok(result) = parse_target(value) {
                if let Some(warning) = result.alias_warning() {
                    eprintln!("{}", warning);
                }
            }
        }
    }
    // Also check --target=value form
    for arg in &args {
        if let Some(value) = arg.strip_prefix("--target=") {
            if let Ok(result) = parse_target(value) {
                if let Some(warning) = result.alias_warning() {
                    eprintln!("{}", warning);
                }
            }
        }
    }
}

fn compile_file(path: &PathBuf, _target: TargetId) -> Result<Vec<u8>, String> {
    let source =
        std::fs::read_to_string(path).map_err(|e| format!("error: {}: {}", path.display(), e))?;

    let mut source_map = SourceMap::new();
    let file_id = source_map.add_file(path.display().to_string(), source.clone());

    let mut sink = DiagnosticSink::new();

    // Lex
    let lexer = Lexer::new(file_id, &source);
    let tokens: Vec<_> = lexer.collect();

    // Parse
    let module = parse(&tokens, &mut sink);

    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    // Name resolution + module loading
    let resolved = ark_resolve::resolve_program_entry(path.as_path(), &mut sink)
        .unwrap_or_else(|_| ark_resolve::resolve_module(module, &mut sink));
    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    // Type check
    let mut checker = ark_typecheck::TypeChecker::new();
    checker.register_builtins();
    checker.check_module(&resolved, &mut sink);
    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    // Lower to MIR
    let mir = ark_mir::lower::lower_to_mir(&resolved.module, &checker, &mut sink);

    // Emit Wasm
    let wasm = ark_wasm::emit(&mir, &mut sink, _target);

    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    // Render warnings even on successful compilation
    if sink.has_warnings() {
        eprint!("{}", render_diagnostics(sink.diagnostics(), &source_map));
    }

    Ok(wasm)
}

fn check_file(path: &PathBuf) -> Result<(), String> {
    let source =
        std::fs::read_to_string(path).map_err(|e| format!("error: {}: {}", path.display(), e))?;

    let mut source_map = SourceMap::new();
    let file_id = source_map.add_file(path.display().to_string(), source.clone());

    let mut sink = DiagnosticSink::new();

    // Lex
    let lexer = Lexer::new(file_id, &source);
    let tokens: Vec<_> = lexer.collect();

    // Parse
    let module = parse(&tokens, &mut sink);

    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    // Name resolution + module loading
    let resolved = ark_resolve::resolve_program_entry(path.as_path(), &mut sink)
        .unwrap_or_else(|_| ark_resolve::resolve_module(module, &mut sink));
    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    // Type check
    let mut checker = ark_typecheck::TypeChecker::new();
    checker.register_builtins();
    checker.check_module(&resolved, &mut sink);
    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    Ok(())
}

fn run_wasm(wasm_bytes: &[u8]) -> Result<(), String> {
    use wasmtime::*;
    use wasmtime_wasi::preview1::WasiP1Ctx;
    use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

    let engine = Engine::default();
    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error: {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    // Preopen current directory for io/fs operations (fd 3)
    builder
        .preopened_dir(".", ".", DirPerms::all(), FilePerms::all())
        .map_err(|e| format!("preopened dir error: {}", e))?;
    let wasi_ctx = builder.build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("wasm instantiation error: {}", e))?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("missing _start: {}", e))?;

    start
        .call(&mut store, ())
        .map_err(|e| format!("runtime error: {}", e))?;

    Ok(())
}
