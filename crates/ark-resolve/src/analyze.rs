use ark_diagnostics::DiagnosticSink;
use ark_parser::ast;

use crate::bind::{
    bind_module, bind_module_skip_dup, bind_module_with_qualifier, inject_prelude_symbols,
};
use crate::module_graph::ModuleGraph;
use crate::resolve::{ResolvedModule, ResolvedProgram};
use crate::scope::{SymbolKind, SymbolTable};

fn split_item_import(import: &ast::Import) -> Option<(&str, &str)> {
    match import.kind {
        ast::ImportKind::ModulePath | ast::ImportKind::PublicModulePath => {
            import.module_name.rsplit_once("::")
        }
        _ => None,
    }
}

fn pub_item_symbol_kind(module: &ast::Module, item_name: &str) -> Option<SymbolKind> {
    for item in &module.items {
        match item {
            ast::Item::FnDef(f) if f.is_pub && f.name == item_name => {
                return Some(SymbolKind::Function { is_pub: true });
            }
            ast::Item::StructDef(s) if s.is_pub && s.name == item_name => {
                return Some(SymbolKind::Struct { is_pub: true });
            }
            ast::Item::EnumDef(e) if e.is_pub && e.name == item_name => {
                return Some(SymbolKind::Enum { is_pub: true });
            }
            _ => {}
        }
    }
    None
}

fn bind_pub_use_item_reexports(
    modules: &[crate::resolve::LoadedModule],
    symbols: &mut SymbolTable,
    global_scope: crate::scope::ScopeId,
) {
    let by_name: std::collections::HashMap<&str, &crate::resolve::LoadedModule> =
        modules.iter().map(|m| (m.name.as_str(), m)).collect();

    for module in modules {
        for import in &module.ast.imports {
            if !matches!(import.kind, ast::ImportKind::PublicModulePath) {
                continue;
            }

            let Some((source_module, source_item)) = split_item_import(import) else {
                continue;
            };

            let Some(source_loaded) = by_name.get(source_module) else {
                continue;
            };

            let Some(kind) = pub_item_symbol_kind(&source_loaded.ast, source_item) else {
                continue;
            };

            let exported_name = import.alias.as_deref().unwrap_or(source_item);
            let qualified = format!("{}::{}", module.name, exported_name);
            if symbols.lookup_local(global_scope, &qualified).is_none() {
                symbols.define(global_scope, qualified, kind, import.span);
            }
        }
    }
}

pub(crate) fn analyze_program(graph: ModuleGraph, sink: &mut DiagnosticSink) -> ResolvedProgram {
    let mut symbols = SymbolTable::new();
    let global_scope = symbols.create_scope(None);
    inject_prelude_symbols(&mut symbols, global_scope);
    bind_module(&graph.entry_module, &mut symbols, global_scope, sink);
    for loaded in graph.loaded.values() {
        // Issue 208: include ALL items (not just pub) from user-local modules,
        // skipping duplicates. This ensures private helpers called by pub fns
        // are visible in the merged module scope.
        bind_module_skip_dup(&loaded.ast, &mut symbols, global_scope, sink);
        // Also register pub items under the qualified name (e.g. `string::split`)
        // so the resolver symbol table records the full qualified form.
        // This covers slice 2 of issue #039 (module-qualified name resolution).
        bind_module_with_qualifier(&loaded.ast, &mut symbols, global_scope, &loaded.name, sink);
    }
    let modules: Vec<_> = graph.loaded.into_values().collect();

    // Register `pub use module::item` re-exports under the re-exporting
    // module qualifier (e.g. `api::split`) so importers can resolve them.
    bind_pub_use_item_reexports(&modules, &mut symbols, global_scope);

    ResolvedProgram {
        entry_module: graph.entry_module,
        modules,
        symbols,
        global_scope,
    }
}

pub(crate) fn analyze_module(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    let mut symbols = SymbolTable::new();
    let global_scope = symbols.create_scope(None);
    inject_prelude_symbols(&mut symbols, global_scope);
    bind_module(&module, &mut symbols, global_scope, sink);
    ResolvedModule {
        module,
        symbols,
        global_scope,
        private_imported_names: std::collections::HashSet::new(),
        entry_fn_names: std::collections::HashSet::new(),
        loaded_module_names: std::collections::HashSet::new(),
        pub_use_reexport_fn_aliases: std::collections::HashMap::new(),
        nonpub_item_import_blocked_qualified: std::collections::HashSet::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_diagnostics::Span;

    #[test]
    fn analyze_module_populates_symbols() {
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
        let resolved = analyze_module(module, &mut sink);
        assert!(
            resolved
                .symbols
                .lookup(resolved.global_scope, "Point")
                .is_some()
        );
    }
}
