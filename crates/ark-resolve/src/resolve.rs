//! Public resolve facade and compatibility wrappers.

use std::path::{Path, PathBuf};

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_lexer::Lexer;
use ark_parser::ast;
use ark_parser::parse;

use crate::analyze::{analyze_module, analyze_program};
use crate::bind::{bind_module, bind_public_module, inject_prelude_symbols};
use crate::load::load_program;
use crate::scope::{ScopeId, SymbolTable};

/// Visibility of a declaration within its module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone)]
pub struct ResolvedFunction {
    pub name: String,
    pub visibility: Visibility,
}

#[derive(Debug, Clone)]
pub struct ResolvedStruct {
    pub name: String,
    pub visibility: Visibility,
}

#[derive(Debug, Clone)]
pub struct ResolvedEnum {
    pub name: String,
    pub visibility: Visibility,
}

/// Result of name resolution: resolved module + symbol table.
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub module: ast::Module,
    pub symbols: SymbolTable,
    pub global_scope: ScopeId,
    /// Names of private (non-pub) functions/types from imported (non-entry) modules.
    /// Used by the type checker to enforce cross-module privacy in qualified name lookups.
    pub private_imported_names: std::collections::HashSet<String>,
    /// Function/method names defined in the entry module (not imported).
    /// Used to scope visibility enforcement to only entry-module code.
    pub entry_fn_names: std::collections::HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub name: String,
    pub path: PathBuf,
    pub ast: ast::Module,
}

#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub entry_module: ast::Module,
    pub modules: Vec<LoadedModule>,
    pub symbols: SymbolTable,
    pub global_scope: ScopeId,
}

