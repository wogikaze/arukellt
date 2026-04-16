//! Subcommand handlers for the Arukellt CLI.

use std::path::PathBuf;
use std::process;

use ark_diagnostics::{DiagnosticSink, SourceMap, render_diagnostics, wit_flags_v2_diagnostic};
use ark_driver::{MirSelection, OptLevel, Session};
use ark_manifest::Manifest;
use ark_mir::mir::{MirModule, MirStmt, Operand, Rvalue};
use ark_target::{EmitKind, TargetId};
use ark_wasm::component::{WitDocument, WitFunction, WitType, parse_wit};
use serde::Serialize;

use crate::native;
use crate::runtime::{RuntimeCaps, run_wasm_gc, run_wasm_p1};

/// Effective lazy reachability for resolve: `--no-lazy-resolve` wins over `--lazy-resolve`.
pub(crate) fn effective_lazy_reachability(lazy_resolve: bool, no_lazy_resolve: bool) -> bool {
    lazy_resolve && !no_lazy_resolve
}

fn wit_type_flags_desc(ty: &WitType) -> Option<String> {
    match ty {
        WitType::Flags(names) => Some(format!("flags {{ {} }}", names.join(", "))),
        WitType::List(inner)
        | WitType::Option(inner)
        | WitType::Own(inner)
        | WitType::Borrow(inner) => wit_type_flags_desc(inner),
        WitType::Result { ok, err } => ok
            .as_deref()
            .and_then(wit_type_flags_desc)
            .or_else(|| err.as_deref().and_then(wit_type_flags_desc)),
        WitType::Tuple(elems) => elems.iter().find_map(wit_type_flags_desc),
        _ => None,
    }
}

fn wit_function_flags_desc(func: &WitFunction) -> Option<String> {
    func.params
        .iter()
        .find_map(|(_, ty)| wit_type_flags_desc(ty))
        .or_else(|| func.result.as_ref().and_then(wit_type_flags_desc))
}

fn first_flags_diagnostic(doc: &WitDocument, path: &std::path::Path) -> Option<String> {
    let mut sink = DiagnosticSink::new();
    let source_map = SourceMap::new();
    for iface in &doc.interfaces {
        for func in &iface.functions {
            if let Some(type_desc) = wit_function_flags_desc(func) {
                sink.emit(wit_flags_v2_diagnostic(
                    &path.display().to_string(),
                    &format!("{}::{}", iface.name, func.name),
                    &type_desc,
                ));
                return Some(render_diagnostics(sink.diagnostics(), &source_map));
            }
        }
    }
    None
}

fn preflight_wit_flags_for_component(wit_files: &[PathBuf]) -> Result<(), String> {
    for wit_path in wit_files {
        let Ok(wit_text) = std::fs::read_to_string(wit_path) else {
            continue;
        };
        let Ok(doc) = parse_wit(&wit_text) else {
            continue;
        };
        if let Some(rendered) = first_flags_diagnostic(&doc, wit_path) {
            return Err(rendered);
        }
    }
    Ok(())
}

