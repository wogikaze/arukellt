use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_lexer::Lexer;
use ark_manifest::Manifest;
use ark_parser::{ast, ast::ImportKind, parse};
use ark_target::TargetId;

use crate::module_graph::ModuleGraph;
use crate::registry::{REGISTRY_ERROR_SENTINEL, RegistryConfig, resolve_registry_path};
use crate::resolve::LoadedModule;

// TODO(issue-077, issue-139): Target-gating for WASI P2-only modules is now
// implemented below via `T3_ONLY_MODULES`. When the active target is
// wasm32-wasi-p1 (T1), imports of `std::host::sockets` emit E0500 at resolve
// time because these modules require WASI Preview 2 host capabilities.
// The TargetId is threaded into load_program via resolve_program_with_target.

fn deprecated_std_import_replacement(module_name: &str) -> Option<(&'static str, &'static str)> {
    match module_name {
        "std::io" => Some((
            "std::host::stdio",
            "host standard I/O is no longer exposed as `std::io`",
        )),
        "std::fs" => Some((
            "std::host::fs",
            "host filesystem access is no longer exposed as `std::fs`",
        )),
        "std::env" => Some((
            "std::host::env",
            "host environment access is no longer exposed as `std::env`",
        )),
        "std::process" => Some((
            "std::host::process",
            "host process control is no longer exposed as `std::process`",
        )),
        _ => None,
    }
}

fn emit_deprecated_std_import(import: &ast::Import, sink: &mut DiagnosticSink) -> bool {
    let Some((replacement, note)) = deprecated_std_import_replacement(&import.module_name) else {
        return false;
    };

    let replacement_text = if let Some(alias) = &import.alias {
        format!("use {} as {}", replacement, alias)
    } else {
        format!("use {}", replacement)
    };

    sink.emit(
        Diagnostic::new(DiagnosticCode::E0104)
            .with_message(format!(
                "module `{}` has moved to `{}`",
                import.module_name, replacement
            ))
            .with_label(import.span, "deprecated std import")
            .with_fix_it(
                import.span,
                replacement_text,
                "replace the deprecated import",
            )
            .with_note(note),
    );
    true
}

fn emit_target_incompatible_import(
    import: &ast::Import,
    target: Option<TargetId>,
    sink: &mut DiagnosticSink,
) -> bool {
    if let Some(TargetId::Wasm32WasiP1) = target
        && T3_ONLY_MODULES.contains(&import.module_name.as_str())
    {
        sink.emit(
            Diagnostic::new(DiagnosticCode::E0500)
                .with_message(format!(
                    "module `{}` requires target wasm32-wasi-p2 (T3); \
                     use `--target wasm32-wasi-p2` to enable this module",
                    import.module_name
                ))
                .with_label(import.span, "requires T3 target"),
        );
        return true;
    }
    false
}

