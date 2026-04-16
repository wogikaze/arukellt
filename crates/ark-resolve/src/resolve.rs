//! Public resolve facade and compatibility wrappers.

use std::path::{Path, PathBuf};

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_lexer::Lexer;
use ark_parser::ast;
use ark_parser::parse;
use ark_target::TargetId;

use crate::analyze::{analyze_module, analyze_program};
use crate::bind::{bind_module, bind_public_module, inject_prelude_symbols};
use crate::load::{load_program, load_program_with_target, load_program_with_target_and_parser};
use crate::scope::{ScopeId, SymbolTable};

/// Options for multi-module crate resolution (`resolve_program*`,
/// `analyze_loaded_program*`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResolveCrateOptions {
    /// When set, only definitions reachable from program entrypoints are bound
    /// into the symbol table (conservative call graph in `reachability.rs`).
    pub lazy_reachability: bool,
}

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
    /// Qualifier names of loaded modules (e.g. "string", "text").
    /// Used by the type checker to distinguish "module not found" from
    /// "symbol not found in module".
    pub loaded_module_names: std::collections::HashSet<String>,
    /// Re-exported fn aliases from `pub use source::item` represented as
    /// `qualified_exported_name -> source_plain_fn_name`.
    pub pub_use_reexport_fn_aliases: std::collections::HashMap<String, String>,
    /// Qualified names from non-`pub` item imports (`use source::item`) that
    /// must not resolve via plain-name fallback.
    pub nonpub_item_import_blocked_qualified: std::collections::HashSet<String>,
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

pub fn load_program_graph_with_target(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
) -> Result<crate::module_graph::ModuleGraph, String> {
    load_program_with_target(entry_path, sink, target)
}

pub fn load_program_graph_with_target_and_parser<F>(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
    parse_module: &mut F,
) -> Result<crate::module_graph::ModuleGraph, String>
where
    F: FnMut(&Path, &mut DiagnosticSink) -> Result<ast::Module, String>,
{
    load_program_with_target_and_parser(entry_path, sink, target, parse_module)
}

// Convenience wrapper; the driver uses `analyze_loaded_program_with_options` directly.
#[allow(dead_code)]
pub fn analyze_loaded_program(
    graph: crate::module_graph::ModuleGraph,
    sink: &mut DiagnosticSink,
) -> ResolvedProgram {
    analyze_loaded_program_with_options(graph, sink, ResolveCrateOptions::default())
}

pub fn analyze_loaded_program_with_options(
    graph: crate::module_graph::ModuleGraph,
    sink: &mut DiagnosticSink,
    options: ResolveCrateOptions,
) -> ResolvedProgram {
    analyze_program(graph, sink, options)
}

pub fn resolve_bound_program(program: ResolvedProgram) -> ResolvedProgram {
    program
}