pub(crate) fn cmd_init(path: PathBuf, template: crate::InitTemplate) {
    let manifest_path = path.join("ark.toml");
    if manifest_path.exists() {
        eprintln!("error: ark.toml already exists in {}", path.display());
        process::exit(1);
    }

    let project_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("hello-ark");

    let src_dir = path.join("src");
    std::fs::create_dir_all(&src_dir).unwrap_or_else(|e| {
        eprintln!("error: failed to create src directory: {}", e);
        process::exit(1);
    });

    match template {
        crate::InitTemplate::Minimal => {
            let manifest_content = format!(
                r#"[package]
name = "{}"
version = "0.1.0"

[bin]
name = "{}"
path = "src/main.ark"
"#,
                project_name, project_name
            );
            let main_ark_content = r#"use std::host::stdio

fn main() {
    stdio::println("Hello, Arukellt!")
}
"#;
            write_init_files(
                &manifest_path,
                &manifest_content,
                &src_dir,
                "main.ark",
                main_ark_content,
            );
        }
        crate::InitTemplate::Cli => {
            let manifest_content = format!(
                r#"[package]
name = "{}"
version = "0.1.0"

[bin]
name = "{}"
path = "src/main.ark"
"#,
                project_name, project_name
            );
            let main_ark_content = r#"use std::host::stdio
use std::host::process

fn greet(name: String) -> String {
    concat(concat("Hello, ", name), "!")
}

fn main() {
    // TODO: use std::host::env::args() for real CLI arg parsing when available
    let name = "World"
    stdio::println(greet(name))
    process::exit(0)
}
"#;
            write_init_files(
                &manifest_path,
                &manifest_content,
                &src_dir,
                "main.ark",
                main_ark_content,
            );
        }
        crate::InitTemplate::WithTests => {
            let manifest_content = format!(
                r#"[package]
name = "{}"
version = "0.1.0"

[bin]
name = "{}"
path = "src/main.ark"
"#,
                project_name, project_name
            );
            let main_ark_content = r#"use std::host::stdio

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

fn test_add() {
    let result = add(2, 3)
    stdio::println(i32_to_string(result))
}

fn test_subtract() {
    let result = subtract(10, 4)
    stdio::println(i32_to_string(result))
}

fn main() {
    let result = add(1, 2)
    stdio::println(concat("1 + 2 = ", i32_to_string(result)))
}
"#;
            write_init_files(
                &manifest_path,
                &manifest_content,
                &src_dir,
                "main.ark",
                main_ark_content,
            );
        }
        crate::InitTemplate::WasiHost => {
            let manifest_content = format!(
                r#"[package]
name = "{}"
version = "0.1.0"

[bin]
name = "{}"
path = "src/main.ark"
target = "wasm32-wasi-p2"
"#,
                project_name, project_name
            );
            let main_ark_content = r#"// This example targets wasm32-wasi-p2.
// Build with: arukellt run --target wasm32-wasi-p2 src/main.ark
use std::host::stdio

fn main() {
    stdio::println("Hello from WASI host!")
    stdio::println("Arukellt supports std::host::stdio for portable I/O.")
    // To make an HTTP request, add: use std::host::http
    // and call: http::get("https://example.com")
}
"#;
            write_init_files(
                &manifest_path,
                &manifest_content,
                &src_dir,
                "main.ark",
                main_ark_content,
            );
        }
    }

    let template_name = match template {
        crate::InitTemplate::Minimal => "minimal",
        crate::InitTemplate::Cli => "cli",
        crate::InitTemplate::WithTests => "with-tests",
        crate::InitTemplate::WasiHost => "wasi-host",
    };
    eprintln!(
        "Initialized Arukellt project in {} (template: {})",
        path.display(),
        template_name
    );
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  cd {}", path.display());
    eprintln!("  arukellt check src/main.ark   # type-check");
    eprintln!("  arukellt run src/main.ark     # run the program");
    match template {
        crate::InitTemplate::WithTests => {
            eprintln!("  arukellt test src/main.ark    # run tests");
        }
        crate::InitTemplate::WasiHost => {
            eprintln!("  arukellt run --target wasm32-wasi-p2 src/main.ark  # run with WASI P2");
        }
        _ => {}
    }
}

pub(crate) fn cmd_list_templates() {
    println!("minimal    - Minimal Hello World project (default)");
    println!("cli        - CLI tool with argument parsing");
    println!("with-tests - Project with test functions");
    println!("wasi-host  - WASI host API usage example (wasm32-wasi-p2)");
}

fn write_init_files(
    manifest_path: &std::path::Path,
    manifest_content: &str,
    src_dir: &std::path::Path,
    main_filename: &str,
    main_content: &str,
) {
    std::fs::write(manifest_path, manifest_content).unwrap_or_else(|e| {
        eprintln!("error: failed to write ark.toml: {}", e);
        process::exit(1);
    });

    let main_path = src_dir.join(main_filename);
    if !main_path.exists() {
        std::fs::write(&main_path, main_content).unwrap_or_else(|e| {
            eprintln!("error: failed to write src/{}: {}", main_filename, e);
            process::exit(1);
        });
    }
}

