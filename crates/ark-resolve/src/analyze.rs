use ark_diagnostics::DiagnosticSink;
use ark_parser::ast;

use crate::bind::{bind_module, bind_module_skip_dup, inject_prelude_symbols};
use crate::module_graph::ModuleGraph;
use crate::resolve::{ResolvedModule, ResolvedProgram};
use crate::scope::SymbolTable;

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
    }
    ResolvedProgram {
        entry_module: graph.entry_module,
        modules: graph.loaded.into_values().collect(),
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
