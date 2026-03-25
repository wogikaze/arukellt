//! Arukellt compiler CLI.
//!
//! Subcommands: compile, run, check

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics};
use ark_lexer::Lexer;
use ark_parser::parse;

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
        /// Output .wasm file (default: <input>.wasm)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Compile and run an .ark file
    Run {
        /// Input .ark file
        file: PathBuf,
    },
    /// Type-check an .ark file without compiling
    Check {
        /// Input .ark file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { file, output } => {
            let output = output.unwrap_or_else(|| file.with_extension("wasm"));
            match compile_file(&file) {
                Ok(wasm) => {
                    std::fs::write(&output, &wasm).unwrap_or_else(|e| {
                        eprintln!("error: failed to write {}: {}", output.display(), e);
                        process::exit(1);
                    });
                    eprintln!("Compiled {} -> {} ({} bytes)", file.display(), output.display(), wasm.len());
                }
                Err(errors) => {
                    eprint!("{}", errors);
                    process::exit(1);
                }
            }
        }
        Commands::Run { file } => {
            match compile_file(&file) {
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
        Commands::Check { file } => {
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
    }
}

fn compile_file(path: &PathBuf) -> Result<Vec<u8>, String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("error: {}: {}", path.display(), e))?;

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

    // Name resolution
    let resolved = ark_resolve::resolve_module(module, &mut sink);
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
    let mut emitter = ark_wasm::WasmEmitter::new();
    let wasm = emitter.emit(&mir, &mut sink);

    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    Ok(wasm)
}

fn check_file(path: &PathBuf) -> Result<(), String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("error: {}: {}", path.display(), e))?;

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

    // Name resolution
    let resolved = ark_resolve::resolve_module(module, &mut sink);
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
    use wasmtime_wasi::{WasiCtxBuilder};

    let engine = Engine::default();
    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error: {}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;

    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    let instance = linker.instantiate(&mut store, &module)
        .map_err(|e| format!("wasm instantiation error: {}", e))?;

    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("missing _start: {}", e))?;

    start.call(&mut store, ())
        .map_err(|e| format!("runtime error: {}", e))?;

    Ok(())
}