pub(crate) fn cmd_fmt(files: Vec<PathBuf>, check: bool) {
    let targets = if files.is_empty() {
        // Find all .ark files in the project
        collect_ark_files(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    } else {
        files
    };

    if targets.is_empty() {
        eprintln!("No .ark files found");
        process::exit(1);
    }

    let mut unformatted = 0;
    let mut formatted_count = 0;

    for path in &targets {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error reading {}: {}", path.display(), e);
                continue;
            }
        };

        let result = match ark_parser::fmt::format_source(&source) {
            Some(formatted) => formatted,
            None => {
                eprintln!("skipping {} (parse errors)", path.display());
                continue;
            }
        };

        if result != source {
            if check {
                eprintln!("would reformat {}", path.display());
                unformatted += 1;
            } else {
                if let Err(e) = std::fs::write(path, &result) {
                    eprintln!("error writing {}: {}", path.display(), e);
                    continue;
                }
                eprintln!("formatted {}", path.display());
                formatted_count += 1;
            }
        }
    }

    if check {
        if unformatted > 0 {
            eprintln!(
                "{} file(s) would be reformatted ({} checked)",
                unformatted,
                targets.len()
            );
            process::exit(1);
        } else {
            eprintln!("All {} file(s) are formatted", targets.len());
        }
    } else if formatted_count > 0 {
        eprintln!(
            "Formatted {} file(s) ({} checked)",
            formatted_count,
            targets.len()
        );
    }
}

/// Recursively collect all .ark files under a directory.
fn collect_ark_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip common non-source directories
                let name = path.file_name().unwrap_or_default().to_str().unwrap_or("");
                if name == "target" || name == ".git" || name == "node_modules" {
                    continue;
                }
                result.extend(collect_ark_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "ark") {
                result.push(path);
            }
        }
    }
    result.sort();
    result
}

