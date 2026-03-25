//! Module-level name resolution pass.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_parser::ast;

use crate::scope::{ScopeId, SymbolKind, SymbolTable};

/// Result of name resolution: resolved module + symbol table.
#[derive(Debug)]
pub struct ResolvedModule {
    pub module: ast::Module,
    pub symbols: SymbolTable,
    pub global_scope: ScopeId,
}

/// Prelude names automatically available in every module.
const PRELUDE_TYPES: &[&str] = &[
    "Option", "Result", "String", "Vec",
];

const PRELUDE_VALUES: &[&str] = &[
    "Some", "None", "Ok", "Err",
    "true", "false",
];

const PRELUDE_FUNCTIONS: &[&str] = &[
    "len", "clone", "unwrap", "unwrap_or", "unwrap_or_else",
    "panic", "println", "print", "eprintln",
    "sqrt", "abs", "min", "max",
    "push", "pop", "get", "set", "is_empty", "clear",
    "concat", "slice", "split", "join",
    "is_some", "is_none", "is_ok", "is_err",
    "i32_to_string", "i64_to_string", "f64_to_string",
    "parse_i32", "parse_i64", "parse_f64",
    "Vec_new_i32", "Vec_new_i64", "Vec_new_f64", "Vec_new_String",
    "Vec_with_capacity_i32", "Vec_with_capacity_String",
    "map_i32_i32", "map_String_String",
    "filter_i32", "filter_String",
    "fold_i32_i32",
    "sort_i32", "sort_i64", "sort_f64", "sort_String",
    "as_slice",
    "String_new", "String_from",
    "push_char", "to_lower", "to_upper",
    "starts_with", "ends_with",
    "ok_or", "ok", "err",
    "expect",
    "map_option_i32_i32", "map_option_String_String",
    "map_result_i32_i32",
];

/// Inject prelude symbols into the global scope.
fn inject_prelude(symbols: &mut SymbolTable, scope: ScopeId) {
    for &name in PRELUDE_TYPES {
        symbols.define(scope, name.into(), SymbolKind::BuiltinType, Span::dummy());
    }
    for &name in PRELUDE_VALUES {
        symbols.define(scope, name.into(), SymbolKind::BuiltinFn, Span::dummy());
    }
    for &name in PRELUDE_FUNCTIONS {
        symbols.define(scope, name.into(), SymbolKind::BuiltinFn, Span::dummy());
    }
}

/// Resolve names in a parsed module.
pub fn resolve_module(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    let mut symbols = SymbolTable::new();
    let global_scope = symbols.create_scope(None);

    // Inject prelude
    inject_prelude(&mut symbols, global_scope);

    // Collect top-level definitions
    for item in &module.items {
        match item {
            ast::Item::FnDef(f) => {
                if symbols.lookup_local(global_scope, &f.name).is_some() {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0101)
                            .with_label(f.span, format!("duplicate definition of `{}`", f.name)),
                    );
                } else {
                    symbols.define(
                        global_scope,
                        f.name.clone(),
                        SymbolKind::Function { is_pub: f.is_pub },
                        f.span,
                    );
                }
            }
            ast::Item::StructDef(s) => {
                if symbols.lookup_local(global_scope, &s.name).is_some() {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0101)
                            .with_label(s.span, format!("duplicate definition of `{}`", s.name)),
                    );
                } else {
                    symbols.define(
                        global_scope,
                        s.name.clone(),
                        SymbolKind::Struct { is_pub: s.is_pub },
                        s.span,
                    );
                }
            }
            ast::Item::EnumDef(e) => {
                if symbols.lookup_local(global_scope, &e.name).is_some() {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0101)
                            .with_label(e.span, format!("duplicate definition of `{}`", e.name)),
                    );
                } else {
                    symbols.define(
                        global_scope,
                        e.name.clone(),
                        SymbolKind::Enum { is_pub: e.is_pub },
                        e.span,
                    );
                    // Register variant constructors
                    for variant in &e.variants {
                        let vname = match variant {
                            ast::Variant::Unit { name, .. } => name.clone(),
                            ast::Variant::Tuple { name, .. } => name.clone(),
                            ast::Variant::Struct { name, .. } => name.clone(),
                        };
                        let qualified = format!("{}::{}", e.name, vname);
                        symbols.define(
                            global_scope,
                            qualified,
                            SymbolKind::EnumVariant { enum_name: e.name.clone() },
                            e.span,
                        );
                    }
                }
            }
        }
    }

    // TODO: resolve function bodies (identifiers in expressions)
    // TODO: handle imports
    // TODO: detect circular imports

    ResolvedModule { module, symbols, global_scope }
}
