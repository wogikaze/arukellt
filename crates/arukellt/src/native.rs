//! Native/LLVM target handling.

use std::path::PathBuf;
use std::process;

use ark_target::EmitKind;

#[cfg(feature = "llvm")]
use ark_driver::Session;
#[cfg(feature = "llvm")]
use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics};
#[cfg(feature = "llvm")]
use ark_lexer::Lexer;
#[cfg(feature = "llvm")]
use ark_parser::parse;

/// Handle `--target native` compilation (requires `llvm` feature).
pub(crate) fn compile_native_target(
    file: &PathBuf,
    output: Option<&PathBuf>,
    _emit_kind: EmitKind,
) {
    #[cfg(feature = "llvm")]
    {
        let mut session = Session::new();
        match session.compile_native_ir(file) {
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
pub(crate) fn run_native_target(file: &PathBuf) {
    #[cfg(feature = "llvm")]
    {
        let mut session = Session::new();
        match session.compile_native_ir(file) {
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