pub(crate) fn cmd_build(
    target: TargetId,
    opt_level_raw: u8,
    strip_debug: bool,
    mir_select: &str,
    profile_mem: bool,
    time: bool,
    lazy_reachability: bool,
) {
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("error: cannot determine current directory: {}", e);
        process::exit(1);
    });

    let (project_root, manifest) = Manifest::find_and_load(&cwd).unwrap_or_else(|e| {
        match e {
            ark_manifest::ManifestError::NotFound => {
                eprintln!("error: ark.toml not found in current directory or any parent");
                eprintln!("hint: run `arukellt init` to create a new project, or `arukellt compile <file>` to compile a single file");
            }
            ark_manifest::ManifestError::Toml(ref te) => {
                eprintln!("error: failed to parse ark.toml: {te}");
            }
            _ => {
                eprintln!("error: failed to load ark.toml: {e}");
            }
        }
        process::exit(1);
    });

    let bin = manifest.require_bin().unwrap_or_else(|_| {
        eprintln!("error: ark.toml must contain a [bin] section with `name` and `path` fields");
        eprintln!("hint: add the following to your ark.toml:\n\n[bin]\nname = \"my-app\"\npath = \"src/main.ark\"");
        process::exit(1);
    });

    let input_file = project_root.join(&bin.path);
    let output_file = project_root.join(format!("{}.wasm", bin.name));

    let profile = target.profile();
    let emit_kind = profile.default_emit_kind;

    let world = manifest.world.as_ref().map(|w| w.name.clone());

    cmd_compile(
        input_file,
        Some(output_file),
        target,
        emit_kind,
        vec![],
        world,
        false,
        profile_mem,
        time,
        opt_level_raw,
        strip_debug,
        vec![],
        mir_select,
        false,
        lazy_reachability,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_compile(
    file: PathBuf,
    output: Option<PathBuf>,
    target: TargetId,
    emit_kind: EmitKind,
    wit_files: Vec<PathBuf>,
    world: Option<String>,
    p2_native: bool,
    profile_mem: bool,
    time: bool,
    opt_level_raw: u8,
    strip_debug: bool,
    no_pass: Vec<String>,
    mir_select: &str,
    json: bool,
    lazy_reachability: bool,
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
    if !wit_files.is_empty()
        && (emit_kind == EmitKind::Component || emit_kind == EmitKind::All)
        && let Err(rendered) = preflight_wit_flags_for_component(&wit_files)
    {
        eprint!("{}", rendered);
        process::exit(1);
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

    // Validate --p2-native flag usage
    if p2_native && target != TargetId::Wasm32WasiP2 {
        eprintln!(
            "error: --p2-native requires --target wasm32-wasi-p2 (current target: {})",
            target
        );
        process::exit(1);
    }
    if p2_native && emit_kind != EmitKind::Component && emit_kind != EmitKind::All {
        eprintln!("warning: --p2-native only affects --emit component or --emit all");
    }

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
        session.set_lazy_reachability(lazy_reachability);
        session.timing_enabled = time;
        session.opt_level = opt_level;
        session.strip_debug = strip_debug;
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
        session.set_lazy_reachability(lazy_reachability);
        session.timing_enabled = time;
        session.opt_level = opt_level;
        session.strip_debug = strip_debug;
        session.disabled_passes = no_pass.clone();
        session.p2_native = p2_native;
        session.wit_files = wit_files.clone();
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
            session.set_lazy_reachability(lazy_reachability);
            if let Ok(info) = session.profile_memory(&file) {
                eprintln!("{}", info);
            }
            eprintln!("{}", Session::profile_rss());
        }
        return;
    }

    let output = output.unwrap_or_else(|| file.with_extension("wasm"));
    let mut session = Session::new();
    session.set_lazy_reachability(lazy_reachability);
    session.timing_enabled = time || json;
    session.opt_level = opt_level;
    session.strip_debug = strip_debug;
    session.disabled_passes = no_pass.clone();
    session.wit_files = wit_files.clone();
    let selection = parse_mir_select(mir_select);

    if json {
        let result = session.compile_selected(&file, target, selection);
        match result {
            Ok(compiled) => {
                let timing = session.last_timing.clone();
                let output_json = serde_json::json!({
                    "status": "success",
                    "file": file.display().to_string(),
                    "wasm_size": compiled.wasm.len(),
                    "timing": timing,
                });
                println!("{}", output_json);
            }
            Err(errors) => {
                let output_json = serde_json::json!({
                    "status": "error",
                    "errors": errors,
                });
                println!("{}", output_json);
                process::exit(1);
            }
        }
        return;
    }

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

            if time && let Some(ref timing) = session.last_timing {
                eprintln!("{}", timing);
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
                comp_session.set_lazy_reachability(lazy_reachability);
                comp_session.timing_enabled = time;
                comp_session.opt_level = opt_level;
                comp_session.strip_debug = strip_debug;
                comp_session.disabled_passes = no_pass.clone();
                comp_session.p2_native = p2_native;
                comp_session.wit_files = wit_files.clone();
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
    strip_debug: bool,
    mir_select: &str,
    watch: bool,
    lazy_reachability: bool,
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

    // deny_clock and deny_random are enforced at compile time (below)

    if profile.experimental {
        eprintln!(
            "warning: target {} is experimental and uses WASI Preview 1 runtime internally",
            target.canonical_name()
        );
    }

    let mut session = Session::new();
    session.set_lazy_reachability(lazy_reachability);
    session.strip_debug = strip_debug;
    let selection = parse_mir_select(mir_select);

    let run_once = |session: &mut Session| -> bool {
        match session.compile_selected(&file, target, selection) {
            Ok(compiled) => {
                // Enforce --deny-clock / --deny-random at compile time
                if deny_clock && mir_uses_capability(&compiled.mir, CLOCK_BUILTINS) {
                    eprintln!(
                        "error: --deny-clock: this program uses clock intrinsics, \
                         which are denied by the current capability policy"
                    );
                    return false;
                }
                if deny_random && mir_uses_capability(&compiled.mir, RANDOM_BUILTINS) {
                    eprintln!(
                        "error: --deny-random: this program uses random intrinsics, \
                         which are denied by the current capability policy"
                    );
                    return false;
                }

                // Reject calls to host_stub functions (always-unimplemented)
                if mir_uses_capability(&compiled.mir, HOST_STUB_BUILTINS) {
                    eprintln!(
                        "error: this program calls an unimplemented host API (host_stub). \
                         Functions marked host_stub (e.g. sockets::connect) \
                         are not yet available."
                    );
                    return false;
                }

                if profile_mem && let Ok(info) = session.profile_memory(&file) {
                    eprintln!("{}", info);
                }
                let caps = RuntimeCaps::from_cli(&dirs, deny_fs, deny_clock, deny_random);
                let result = match target {
                    TargetId::Wasm32WasiP2 => run_wasm_gc(&compiled.wasm, &caps),
                    _ => run_wasm_p1(&compiled.wasm, &caps),
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

pub(crate) fn cmd_check(file: PathBuf, target: TargetId, lazy_reachability: bool) {
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
    session.set_lazy_reachability(lazy_reachability);
    // Set the target so that target-gating checks (e.g., E0500 for T1-incompatible
    // imports) work during the resolve phase.
    session.active_target = Some(target);
    // Load lint config from ark.toml if available
    if let Some(root) = Manifest::find_root(&std::env::current_dir().unwrap_or_default())
        && let Ok(manifest) = Manifest::load_from_dir(&root)
        && let Some(lint) = &manifest.lint
    {
        session.lint_allow = lint.allow.clone();
        session.lint_deny = lint.deny.clone();
    }
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

pub(crate) fn cmd_lint(
    file: Option<PathBuf>,
    target: TargetId,
    list: bool,
    lazy_reachability: bool,
) {
    use ark_diagnostics::LintRegistry;

    let registry = LintRegistry::new();

    if list {
        println!("Available lint rules ({}):\n", registry.len());
        println!("{:<8} {:<14} {:<7} Description", "ID", "Category", "Level");
        println!("{}", "-".repeat(70));
        for rule in registry.rules() {
            let level = match rule.default_level {
                ark_diagnostics::LintLevel::Allow => "allow",
                ark_diagnostics::LintLevel::Warn => "warn",
                ark_diagnostics::LintLevel::Deny => "deny",
            };
            println!(
                "{:<8} {:<14} {:<7} {}",
                rule.id,
                rule.category.as_str(),
                level,
                rule.description
            );
        }
        return;
    }

    let file = match file {
        Some(f) => f,
        None => {
            eprintln!("error: <FILE> is required when not using --list");
            process::exit(1);
        }
    };

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
    session.set_lazy_reachability(lazy_reachability);
    // Set the target so that target-gating checks (e.g., E0500 for T1-incompatible
    // imports) work during the resolve phase.
    session.active_target = Some(target);
    // Load lint config from ark.toml if available
    if let Some(root) = Manifest::find_root(&std::env::current_dir().unwrap_or_default())
        && let Ok(manifest) = Manifest::load_from_dir(&root)
        && let Some(lint) = &manifest.lint
    {
        session.lint_allow = lint.allow.clone();
        session.lint_deny = lint.deny.clone();
    }
    match session.check(&file) {
        Ok(()) => {
            eprintln!("lint OK: {}", file.display());
        }
        Err(errors) => {
            eprint!("{}", errors);
            process::exit(1);
        }
    }
}

#[derive(Serialize)]
struct TestResult {
    name: String,
    status: String,
    message: Option<String>,
    duration_ms: f64,
}

#[derive(Serialize)]
struct TestSuiteResult {
    file: String,
    tests: Vec<TestResult>,
    passed: usize,
    failed: usize,
    total_duration_ms: f64,
}

pub(crate) fn cmd_test(
    file: PathBuf,
    target: TargetId,
    json: bool,
    list: bool,
    filter: Option<String>,
    lazy_reachability: bool,
) {
    let mut session = Session::new();
    session.set_lazy_reachability(lazy_reachability);
    let tests = match session.find_tests(&file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error discovering tests: {}", e);
            process::exit(1);
        }
    };

    let tests = if let Some(ref pat) = filter {
        tests
            .into_iter()
            .filter(|t| t.contains(pat.as_str()))
            .collect::<Vec<_>>()
    } else {
        tests
    };

    if list {
        if json {
            println!(
                "{}",
                serde_json::to_string(&tests).expect("test list serialization")
            );
        } else {
            for t in tests {
                println!("{}", t);
            }
        }
        return;
    }

    if tests.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::to_string(&TestSuiteResult {
                    file: file.display().to_string(),
                    tests: vec![],
                    passed: 0,
                    failed: 0,
                    total_duration_ms: 0.0,
                })
                .expect("empty suite serialization")
            );
        } else {
            println!("no tests found in {}", file.display());
        }
        return;
    }

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let t_suite_start = std::time::Instant::now();

    // Always use CoreHir: both Legacy and CoreHir currently fall back to the legacy
    // AST lowerer (lower_hir_to_mir is still a stub), so the output is identical.
    // Legacy variant is deprecated — use CoreHir unconditionally.
    // ARK_USE_COREHIR env var is no longer needed and is ignored.
    let selection = MirSelection::OptimizedCoreHir;

    for test_name in tests {
        if !json {
            print!("test {} ... ", test_name);
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
        let t_start = std::time::Instant::now();
        let compile_result = session.compile_with_entry(&file, target, selection, Some(&test_name));
        let duration = t_start.elapsed().as_secs_f64() * 1000.0;

        let result = match compile_result {
            Ok(compiled) => {
                let caps = RuntimeCaps::from_cli(&[], false, false, false);
                let run_result = if target == TargetId::Wasm32WasiP1 {
                    crate::runtime::run_wasm_p1(&compiled.wasm, &caps)
                } else {
                    crate::runtime::run_wasm_gc(&compiled.wasm, &caps)
                };
                match run_result {
                    Ok(_) => {
                        passed += 1;
                        if !json {
                            println!("ok");
                        }
                        TestResult {
                            name: test_name,
                            status: "pass".to_string(),
                            message: None,
                            duration_ms: duration,
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        if !json {
                            println!("FAILED");
                            println!("  runtime error: {}", e);
                        }
                        TestResult {
                            name: test_name,
                            status: "fail".to_string(),
                            message: Some(format!("runtime error: {}", e)),
                            duration_ms: duration,
                        }
                    }
                }
            }
            Err(e) => {
                failed += 1;
                if !json {
                    println!("FAILED");
                    println!("  compile error: {}", e);
                }
                TestResult {
                    name: test_name,
                    status: "fail".to_string(),
                    message: Some(format!("compile error: {}", e)),
                    duration_ms: duration,
                }
            }
        };
        results.push(result);
    }

    let suite_duration = t_suite_start.elapsed().as_secs_f64() * 1000.0;
    if json {
        let suite = TestSuiteResult {
            file: file.display().to_string(),
            tests: results,
            passed,
            failed,
            total_duration_ms: suite_duration,
        };
        println!(
            "{}",
            serde_json::to_string(&suite).expect("test suite serialization")
        );
    } else {
        println!();
        println!(
            "test result: {}. {} passed; {} failed; finished in {:.2}ms",
            if failed == 0 { "ok" } else { "FAILED" },
            passed,
            failed,
            suite_duration
        );
        if failed > 0 {
            process::exit(1);
        }
    }
}

pub(crate) fn cmd_script_list(json: bool) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot determine current directory: {}", e);
            process::exit(1);
        }
    };
    let root = Manifest::find_root(&cwd).unwrap_or_else(|| {
        eprintln!("error: ark.toml not found in current directory or any parent");
        process::exit(1);
    });
    let manifest = match Manifest::load_from_dir(&root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "error: failed to load ark.toml in {}: {}",
                root.display(),
                e
            );
            process::exit(1);
        }
    };

    if json {
        println!(
            "{}",
            serde_json::to_string(&manifest.scripts).expect("scripts serialization")
        );
    } else {
        println!("Scripts in {}:", root.display());
        let mut names: Vec<_> = manifest.scripts.keys().collect();
        names.sort();
        for name in names {
            if let Some(cmd) = manifest.scripts.get(name) {
                println!("  {:10}  {}", name, cmd);
            }
        }
    }
}

