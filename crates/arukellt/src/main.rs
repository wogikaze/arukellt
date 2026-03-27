//! Arukellt compiler CLI.
//!
//! Subcommands: compile, run, check

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

use ark_driver::Session;
use ark_target::{EmitKind, TargetId, parse_target};

#[cfg(feature = "llvm")]
use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics};
#[cfg(feature = "llvm")]
use ark_lexer::Lexer;
#[cfg(feature = "llvm")]
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
        /// Output file (default: <input>.wasm for T1, <input>.component.wasm for T3)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Emit kind (core-wasm, component, wit, all)
        #[arg(long)]
        emit: Option<EmitKind>,
        /// Show memory profiling info (escape analysis, allocation hints)
        #[arg(long)]
        profile_mem: bool,
    },
    /// Compile and run an .ark file
    Run {
        /// Input .ark file
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Grant directory access (format: path or path:ro or path:rw)
        #[arg(long = "dir", value_name = "PATH[:PERMS]")]
        dirs: Vec<String>,
        /// Deny filesystem access (overrides --dir)
        #[arg(long)]
        deny_fs: bool,
        /// Deny clock/time access
        #[arg(long)]
        deny_clock: bool,
        /// Deny random number access
        #[arg(long)]
        deny_random: bool,
        /// Show memory profiling info (escape analysis, allocation hints)
        #[arg(long)]
        profile_mem: bool,
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
    /// Start the LSP server (stdio transport)
    Lsp,
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
            profile_mem,
        } => {
            let profile = target.profile();
            let emit_kind = emit_kind.unwrap_or(profile.default_emit_kind);

            // Native target: handled separately via LLVM backend
            if target == TargetId::Native {
                compile_native_target(&file, output.as_ref(), emit_kind);
                return;
            }

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

            // WIT-only emit
            if emit_kind == EmitKind::Wit {
                let mut session = Session::new();
                match session.compile_wit(&file) {
                    Ok(wit_text) => {
                        let wit_output = output.unwrap_or_else(|| file.with_extension("wit"));
                        std::fs::write(&wit_output, &wit_text).unwrap_or_else(|e| {
                            eprintln!("error: failed to write {}: {}", wit_output.display(), e);
                            process::exit(1);
                        });
                        eprintln!(
                            "Generated WIT {} -> {} ({} bytes)",
                            file.display(),
                            wit_output.display(),
                            wit_text.len()
                        );
                    }
                    Err(errors) => {
                        eprint!("{}", errors);
                        process::exit(1);
                    }
                }
                return;
            }

            let output = output.unwrap_or_else(|| file.with_extension("wasm"));
            let mut session = Session::new();
            match session.compile(&file, target) {
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

                    // For --emit all, also generate WIT
                    if emit_kind == EmitKind::All {
                        if let Ok(wit_text) = session.compile_wit(&file) {
                            let wit_output = file.with_extension("wit");
                            if let Err(e) = std::fs::write(&wit_output, &wit_text) {
                                eprintln!(
                                    "warning: failed to write WIT {}: {}",
                                    wit_output.display(),
                                    e
                                );
                            } else {
                                eprintln!(
                                    "Generated WIT {} ({} bytes)",
                                    wit_output.display(),
                                    wit_text.len()
                                );
                            }
                        }
                    }

                    if profile_mem {
                        if let Ok(info) = session.profile_memory(&file) {
                            eprintln!("{}", info);
                        }
                    }
                }
                Err(errors) => {
                    eprint!("{}", errors);
                    process::exit(1);
                }
            }
        }
        Commands::Run {
            file,
            target,
            dirs,
            deny_fs,
            deny_clock,
            deny_random,
            profile_mem,
        } => {
            // Native target: handled separately
            if target == TargetId::Native {
                run_native_target(&file);
                return;
            }

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

            // deny_clock and deny_random are not yet enforced
            if deny_clock {
                eprintln!(
                    "error: --deny-clock is not yet implemented. The Wasm runtime always \
                     provides clock access. This flag will be supported in a future version."
                );
                process::exit(1);
            }
            if deny_random {
                eprintln!(
                    "error: --deny-random is not yet implemented. The Wasm runtime always \
                     provides random access. This flag will be supported in a future version."
                );
                process::exit(1);
            }

            if profile.experimental {
                eprintln!(
                    "warning: target {} is experimental and uses WASI Preview 1 runtime internally",
                    target.canonical_name()
                );
            }

            let mut session = Session::new();
            match session.compile(&file, target) {
                Ok(wasm) => {
                    if profile_mem {
                        if let Ok(info) = session.profile_memory(&file) {
                            eprintln!("{}", info);
                        }
                    }
                    let caps = RuntimeCaps::from_cli(&dirs, deny_fs, deny_clock, deny_random);
                    let result = match target {
                        TargetId::Wasm32WasiP2 => run_wasm_gc(&wasm, &caps),
                        _ => run_wasm_p1(&wasm, &caps),
                    };
                    if let Err(e) = result {
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
            let mut session = Session::new();
            match session.check(&file) {
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
        Commands::Lsp => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(ark_lsp::run_lsp());
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

/// Handle `--target native` compilation (requires `llvm` feature).
fn compile_native_target(file: &PathBuf, output: Option<&PathBuf>, _emit_kind: EmitKind) {
    #[cfg(feature = "llvm")]
    {
        match compile_file_native(file) {
            Ok(llvm_ir) => {
                let output = output.cloned().unwrap_or_else(|| file.with_extension("ll"));
                std::fs::write(&output, &llvm_ir).unwrap_or_else(|e| {
                    eprintln!("error: failed to write {}: {}", output.display(), e);
                    process::exit(1);
                });
                eprintln!(
                    "Compiled {} -> {} ({} bytes, target: native)",
                    file.display(),
                    output.display(),
                    llvm_ir.len()
                );
            }
            Err(errors) => {
                eprint!("{}", errors);
                process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "llvm"))]
    {
        let _ = (file, output, _emit_kind);
        eprintln!(
            "error: native target requires LLVM backend support.\n\
             Rebuild with: cargo build --features llvm\n\
             Requires LLVM 18 libraries installed on the system."
        );
        process::exit(1);
    }
}

/// Compile to LLVM IR (only available with `llvm` feature).
#[cfg(feature = "llvm")]
fn compile_file_native(path: &PathBuf) -> Result<String, String> {
    let source =
        std::fs::read_to_string(path).map_err(|e| format!("error: {}: {}", path.display(), e))?;

    let mut source_map = SourceMap::new();
    let file_id = source_map.add_file(path.display().to_string(), source.clone());

    let mut sink = DiagnosticSink::new();

    let lexer = Lexer::new(file_id, &source);
    let tokens: Vec<_> = lexer.collect();
    let module = parse(&tokens, &mut sink);
    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    let resolved = ark_resolve::resolve_program_entry(path.as_path(), &mut sink)
        .unwrap_or_else(|_| ark_resolve::resolve_module(module, &mut sink));
    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    let mut checker = ark_typecheck::TypeChecker::new();
    checker.register_builtins();
    checker.check_module(&resolved, &mut sink);
    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    let mir = ark_mir::lower::lower_to_mir(&resolved.module, &checker, &mut sink);

    let llvm_ir = ark_llvm::emit_llvm_ir(&mir, &mut sink);

    if sink.has_errors() {
        return Err(render_diagnostics(sink.diagnostics(), &source_map));
    }

    if sink.has_warnings() {
        eprint!("{}", render_diagnostics(sink.diagnostics(), &source_map));
    }

    Ok(llvm_ir)
}

/// Handle `arukellt run --target native` (requires `llvm` feature).
fn run_native_target(file: &PathBuf) {
    #[cfg(feature = "llvm")]
    {
        match compile_file_native(file) {
            Ok(llvm_ir) => {
                if let Err(e) = run_native_ir(&llvm_ir, file) {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            }
            Err(errors) => {
                eprint!("{}", errors);
                process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "llvm"))]
    {
        let _ = file;
        eprintln!(
            "error: native target requires LLVM backend support.\n\
             Rebuild with: cargo build --features llvm\n\
             Requires LLVM 18 libraries installed on the system."
        );
        process::exit(1);
    }
}

/// Execute LLVM IR: try `lli` first, fall back to `llc` + `cc` + run.
#[cfg(feature = "llvm")]
fn run_native_ir(llvm_ir: &str, source_file: &PathBuf) -> Result<(), String> {
    let stem = source_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("arukellt_out");
    let pid = process::id();
    let ir_path = PathBuf::from(format!(".arukellt_{pid}_{stem}.ll"));

    std::fs::write(&ir_path, llvm_ir).map_err(|e| format!("failed to write IR file: {}", e))?;

    // Try lli (LLVM interpreter) first
    match process::Command::new("lli").arg(&ir_path).status() {
        Ok(status) => {
            let _ = std::fs::remove_file(&ir_path);
            if !status.success() {
                process::exit(status.code().unwrap_or(1));
            }
            return Ok(());
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // lli not found, fall through to llc + cc
        }
        Err(e) => {
            let _ = std::fs::remove_file(&ir_path);
            return Err(format!("failed to run lli: {}", e));
        }
    }

    // Fallback: compile with llc + cc and run the binary
    let obj_path = PathBuf::from(format!(".arukellt_{pid}_{stem}.o"));
    let bin_path = PathBuf::from(format!(".arukellt_{pid}_{stem}"));

    let cleanup = |paths: &[&PathBuf]| {
        for p in paths {
            let _ = std::fs::remove_file(p);
        }
    };

    let llc_status = process::Command::new("llc")
        .args(["-filetype=obj", "-o"])
        .arg(&obj_path)
        .arg(&ir_path)
        .status()
        .map_err(|e| {
            cleanup(&[&ir_path]);
            format!("failed to run llc: {} (is LLVM installed?)", e)
        })?;

    if !llc_status.success() {
        cleanup(&[&ir_path, &obj_path]);
        return Err(format!("llc failed with status {}", llc_status));
    }

    let cc_status = process::Command::new("cc")
        .arg("-o")
        .arg(&bin_path)
        .arg(&obj_path)
        .status()
        .map_err(|e| {
            cleanup(&[&ir_path, &obj_path]);
            format!("failed to run cc: {}", e)
        })?;

    if !cc_status.success() {
        cleanup(&[&ir_path, &obj_path, &bin_path]);
        return Err(format!("cc failed with status {}", cc_status));
    }

    let run_result = process::Command::new(&bin_path).status();
    cleanup(&[&ir_path, &obj_path, &bin_path]);

    match run_result {
        Ok(status) if !status.success() => {
            process::exit(status.code().unwrap_or(1));
        }
        Ok(_) => Ok(()),
        Err(e) => Err(format!("failed to run compiled binary: {}", e)),
    }
}

struct DirGrant {
    host_path: String,
    guest_path: String,
    read_only: bool,
}

struct RuntimeCaps {
    dirs: Vec<DirGrant>,
    deny_fs: bool,
    deny_clock: bool,
    deny_random: bool,
}

impl RuntimeCaps {
    fn from_cli(dirs: &[String], deny_fs: bool, deny_clock: bool, deny_random: bool) -> Self {
        let dir_grants = dirs.iter().map(|s| DirGrant::parse(s)).collect();
        RuntimeCaps {
            dirs: dir_grants,
            deny_fs,
            deny_clock,
            deny_random,
        }
    }
}

impl DirGrant {
    fn parse(s: &str) -> Self {
        if let Some(path) = s.strip_suffix(":ro") {
            DirGrant {
                host_path: path.to_string(),
                guest_path: path.to_string(),
                read_only: true,
            }
        } else if let Some(path) = s.strip_suffix(":rw") {
            DirGrant {
                host_path: path.to_string(),
                guest_path: path.to_string(),
                read_only: false,
            }
        } else {
            DirGrant {
                host_path: s.to_string(),
                guest_path: s.to_string(),
                read_only: false,
            }
        }
    }
}

fn run_wasm_p1(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
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

    // deny_clock and deny_random are accepted but not yet enforced;
    // callers reject these flags before reaching this function.
    let _ = caps.deny_clock;
    let _ = caps.deny_random;

    if !caps.deny_fs {
        for grant in &caps.dirs {
            let (dp, fp) = if grant.read_only {
                (DirPerms::READ, FilePerms::READ)
            } else {
                (DirPerms::all(), FilePerms::all())
            };
            builder
                .preopened_dir(&grant.host_path, &grant.guest_path, dp, fp)
                .map_err(|e| format!("preopened dir error for '{}': {}", grant.host_path, e))?;
        }
    }
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

/// Run a Wasm GC module (T3 target) with wasmtime GC support enabled.
fn run_wasm_gc(wasm_bytes: &[u8], caps: &RuntimeCaps) -> Result<(), String> {
    use wasmtime::*;
    use wasmtime_wasi::preview1::WasiP1Ctx;
    use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

    let mut config = Config::new();
    config.wasm_gc(true);

    let engine =
        Engine::new(&config).map_err(|e| format!("engine creation error (GC): {:?}", e))?;
    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("wasm compile error (GC): {:?}", e))?;

    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| format!("wasi link error: {}", e))?;

    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();

    let _ = caps.deny_clock;
    let _ = caps.deny_random;

    if !caps.deny_fs {
        for grant in &caps.dirs {
            let (dp, fp) = if grant.read_only {
                (DirPerms::READ, FilePerms::READ)
            } else {
                (DirPerms::all(), FilePerms::all())
            };
            builder
                .preopened_dir(&grant.host_path, &grant.guest_path, dp, fp)
                .map_err(|e| format!("preopened dir error for '{}': {}", grant.host_path, e))?;
        }
    }
    let wasi_ctx = builder.build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("wasm instantiation error (GC): {}", e))?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("missing _start: {}", e))?;

    start
        .call(&mut store, ())
        .map_err(|e| format!("runtime error: {}", e))?;

    Ok(())
}
