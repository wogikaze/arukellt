//! Subcommand handlers for the Arukellt CLI.

use std::path::PathBuf;
use std::process;

use ark_driver::Session;
use ark_target::{EmitKind, TargetId};

use crate::native;
use crate::runtime::{RuntimeCaps, run_wasm_gc, run_wasm_p1};

pub(crate) fn cmd_compile(
    file: PathBuf,
    output: Option<PathBuf>,
    target: TargetId,
    emit_kind: EmitKind,
    wit_files: Vec<PathBuf>,
    profile_mem: bool,
) {
    // Native target: handled separately via LLVM backend
    if target == TargetId::Native {
        native::compile_native_target(&file, output.as_ref(), emit_kind);
        return;
    }

    let profile = target.profile();
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

    // Validate --wit files exist
    for wit_path in &wit_files {
        if !wit_path.exists() {
            eprintln!("error: WIT file not found: {}", wit_path.display());
            process::exit(1);
        }
    }
    if !wit_files.is_empty() && emit_kind != EmitKind::Component && emit_kind != EmitKind::All {
        eprintln!(
            "warning: --wit flag is only used with --emit component or --emit all"
        );
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

    // Component emit
    if emit_kind == EmitKind::Component {
        let component_output = output.unwrap_or_else(|| file.with_extension("component.wasm"));
        let mut session = Session::new();
        match session.compile_component(&file, target) {
            Ok(component) => {
                std::fs::write(&component_output, &component).unwrap_or_else(|e| {
                    eprintln!(
                        "error: failed to write {}: {}",
                        component_output.display(),
                        e
                    );
                    process::exit(1);
                });
                eprintln!(
                    "Compiled component {} -> {} ({} bytes, target: {})",
                    file.display(),
                    component_output.display(),
                    component.len(),
                    target,
                );
            }
            Err(errors) => {
                eprint!("{}", errors);
                process::exit(1);
            }
        }

        if profile_mem {
            let mut session = Session::new();
            if let Ok(info) = session.profile_memory(&file) {
                eprintln!("{}", info);
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

            // For --emit all, also generate WIT and component
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
                // Also generate component
                let mut comp_session = Session::new();
                match comp_session.compile_component(&file, target) {
                    Ok(component) => {
                        let comp_output = file.with_extension("component.wasm");
                        if let Err(e) = std::fs::write(&comp_output, &component) {
                            eprintln!(
                                "warning: failed to write component {}: {}",
                                comp_output.display(),
                                e
                            );
                        } else {
                            eprintln!(
                                "Compiled component {} ({} bytes)",
                                comp_output.display(),
                                component.len()
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("warning: component generation failed: {}", e);
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

pub(crate) fn cmd_run(
    file: PathBuf,
    target: TargetId,
    dirs: Vec<String>,
    deny_fs: bool,
    deny_clock: bool,
    deny_random: bool,
    profile_mem: bool,
) {
    // Native target: handled separately
    if target == TargetId::Native {
        native::run_native_target(&file);
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

pub(crate) fn cmd_check(file: PathBuf, target: TargetId) {
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

pub(crate) fn cmd_targets() {
    print!("{}", ark_target::targets_help());
}

pub(crate) fn cmd_lsp() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(ark_lsp::run_lsp());
}