pub(crate) fn cmd_script_run(name: String, extra_args: Vec<String>) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot determine current directory: {}", e);
            process::exit(1);
        }
    };
    let root = Manifest::find_root(&cwd).unwrap_or_else(|| {
        eprintln!("error: ark.toml not found in current directory or any parent");
        process::exit(1);
    });
    let manifest = match Manifest::load_from_dir(&root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "error: failed to load ark.toml in {}: {}",
                root.display(),
                e
            );
            process::exit(1);
        }
    };

    let script = manifest.scripts.get(&name).unwrap_or_else(|| {
        eprintln!("error: script `{}` not found in ark.toml", name);
        process::exit(1);
    });

    let full_command = if extra_args.is_empty() {
        script.clone()
    } else {
        format!("{} {}", script, extra_args.join(" "))
    };

    let child = if cfg!(target_os = "windows") {
        process::Command::new("cmd")
            .arg("/C")
            .arg(&full_command)
            .current_dir(&root)
            .spawn()
    } else {
        process::Command::new("sh")
            .arg("-c")
            .arg(&full_command)
            .current_dir(&root)
            .spawn()
    };

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to run script `{}`: {}", name, e);
            process::exit(1);
        }
    };

    match child.wait() {
        Ok(status) => process::exit(status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("error: script `{}` failed: {}", name, e);
            process::exit(1);
        }
    }
}