pub(crate) fn parse_module_file(
    path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ast::Module, String> {
    let source =
        std::fs::read_to_string(path).map_err(|e| format!("error: {}: {}", path.display(), e))?;
    let lexer = Lexer::new(0, &source);
    let tokens: Vec<_> = lexer.collect();
    Ok(parse(&tokens, sink))
}

/// Modules whose functions are all `host_stub` — always return errors at
/// runtime.  We reject imports of these at resolve time so the user gets a
/// clear compile-time error instead of a surprising runtime failure.
///
/// `std::host::sockets` was removed when T3 TCP connect was implemented
/// (issue 447).  The array is kept empty; T1 is still blocked by T3_ONLY_MODULES.
const HOST_STUB_MODULES: &[&str] = &[];

/// Modules that are only available on wasm32-wasi-p2 (T3) or later.
/// Importing these on wasm32-wasi-p1 (T1) emits E0500 (incompatible target).
///
/// `std::host::http` was removed from this list in issue 446: both T1 and T3
/// register `register_http_host_fns` in the Wasmtime linker, so the module is
/// available on both targets via TCP HTTP/1.1.
///
/// `std::host::udp` is T3-only because UDP datagrams require the WASI Preview 2
/// `wasi:sockets/udp` interface which is not available on wasm32-wasi-p1.
const T3_ONLY_MODULES: &[&str] = &["std::host::sockets", "std::host::udp"];

pub(crate) fn resolve_import_path(
    current_path: &Path,
    module_name: &str,
    std_root: &Path,
    sink: &mut DiagnosticSink,
    _target: Option<TargetId>,
) -> PathBuf {
    // Reject host_stub modules at import time
    if HOST_STUB_MODULES.contains(&module_name) {
        sink.emit(Diagnostic::new(DiagnosticCode::E0211).with_message(format!(
            "module `{}` is not yet implemented (host_stub); \
                 its functions always return errors at runtime",
            module_name,
        )));
        // Return a dummy path – the error above will prevent codegen
        return PathBuf::from("<host_stub>");
    }

    if module_name.starts_with("std") {
        let rel = module_name.replace("::", "/");
        let file_path = std_root.join(format!("{}.ark", rel));
        if file_path.exists() {
            return file_path;
        }
        // Fallback: try mod.ark inside directory
        let dir_path = std_root.join(&rel);
        let mod_path = dir_path.join("mod.ark");
        if mod_path.exists() {
            return mod_path;
        }
        // Strip leading "std/" and try directly under std_root
        let stripped = rel.strip_prefix("std/").unwrap_or(&rel);
        let stripped_file = std_root.join(format!("{}.ark", stripped));
        if stripped_file.exists() {
            return stripped_file;
        }
        let stripped_mod = std_root.join(stripped).join("mod.ark");
        if stripped_mod.exists() {
            return stripped_mod;
        }
        // Return the file path (will error at load time if not found)
        stripped_file
    } else {
        let rel = module_name.replace("::", "/");
        let parent = current_path.parent().unwrap_or_else(|| Path::new("."));
        let local_path = parent.join(format!("{}.ark", rel));
        let local_mod = parent.join(&rel).join("mod.ark");
        let std_path = std_root.join(format!("{}.ark", rel));
        let std_mod = std_root.join(&rel).join("mod.ark");

        // Determine effective local and std paths (prefer file, fallback to mod.ark)
        let effective_local = if local_path.exists() {
            Some(&local_path)
        } else if local_mod.exists() {
            Some(&local_mod)
        } else {
            None
        };
        let effective_std = if std_path.exists() {
            Some(&std_path)
        } else if std_mod.exists() {
            Some(&std_mod)
        } else {
            None
        };

        if effective_local.is_some() && effective_std.is_some() {
            sink.emit(Diagnostic::new(DiagnosticCode::W0003).with_message(format!(
                "ambiguous import `{}`: both local and std exist; using local",
                module_name,
            )));
        }

        if let Some(p) = effective_local {
            return p.clone();
        }
        if let Some(p) = effective_std {
            return p.clone();
        }
        local_path
    }
}

fn load_module_recursive<F>(
    module_name: String,
    path: PathBuf,
    std_root: &Path,
    sink: &mut DiagnosticSink,
    visiting: &mut HashSet<PathBuf>,
    loaded: &mut HashMap<PathBuf, LoadedModule>,
    target: Option<TargetId>,
    registry: Option<&RegistryConfig>,
    parse_module: &mut F,
) where
    F: FnMut(&Path, &mut DiagnosticSink) -> Result<ast::Module, String>,
{
    // Skip sentinel paths — errors were already emitted upstream.
    if path.as_os_str() == "<host_stub>"
        || path.as_os_str() == "<target-incompatible>"
        || path.as_os_str() == REGISTRY_ERROR_SENTINEL
    {
        return;
    }
    if loaded.contains_key(&path) {
        return;
    }

    if !visiting.insert(path.clone()) {
        let cycle: Vec<String> = visiting
            .iter()
            .map(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        sink.emit(Diagnostic::new(DiagnosticCode::E0103).with_message(format!(
            "circular import detected: {} → {}",
            cycle.join(" → "),
            path.file_name().unwrap_or_default().to_string_lossy()
        )));
        return;
    }

    let module = match parse_module(&path, sink) {
        Ok(module) => module,
        Err(msg) => {
            sink.emit(
                Diagnostic::new(DiagnosticCode::E0104)
                    .with_message(format!("module `{}` not found: {}", module_name, msg)),
            );
            visiting.remove(&path);
            return;
        }
    };

    for import in &module.imports {
        load_single_import(
            import,
            &path,
            std_root,
            sink,
            visiting,
            loaded,
            target,
            registry,
            parse_module,
        );
    }

    visiting.remove(&path);
    loaded.insert(
        path.clone(),
        LoadedModule {
            name: module_name,
            path,
            ast: module,
        },
    );
}

/// Resolve and recursively load one import declaration.
///
/// Handles all three import kinds:
/// - `Simple` / `ModulePath`: load the single module file.
/// - `DestructureImport { names }`: load each `base::name` sub-module separately.
fn load_single_import<F>(
    import: &ast::Import,
    current_path: &Path,
    std_root: &Path,
    sink: &mut DiagnosticSink,
    visiting: &mut HashSet<PathBuf>,
    loaded: &mut HashMap<PathBuf, LoadedModule>,
    target: Option<TargetId>,
    registry: Option<&RegistryConfig>,
    parse_module: &mut F,
) where
    F: FnMut(&Path, &mut DiagnosticSink) -> Result<ast::Module, String>,
{
    match &import.kind {
        ImportKind::DestructureImport { names } => {
            // `use a::b::{c, d}` → load `a::b::c` and `a::b::d` as separate modules.
            for name in names {
                let sub_module = format!("{}::{}", import.module_name, name);
                // Run the same deprecated / target-incompatible checks on the sub-path.
                // Build a synthetic import for the sub-module so the check helpers work.
                let sub_import = ast::Import {
                    module_name: sub_module.clone(),
                    alias: None,
                    kind: ImportKind::ModulePath,
                    span: import.span,
                };
                if emit_deprecated_std_import(&sub_import, sink) {
                    continue;
                }
                if emit_target_incompatible_import(&sub_import, target, sink) {
                    continue;
                }
                let import_path =
                    resolve_import_path(current_path, &sub_module, std_root, sink, target);
                load_module_recursive(
                    name.clone(),
                    import_path,
                    std_root,
                    sink,
                    visiting,
                    loaded,
                    target,
                    registry,
                    parse_module,
                );
            }
        }
        _ => {
            // Simple or ModulePath: load the single target module.
            if emit_deprecated_std_import(import, sink) {
                return;
            }
            if emit_target_incompatible_import(import, target, sink) {
                return;
            }
            let mut target_module_name = import.module_name.clone();
            let mut item_import_fallback = false;
            let mut import_path =
                resolve_import_path(current_path, &target_module_name, std_root, sink, target);

            // Registry fallback: if the path was not resolved locally or in
            // stdlib, and the module is a declared registry dependency, invoke
            // the registry resolver (ADR-023, E012x range).
            if !import_path.exists() {
                if let Some(reg) = registry {
                    if reg.is_registry_dep(&target_module_name) {
                        import_path =
                            resolve_registry_path(&target_module_name, reg, import.span, sink);
                    }
                }
            }

            // `use a::b::item` / `pub use a::b::item` item-import fallback:
            // if `a::b::item` is not a loadable module path, try loading `a::b`.
            if !import_path.exists()
                && let Some((parent_module, _item_name)) = target_module_name.rsplit_once("::")
            {
                let parent_path =
                    resolve_import_path(current_path, parent_module, std_root, sink, target);
                if parent_path.exists() {
                    target_module_name = parent_module.to_string();
                    import_path = parent_path;
                    item_import_fallback = true;
                }
            }

            let effective_name = if item_import_fallback {
                target_module_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&target_module_name)
                    .to_string()
            } else {
                import.alias.clone().unwrap_or_else(|| {
                    target_module_name
                        .rsplit("::")
                        .next()
                        .unwrap_or(&target_module_name)
                        .to_string()
                })
            };
            load_module_recursive(
                effective_name,
                import_path,
                std_root,
                sink,
                visiting,
                loaded,
                target,
                registry,
                parse_module,
            );
        }
    }
}

pub(crate) fn load_program(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ModuleGraph, String> {
    load_program_with_target(entry_path, sink, None)
}

pub(crate) fn load_program_with_target(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
) -> Result<ModuleGraph, String> {
    let mut parse_module = parse_module_file;
    load_program_with_target_and_parser(entry_path, sink, target, &mut parse_module)
}

pub(crate) fn load_program_with_target_and_parser<F>(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
    parse_module: &mut F,
) -> Result<ModuleGraph, String>
where
    F: FnMut(&Path, &mut DiagnosticSink) -> Result<ast::Module, String>,
{
    let std_root = entry_path
        .ancestors()
        .find(|p| p.join("std").is_dir())
        .map(|p| p.join("std"))
        .unwrap_or_else(|| PathBuf::from("std"));

    // Build registry config from the nearest ark.toml, if one exists.
    let registry_config: Option<RegistryConfig> = entry_path
        .parent()
        .and_then(|dir| Manifest::find_and_load(dir).ok())
        .map(|(root, manifest)| RegistryConfig::from_manifest(&manifest, &root));
    let registry = registry_config.as_ref();

    let entry_module = parse_module(entry_path, sink)?;
    let mut visiting = HashSet::new();
    let mut loaded = HashMap::new();

    for import in &entry_module.imports {
        load_single_import(
            import,
            entry_path,
            &std_root,
            sink,
            &mut visiting,
            &mut loaded,
            target,
            registry,
            parse_module,
        );
    }

    Ok(ModuleGraph {
        entry_module,
        loaded,
        _std_root: std_root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_import_prefers_local_path() {
        let mut sink = DiagnosticSink::new();
        let path = resolve_import_path(
            Path::new("/tmp/main.ark"),
            "foo::bar",
            Path::new("/std"),
            &mut sink,
            None,
        );
        assert!(path.ends_with("foo/bar.ark"));
    }

    #[test]
    fn resolve_destructure_sub_module_path() {
        // `use std::collections::{vec}` should resolve `std::collections::vec`
        // to `std/collections/vec.ark` relative to the std root.
        let mut sink = DiagnosticSink::new();
        let path = resolve_import_path(
            Path::new("/project/main.ark"),
            "std::collections::vec",
            Path::new("/project/std"),
            &mut sink,
            None,
        );
        // Path lands under std/collections/vec.ark (may not exist on disk in test,
        // but the resolved path should be correct).
        assert!(
            path.to_str().unwrap_or("").ends_with("collections/vec.ark"),
            "expected collections/vec.ark path, got {:?}",
            path
        );
    }
}
