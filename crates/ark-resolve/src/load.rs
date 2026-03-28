use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_lexer::Lexer;
use ark_parser::{ast, parse};

use crate::module_graph::ModuleGraph;
use crate::resolve::LoadedModule;

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

pub(crate) fn resolve_import_path(
    current_path: &Path,
    module_name: &str,
    std_root: &Path,
    sink: &mut DiagnosticSink,
) -> PathBuf {
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
        let parent = current_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
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

fn load_module_recursive(
    module_name: String,
    path: PathBuf,
    std_root: &Path,
    sink: &mut DiagnosticSink,
    visiting: &mut HashSet<PathBuf>,
    loaded: &mut HashMap<PathBuf, LoadedModule>,
) {
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

    let module = match parse_module_file(&path, sink) {
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
        let import_path = resolve_import_path(&path, &import.module_name, std_root, sink);
        let effective_name = import.alias.clone().unwrap_or_else(|| {
            import
                .module_name
                .rsplit("::")
                .next()
                .unwrap_or(&import.module_name)
                .to_string()
        });
        load_module_recursive(
            effective_name,
            import_path,
            std_root,
            sink,
            visiting,
            loaded,
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

pub(crate) fn load_program(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ModuleGraph, String> {
    let std_root = entry_path
        .ancestors()
        .find(|p| p.join("std").is_dir())
        .map(|p| p.join("std"))
        .unwrap_or_else(|| PathBuf::from("std"));

    let entry_module = parse_module_file(entry_path, sink)?;
    let mut visiting = HashSet::new();
    let mut loaded = HashMap::new();

    for import in &entry_module.imports {
        let import_path = resolve_import_path(entry_path, &import.module_name, &std_root, sink);
        let effective_name = import.alias.clone().unwrap_or_else(|| {
            // For `use std::text`, the effective name should be `text` (last segment)
            import
                .module_name
                .rsplit("::")
                .next()
                .unwrap_or(&import.module_name)
                .to_string()
        });
        load_module_recursive(
            effective_name,
            import_path,
            &std_root,
            sink,
            &mut visiting,
            &mut loaded,
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
        );
        assert!(path.ends_with("foo/bar.ark"));
    }
}