pub(crate) fn cmd_targets() {
    print!("{}", ark_target::targets_help());
}

pub(crate) fn cmd_lsp() {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime for LSP: {}", e);
            process::exit(1);
        }
    };
    rt.block_on(ark_lsp::run_lsp());
}

pub(crate) fn cmd_debug_adapter() {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!(
                "error: failed to start async runtime for debug adapter: {}",
                e
            );
            process::exit(1);
        }
    };
    if let Err(e) = rt.block_on(ark_dap::run_dap()) {
        eprintln!("error: debug adapter: {}", e);
        process::exit(1);
    }
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
        "legacy" => {
            eprintln!(
                "warning: --mir-select legacy is deprecated and will be removed in a future release"
            );
            #[allow(deprecated)]
            {
                MirSelection::Legacy
            }
        }
        "corehir" | "optimized-corehir" => MirSelection::OptimizedCoreHir,
        "corehir-debug" => MirSelection::CoreHir,
        other => {
            eprintln!(
                "error: unknown --mir-select value: {:?} (expected \"legacy\", \"corehir\", \"corehir-debug\", or \"optimized-corehir\")",
                other
            );
            process::exit(1);
        }
    }
}

// ── Capability scanning ────────────────────────────────────────────

const CLOCK_BUILTINS: &[&str] = &[
    "clock_now",
    "clock_now_ms",
    "monotonic_now",
    "__intrinsic_clock_now",
    "__intrinsic_clock_now_ms",
];

