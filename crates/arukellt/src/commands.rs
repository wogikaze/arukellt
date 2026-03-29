//! Subcommand handlers for the Arukellt CLI.

use std::path::PathBuf;
use std::process;

use ark_driver::{MirSelection, OptLevel, Session};
use ark_target::{EmitKind, TargetId};

use crate::native;
use crate::runtime::{RuntimeCaps, run_wasm_gc, run_wasm_p1};

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_compile(
    file: PathBuf,
    output: Option<PathBuf>,
    target: TargetId,
    emit_kind: EmitKind,
    wit_files: Vec<PathBuf>,
    world: Option<String>,
    profile_mem: bool,
    time: bool,
    opt_level_raw: u8,
    no_pass: Vec<String>,
    mir_select: &str,
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
        eprintln!("warning: --wit flag is only used with --emit component or --emit all");
    }

    // Validate --world flag usage
    if world.is_some()
        && emit_kind != EmitKind::Component
        && emit_kind != EmitKind::Wit
        && emit_kind != EmitKind::All
    {
        eprintln!(
            "warning: --world flag is only used with --emit component, --emit wit, or --emit all"
        );
    }
    let world_spec = world.as_deref();

    let opt_level = match OptLevel::from_u8(opt_level_raw) {
        Ok(level) => level,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // WIT-only emit
    if emit_kind == EmitKind::Wit {
        let mut session = Session::new();
        session.timing_enabled = time;
        session.opt_level = opt_level;
        session.disabled_passes = no_pass.clone();
        match session.compile_wit_with_world(&file, world_spec) {
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
        session.timing_enabled = time;
        session.opt_level = opt_level;
        session.disabled_passes = no_pass.clone();
        match session.compile_component_with_world(&file, target, world_spec) {
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
            eprintln!("{}", Session::profile_rss());
        }
        return;
    }

    let output = output.unwrap_or_else(|| file.with_extension("wasm"));
    let mut session = Session::new();
    session.timing_enabled = time;
    session.opt_level = opt_level;
    session.disabled_passes = no_pass;
    let selection = parse_mir_select(mir_select);
    match session
        .compile_selected(&file, target, selection)
        .map(|c| c.wasm)
    {
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

            if time {
                if let Some(ref timing) = session.last_timing {
                    eprintln!("{}", timing);
                }
            }

            // For --emit all, also generate WIT and component
            if emit_kind == EmitKind::All {
                if let Ok(wit_text) = session.compile_wit_with_world(&file, world_spec) {
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
                match comp_session.compile_component_with_world(&file, target, world_spec) {
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
                eprintln!("{}", Session::profile_rss());
            }
        }
        Err(errors) => {
            eprint!("{}", errors);
            process::exit(1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_run(
    file: PathBuf,
    target: TargetId,
    dirs: Vec<String>,
    deny_fs: bool,
    deny_clock: bool,
    deny_random: bool,
    profile_mem: bool,
    mir_select: &str,
    watch: bool,
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
    let selection = parse_mir_select(mir_select);

    let run_once = |session: &mut Session| -> bool {
        match session
            .compile_selected(&file, target, selection)
            .map(|c| c.wasm)
        {
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
                }
                true
            }
            Err(errors) => {
                eprint!("{}", errors);
                false
            }
        }
    };

    if !watch {
        let ok = run_once(&mut session);
        if !ok {
            process::exit(1);
        }
        return;
    }

    // --watch: poll file mtime every 200ms and recompile on change.
    eprintln!("[watch] watching {} for changes", file.display());
    run_once(&mut session);

    let mut last_mtime = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let mtime = match std::fs::metadata(&file).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if mtime != last_mtime {
            last_mtime = mtime;
            eprintln!("[watch] change detected, recompiling...");
            session.invalidate_file(&file);
            run_once(&mut session);
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

pub(crate) fn cmd_analyze_wasm_size(path: &std::path::Path) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("error: failed to read {}: {}", path.display(), e);
        process::exit(1);
    });

    let total_size = bytes.len();

    let mut sections: Vec<(&str, usize)> = Vec::new();
    let mut custom_sections: Vec<(String, usize)> = Vec::new();
    let mut code_funcs: Vec<(u32, usize)> = Vec::new();
    let mut func_index: u32 = 0;

    let parser = wasmparser::Parser::new(0);
    for payload in parser.parse_all(&bytes) {
        let payload = payload.unwrap_or_else(|e| {
            eprintln!("error: failed to parse wasm: {}", e);
            process::exit(1);
        });
        match payload {
            wasmparser::Payload::TypeSection(reader) => {
                sections.push(("type", reader.range().len()));
            }
            wasmparser::Payload::ImportSection(reader) => {
                sections.push(("import", reader.range().len()));
            }
            wasmparser::Payload::FunctionSection(reader) => {
                sections.push(("function", reader.range().len()));
            }
            wasmparser::Payload::TableSection(reader) => {
                sections.push(("table", reader.range().len()));
            }
            wasmparser::Payload::MemorySection(reader) => {
                sections.push(("memory", reader.range().len()));
            }
            wasmparser::Payload::GlobalSection(reader) => {
                sections.push(("global", reader.range().len()));
            }
            wasmparser::Payload::ExportSection(reader) => {
                sections.push(("export", reader.range().len()));
            }
            wasmparser::Payload::ElementSection(reader) => {
                sections.push(("element", reader.range().len()));
            }
            wasmparser::Payload::DataSection(reader) => {
                sections.push(("data", reader.range().len()));
            }
            wasmparser::Payload::CodeSectionStart { range, .. } => {
                sections.push(("code", range.len()));
            }
            wasmparser::Payload::CodeSectionEntry(body) => {
                code_funcs.push((func_index, body.range().len()));
                func_index += 1;
            }
            wasmparser::Payload::TagSection(reader) => {
                sections.push(("tag", reader.range().len()));
            }
            wasmparser::Payload::CustomSection(reader) => {
                let size = reader.range().len();
                custom_sections.push((reader.name().to_string(), size));
                sections.push(("custom", size));
            }
            wasmparser::Payload::StartSection { range, .. } => {
                sections.push(("start", range.len()));
            }
            wasmparser::Payload::DataCountSection { range, .. } => {
                sections.push(("datacount", range.len()));
            }
            _ => {}
        }
    }

    // Aggregate by section name
    let mut aggregated: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (name, size) in &sections {
        *aggregated.entry(name).or_insert(0) += size;
    }

    println!(
        "Wasm binary size analysis: {} ({} bytes)",
        path.display(),
        total_size
    );
    println!();

    let mut sorted: Vec<_> = aggregated.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    for (name, size) in &sorted {
        let pct = (**size as f64 / total_size as f64) * 100.0;
        println!("{}: {} bytes ({:.1}%)", name, size, pct);
    }

    if !custom_sections.is_empty() {
        println!();
        println!("Custom sections:");
        for (name, size) in &custom_sections {
            let pct = (*size as f64 / total_size as f64) * 100.0;
            println!("  custom({}): {} bytes ({:.1}%)", name, size, pct);
        }
    }

    if !code_funcs.is_empty() {
        code_funcs.sort_by(|a, b| b.1.cmp(&a.1));
        println!();
        println!("Top functions by code size:");
        for (idx, size) in code_funcs.iter().take(10) {
            println!("  func[{}]: {} bytes", idx, size);
        }
    }

    let accounted: usize = aggregated.values().sum();
    let overhead = total_size.saturating_sub(accounted);
    if overhead > 0 {
        let pct = (overhead as f64 / total_size as f64) * 100.0;
        println!();
        println!("header/overhead: {} bytes ({:.1}%)", overhead, pct);
    }
}

fn parse_mir_select(s: &str) -> MirSelection {
    match s {
        "legacy" => MirSelection::Legacy,
        "corehir" => MirSelection::CoreHir,
        other => {
            eprintln!(
                "error: unknown --mir-select value: {:?} (expected \"legacy\" or \"corehir\")",
                other
            );
            process::exit(1);
        }
    }
}