pub fn bind_program(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ResolvedProgram, String> {
    resolve_program(entry_path, sink)
}

pub fn load_program_graph(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<crate::module_graph::ModuleGraph, String> {
    load_program(entry_path, sink)
}

pub fn analyze_loaded_program(
    graph: crate::module_graph::ModuleGraph,
    sink: &mut DiagnosticSink,
) -> ResolvedProgram {
    analyze_program(graph, sink)
}

pub fn resolve_bound_program(program: ResolvedProgram) -> ResolvedProgram {
    program
}

pub fn resolve_program(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ResolvedProgram, String> {
    let graph = load_program_graph(entry_path, sink)?;
    Ok(resolve_bound_program(analyze_loaded_program(graph, sink)))
}

/// Resolve names in a parsed module.
pub fn resolve_module(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    analyze_module(module, sink)
}

#[deprecated(note = "use ResolvedProgram directly; flatten merge loses module identity")]
pub fn resolved_program_to_module(program: &ResolvedProgram) -> ast::Module {
    let mut module = program.entry_module.clone();
    // Track names already present (from entry module or earlier loaded modules)
    // so we can skip duplicate definitions (e.g. `Token` defined in both
    // lexer.ark and parser.ark).
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for item in &module.items {
        let name = item_name(item);
        if let Some(n) = name {
            seen_names.insert(n.to_string());
        }
    }
    for loaded in &program.modules {
        let is_stdlib = loaded.path.to_str().is_some_and(|p| p.starts_with('<'));
        for item in &loaded.ast.items {
            // For stdlib: only include pub items (strip is_pub flag).
            // For user-local: include ALL items so private helpers are available.
            if is_stdlib {
                let is_pub = match item {
                    ast::Item::FnDef(f) => f.is_pub,
                    ast::Item::StructDef(s) => s.is_pub,
                    ast::Item::EnumDef(e) => e.is_pub,
                    ast::Item::TraitDef(t) => t.is_pub,
                    ast::Item::ImplBlock(_) => false,
                };
                if !is_pub {
                    continue;
                }
                let name = item_name(item);
                if let Some(n) = name {
                    if seen_names.contains(n) {
                        continue;
                    }
                    seen_names.insert(n.to_string());
                }
                // Strip is_pub so stdlib items are not treated as user exports.
                let mut item = item.clone();
                match &mut item {
                    ast::Item::FnDef(f) => f.is_pub = false,
                    ast::Item::StructDef(s) => s.is_pub = false,
                    ast::Item::EnumDef(e) => e.is_pub = false,
                    ast::Item::TraitDef(t) => t.is_pub = false,
                    ast::Item::ImplBlock(_) => {}
                }
                module.items.push(item);
            } else {
                // User-local module: include all items, skip duplicates by name.
                let name = item_name(item);
                if let Some(n) = name {
                    if seen_names.contains(n) {
                        continue;
                    }
                    seen_names.insert(n.to_string());
                }
                module.items.push(item.clone());
            }
        }
    }
    module
}

fn item_name(item: &ast::Item) -> Option<&str> {
    match item {
        ast::Item::FnDef(f) => Some(&f.name),
        ast::Item::StructDef(s) => Some(&s.name),
        ast::Item::EnumDef(e) => Some(&e.name),
        ast::Item::TraitDef(t) => Some(&t.name),
        ast::Item::ImplBlock(_) => None,
    }
}

#[allow(deprecated)]
pub fn resolved_program_entry(program: ResolvedProgram) -> ResolvedModule {
    let mut private_imported_names = std::collections::HashSet::new();
    for loaded in &program.modules {
        let is_stdlib = loaded.path.to_str().is_some_and(|p| p.starts_with('<'));
        if is_stdlib {
            continue;
        }
        for item in &loaded.ast.items {
            let (name, is_pub) = match item {
                ast::Item::FnDef(f) => (f.name.as_str(), f.is_pub),
                ast::Item::StructDef(s) => (s.name.as_str(), s.is_pub),
                ast::Item::EnumDef(e) => (e.name.as_str(), e.is_pub),
                ast::Item::TraitDef(t) => (t.name.as_str(), t.is_pub),
                ast::Item::ImplBlock(_) => continue,
            };
            if !is_pub {
                private_imported_names.insert(name.to_string());
            }
        }
    }

    // Collect entry-module function names for visibility scoping.
    let mut entry_fn_names = std::collections::HashSet::new();
    for item in &program.entry_module.items {
        match item {
            ast::Item::FnDef(f) => {
                entry_fn_names.insert(f.name.clone());
            }
            ast::Item::ImplBlock(ib) => {
                for method in &ib.methods {
                    entry_fn_names.insert(method.name.clone());
                }
            }
            _ => {}
        }
    }

    ResolvedModule {
        module: resolved_program_to_module(&program),
        symbols: program.symbols,
        global_scope: program.global_scope,
        private_imported_names,
        entry_fn_names,
    }
}

fn parse_prelude_module(sink: &mut DiagnosticSink) -> ast::Module {
    if std::env::var("ARK_PRELUDE_FS").as_deref() == Ok("1") {
        let path = env!("ARK_PRELUDE_PATH");
        match std::fs::read_to_string(path) {
            Ok(src) => {
                let lexer = Lexer::new(0, &src);
                let tokens: Vec<_> = lexer.collect();
                return parse(&tokens, sink);
            }
            Err(e) => {
                sink.emit(
                    Diagnostic::new(DiagnosticCode::E0100)
                        .with_message(format!("failed to read prelude from filesystem: {}", e)),
                );
            }
        }
    }
    const PRELUDE_SRC: &str = include_str!(env!("ARK_PRELUDE_PATH"));
    let lexer = Lexer::new(0, PRELUDE_SRC);
    let tokens: Vec<_> = lexer.collect();
    parse(&tokens, sink)
}

pub fn merge_prelude(program: &mut ResolvedProgram, sink: &mut DiagnosticSink) {
    let prelude = parse_prelude_module(sink);
    // Use bind_public_module to silently skip symbols already defined
    // by user imports (prelude should not shadow explicit imports).
    bind_public_module(&prelude, &mut program.symbols, program.global_scope, sink);
    program.modules.push(LoadedModule {
        name: "std::prelude".into(),
        path: PathBuf::from("<prelude>"),
        ast: prelude,
    });
}

pub fn resolve_program_entry(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ResolvedModule, String> {
    let mut program = resolve_program(entry_path, sink)?;
    merge_prelude(&mut program, sink);
    Ok(resolved_program_entry(program))
}

pub fn resolve_module_with_intrinsic_prelude(
    module: ast::Module,
    sink: &mut DiagnosticSink,
) -> ResolvedModule {
    let mut symbols = SymbolTable::new();
    let global_scope = symbols.create_scope(None);
    inject_prelude_symbols(&mut symbols, global_scope);
    bind_module(&module, &mut symbols, global_scope, sink);
    let mut program = ResolvedProgram {
        entry_module: module,
        modules: vec![],
        symbols,
        global_scope,
    };
    merge_prelude(&mut program, sink);
    resolved_program_entry(program)
}

pub fn resolve_module_legacy(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    resolve_module_with_intrinsic_prelude(module, sink)
}

pub fn resolve_module_for_tests(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    resolve_module_with_intrinsic_prelude(module, sink)
}

pub fn resolve_module_default(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    resolve_module_with_intrinsic_prelude(module, sink)
}

pub fn resolve_module_public(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    resolve_module_with_intrinsic_prelude(module, sink)
}

pub fn resolve_module_stdlib(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    resolve_module_with_intrinsic_prelude(module, sink)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_diagnostics::Span;

    #[test]
    fn resolve_module_preserves_symbols() {
        let module = ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![ast::Item::StructDef(ast::StructDef {
                docs: vec![],
                name: "Point".into(),
                type_params: vec![],
                fields: vec![],
                is_pub: true,
                span: Span::dummy(),
            })],
        };
        let mut sink = DiagnosticSink::new();
        let resolved = resolve_module(module, &mut sink);
        assert!(
            resolved
                .symbols
                .lookup(resolved.global_scope, "Point")
                .is_some()
        );
    }
}