const RANDOM_BUILTINS: &[&str] = &[
    "random_i32",
    "random_f64",
    "next_f64",
    "__intrinsic_random_i32",
    "__intrinsic_random_f64",
    "__intrinsic_random_next_f64",
];

/// Functions marked `kind = "host_stub"` in std/manifest.toml.
/// These always return Err("not implemented") and should be rejected at
/// compile time rather than letting users discover the failure at runtime.
///
/// `sockets_connect` / `__intrinsic_sockets_connect` were removed when T3 TCP
/// connect was implemented (issue 447).  The array is kept empty so the
/// infrastructure is available for future host-stub additions.
const HOST_STUB_BUILTINS: &[&str] = &[];

/// Scan MIR for calls to any of the given builtin names.
fn mir_uses_capability(mir: &MirModule, builtins: &[&str]) -> bool {
    for func in &mir.functions {
        // Check function name itself (stdlib wrappers like monotonic_now)
        if builtins.contains(&func.name.as_str()) {
            return true;
        }
        for block in &func.blocks {
            for stmt in &block.stmts {
                if stmt_uses_capability(stmt, mir, builtins) {
                    return true;
                }
            }
        }
    }
    false
}

fn stmt_uses_capability(stmt: &MirStmt, mir: &MirModule, builtins: &[&str]) -> bool {
    match stmt {
        MirStmt::CallBuiltin { name, .. } => builtins.contains(&name.as_str()),
        MirStmt::Call { func, .. } => {
            if let Some(f) = mir.functions.iter().find(|f| f.id == *func) {
                builtins.contains(&f.name.as_str())
            } else {
                false
            }
        }
        MirStmt::Assign(_, rvalue) => rvalue_uses_capability(rvalue, builtins),
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            operand_uses_capability(cond, builtins)
                || then_body
                    .iter()
                    .any(|s| stmt_uses_capability(s, mir, builtins))
                || else_body
                    .iter()
                    .any(|s| stmt_uses_capability(s, mir, builtins))
        }
        MirStmt::WhileStmt { cond, body } => {
            operand_uses_capability(cond, builtins)
                || body.iter().any(|s| stmt_uses_capability(s, mir, builtins))
        }
        MirStmt::Return(Some(op)) => operand_uses_capability(op, builtins),
        _ => false,
    }
}

