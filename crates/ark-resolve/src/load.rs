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
        std_root.join(format!("{}.ark", rel))
    } else {
        let rel = module_name.replace("::", "/");
        let local_path = current_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{}.ark", rel));
        let std_path = std_root.join(format!("{}.ark", rel));
        if local_path.exists() && std_path.exists() {
            sink.emit(Diagnostic::new(DiagnosticCode::W0003).with_message(format!(
                "ambiguous import `{}`: both local `{}` and std `{}` exist; using local",
                module_name,
                local_path.display(),
                std_path.display()
            )));
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
            sink.emit(Diagnostic::new(DiagnosticCode::E0100).with_message(msg));
            visiting.remove(&path);
            return;
        }
    };

    for import in &module.imports {
        let import_path = resolve_import_path(&path, &import.module_name, std_root, sink);
        load_module_recursive(
            import
                .alias
                .clone()
                .unwrap_or_else(|| import.module_name.clone()),
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
        load_module_recursive(
            import
                .alias
                .clone()
                .unwrap_or_else(|| import.module_name.clone()),
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