pub fn resolve_program(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ResolvedProgram, String> {
    resolve_program_with_crate_options(entry_path, sink, ResolveCrateOptions::default())
}

pub fn resolve_program_with_crate_options(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    options: ResolveCrateOptions,
) -> Result<ResolvedProgram, String> {
    let graph = load_program_graph(entry_path, sink)?;
    Ok(resolve_bound_program(analyze_loaded_program_with_options(
        graph, sink, options,
    )))
}

pub fn resolve_program_with_target(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
) -> Result<ResolvedProgram, String> {
    resolve_program_with_target_and_crate_options(
        entry_path,
        sink,
        target,
        ResolveCrateOptions::default(),
    )
}

pub fn resolve_program_with_target_and_crate_options(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
    options: ResolveCrateOptions,
) -> Result<ResolvedProgram, String> {
    let graph = load_program_graph_with_target(entry_path, sink, target)?;
    Ok(resolve_bound_program(analyze_loaded_program_with_options(
        graph, sink, options,
    )))
}

pub fn resolve_program_with_target_and_parser<F>(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
    parse_module: &mut F,
) -> Result<ResolvedProgram, String>
where
    F: FnMut(&Path, &mut DiagnosticSink) -> Result<ast::Module, String>,
{
    resolve_program_with_target_and_parser_and_crate_options(
        entry_path,
        sink,
        target,
        parse_module,
        ResolveCrateOptions::default(),
    )
}

pub fn resolve_program_with_target_and_parser_and_crate_options<F>(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
    parse_module: &mut F,
    options: ResolveCrateOptions,
) -> Result<ResolvedProgram, String>
where
    F: FnMut(&Path, &mut DiagnosticSink) -> Result<ast::Module, String>,
{
    let graph = load_program_graph_with_target_and_parser(entry_path, sink, target, parse_module)?;
    Ok(resolve_bound_program(analyze_loaded_program_with_options(
        graph, sink, options,
    )))
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

    // Collect loaded module qualifier names.
    let loaded_module_names: std::collections::HashSet<String> =
        program.modules.iter().map(|m| m.name.clone()).collect();

    let loaded_names: std::collections::HashSet<&str> =
        program.modules.iter().map(|m| m.name.as_str()).collect();
    let loaded_by_name: std::collections::HashMap<&str, &LoadedModule> = program
        .modules
        .iter()
        .map(|m| (m.name.as_str(), m))
        .collect();

    let mut pub_use_reexport_fn_aliases = std::collections::HashMap::new();
    let mut nonpub_item_import_blocked_qualified = std::collections::HashSet::new();

    for module in &program.modules {
        for import in &module.ast.imports {
            let Some((source_module, source_item)) = import.module_name.rsplit_once("::") else {
                continue;
            };

            let import_leaf = import
                .alias
                .as_deref()
                .unwrap_or_else(|| import.module_name.rsplit("::").next().unwrap_or(""));
            let is_module_import_shape = loaded_names.contains(import_leaf);

            match import.kind {
                ast::ImportKind::PublicModulePath if !is_module_import_shape => {
                    let Some(source_loaded) = loaded_by_name.get(source_module) else {
                        continue;
                    };

                    let is_pub_fn = source_loaded.ast.items.iter().any(|item| {
                        matches!(
                            item,
                            ast::Item::FnDef(f) if f.is_pub && f.name == source_item
                        )
                    });
                    if is_pub_fn {
                        let exported_name = import.alias.as_deref().unwrap_or(source_item);
                        pub_use_reexport_fn_aliases.insert(
                            format!("{}::{}", module.name, exported_name),
                            source_item.to_string(),
                        );
                    }
                }
                ast::ImportKind::ModulePath if !is_module_import_shape => {
                    let exported_name = import.alias.as_deref().unwrap_or(source_item);
                    nonpub_item_import_blocked_qualified
                        .insert(format!("{}::{}", module.name, exported_name));
                }
                _ => {}
            }
        }
    }

    ResolvedModule {
        module: resolved_program_to_module(&program),
        symbols: program.symbols,
        global_scope: program.global_scope,
        private_imported_names,
        entry_fn_names,
        loaded_module_names,
        pub_use_reexport_fn_aliases,
        nonpub_item_import_blocked_qualified,
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

pub fn resolve_program_entry_with_target(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
    target: Option<TargetId>,
) -> Result<ResolvedModule, String> {
    let mut program = resolve_program_with_target(entry_path, sink, target)?;
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

/// Inject WIT-imported function names as [`ExternWitFn`] symbols into a scope.
///
/// For v2, only the function name is registered in the symbol table — no full
/// type signature is stored. This allows the type-checker to accept calls to
/// WIT-imported names without failing on "undefined symbol" errors.
///
/// The injection is idempotent: names already present in the scope are skipped.
pub fn inject_wit_externs(
    table: &mut crate::scope::SymbolTable,
    scope: crate::scope::ScopeId,
    names: &[&str],
) {
    for &name in names {
        // Skip if already defined (handles duplicate WIT file / double-inject).
        if table.lookup_local(scope, name).is_some() {
            continue;
        }
        table.define(
            scope,
            name.to_string(),
            crate::scope::SymbolKind::ExternWitFn {
                name: name.to_string(),
            },
            ark_diagnostics::Span::dummy(),
        );
    }
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