fn rvalue_uses_capability(rvalue: &Rvalue, builtins: &[&str]) -> bool {
    match rvalue {
        Rvalue::Use(op) => operand_uses_capability(op, builtins),
        Rvalue::BinaryOp(_, l, r) => {
            operand_uses_capability(l, builtins) || operand_uses_capability(r, builtins)
        }
        Rvalue::UnaryOp(_, op) => operand_uses_capability(op, builtins),
        _ => false,
    }
}

fn operand_uses_capability(op: &Operand, builtins: &[&str]) -> bool {
    match op {
        Operand::Call(name, args) => {
            if builtins.contains(&name.as_str()) {
                return true;
            }
            args.iter().any(|a| operand_uses_capability(a, builtins))
        }
        _ => false,
    }
}

/// Compose multiple WebAssembly components into a single composed component.
///
/// Reads each input component's imports/exports, prints the dependency graph,
/// detects export conflicts, and invokes `wasm-tools component compose`.
pub(crate) fn cmd_compose(inputs: Vec<PathBuf>, output: PathBuf) {
    use ark_wasm::component::{WrapError, compose_components};

    if inputs.is_empty() {
        eprintln!("error: arukellt compose requires at least one component input");
        process::exit(1);
    }

    let input_paths: Vec<&std::path::Path> = inputs.iter().map(PathBuf::as_path).collect();

    match compose_components(&input_paths) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(&output, &bytes) {
                eprintln!("error: failed to write output {}: {}", output.display(), e);
                process::exit(1);
            }
            eprintln!(
                "[arukellt compose] composed {} components → {} ({} bytes)",
                inputs.len(),
                output.display(),
                bytes.len()
            );
        }
        Err(WrapError::ToolNotFound(msg)) => {
            eprintln!("error: {}", msg);
            eprintln!("hint: install wasm-tools with: cargo install wasm-tools");
            process::exit(1);
        }
        Err(WrapError::WasmTools(msg)) => {
            eprintln!("error: {}", msg);
            process::exit(1);
        }
        Err(WrapError::Io(msg)) => {
            eprintln!("error: {}", msg);
            process::exit(1);
        }
    }
}
