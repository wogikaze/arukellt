use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_parser::ast;

use crate::scope::{ScopeId, SymbolKind, SymbolTable};

pub(crate) fn bind_module(
    module: &ast::Module,
    symbols: &mut SymbolTable,
    global_scope: ScopeId,
    sink: &mut DiagnosticSink,
) {
    bind_module_impl(module, symbols, global_scope, sink, false, false);
}

pub(crate) fn bind_public_module(
    module: &ast::Module,
    symbols: &mut SymbolTable,
    global_scope: ScopeId,
    sink: &mut DiagnosticSink,
) {
    bind_module_impl(module, symbols, global_scope, sink, true, true);
}

pub(crate) fn bind_module_skip_dup(
    module: &ast::Module,
    symbols: &mut SymbolTable,
    global_scope: ScopeId,
    sink: &mut DiagnosticSink,
) {
    bind_module_impl(module, symbols, global_scope, sink, false, true);
}

/// Bind all items (pub + private) from an imported user module, skipping
/// duplicate definitions when `skip_duplicates` is true.
fn bind_module_impl(
    module: &ast::Module,
    symbols: &mut SymbolTable,
    global_scope: ScopeId,
    sink: &mut DiagnosticSink,
    pub_only: bool,
    skip_duplicates: bool,
) {
    for item in &module.items {
        match item {
            ast::Item::FnDef(f) => {
                if pub_only && !f.is_pub {
                    continue;
                }
                if symbols.lookup_local(global_scope, &f.name).is_some() {
                    if !skip_duplicates {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0101).with_label(
                                f.span,
                                format!("duplicate definition of `{}`", f.name),
                            ),
                        );
                    }
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
                if pub_only && !s.is_pub {
                    continue;
                }
                if symbols.lookup_local(global_scope, &s.name).is_some() {
                    if !skip_duplicates {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0101).with_label(
                                s.span,
                                format!("duplicate definition of `{}`", s.name),
                            ),
                        );
                    }
                } else {
                    symbols.define(
                        global_scope,
                        s.name.clone(),
                        SymbolKind::Struct { is_pub: s.is_pub },
                        s.span,
                    );
                    let vec_new_name = format!("Vec_new_{}", s.name);
                    symbols.define(global_scope, vec_new_name, SymbolKind::BuiltinFn, s.span);
                }
            }
            ast::Item::EnumDef(e) => {
                if pub_only && !e.is_pub {
                    continue;
                }
                if symbols.lookup_local(global_scope, &e.name).is_some() {
                    if !skip_duplicates {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0101).with_label(
                                e.span,
                                format!("duplicate definition of `{}`", e.name),
                            ),
                        );
                    }
                } else {
                    symbols.define(
                        global_scope,
                        e.name.clone(),
                        SymbolKind::Enum { is_pub: e.is_pub },
                        e.span,
                    );
                    for variant in &e.variants {
                        let vname = match variant {
                            ast::Variant::Unit { name, .. }
                            | ast::Variant::Tuple { name, .. }
                            | ast::Variant::Struct { name, .. } => name.clone(),
                        };
                        let qualified = format!("{}::{}", e.name, vname);
                        symbols.define(
                            global_scope,
                            qualified,
                            SymbolKind::EnumVariant {
                                enum_name: e.name.clone(),
                            },
                            e.span,
                        );
                    }
                }
            }
            ast::Item::TraitDef(t) => {
                if pub_only && !t.is_pub {
                    continue;
                }
                symbols.define(
                    global_scope,
                    t.name.clone(),
                    SymbolKind::Struct { is_pub: t.is_pub },
                    t.span,
                );
            }
            ast::Item::ImplBlock(ib) => {
                for method in &ib.methods {
                    let mangled = format!("{}__{}", ib.target_type, method.name);
                    symbols.define(
                        global_scope,
                        mangled,
                        SymbolKind::Function { is_pub: false },
                        method.span,
                    );
                }
            }
        }
    }
}

pub(crate) fn inject_prelude_symbols(symbols: &mut SymbolTable, scope: ScopeId) {
    const PRELUDE_TYPES: &[&str] = &["Option", "Result", "String", "Vec"];
    const PRELUDE_VALUES: &[&str] = &["Some", "None", "Ok", "Err", "true", "false"];
    const PRELUDE_FUNCTIONS: &[&str] = &[
        "len",
        "unwrap",
        "unwrap_or",
        "unwrap_or_else",
        "push",
        "pop",
        "get",
        "get_unchecked",
        "set",
        "is_empty",
        "clear",
        "is_some",
        "is_none",
        "is_ok",
        "is_err",
        "Vec_with_capacity_i32",
        "Vec_with_capacity_String",
        "as_slice",
        "ok_or",
        "ok",
        "err",
        "expect",
        "map_option_String_String",
        "map_result_i32_i32",
        "Box_new",
        "unbox",
        "to_string",
        "__intrinsic_println",
        "__intrinsic_print",
        "__intrinsic_eprintln",
        "__intrinsic_string_from",
        "__intrinsic_string_eq",
        "__intrinsic_string_new",
        "__intrinsic_concat",
        "__intrinsic_string_clone",
        "__intrinsic_starts_with",
        "__intrinsic_ends_with",
        "__intrinsic_to_lower",
        "__intrinsic_to_upper",
        "__intrinsic_string_slice",
        "__intrinsic_string_is_empty",
        "__intrinsic_i32_to_string",
        "__intrinsic_i64_to_string",
        "__intrinsic_f64_to_string",
        "__intrinsic_bool_to_string",
        "__intrinsic_char_to_string",
        "__intrinsic_parse_i32",
        "__intrinsic_parse_i64",
        "__intrinsic_parse_f64",
        "__intrinsic_sqrt",
        "__intrinsic_abs",
        "__intrinsic_min",
        "__intrinsic_max",
        "__intrinsic_panic",
        "__intrinsic_Vec_new_i32",
        "__intrinsic_Vec_new_i64",
        "__intrinsic_Vec_new_f64",
        "__intrinsic_Vec_new_String",
        "__intrinsic_sort_i32",
        "__intrinsic_sort_String",
        "__intrinsic_sort_i64",
        "__intrinsic_sort_f64",
        "__intrinsic_map_i32_i32",
        "__intrinsic_filter_i32",
        "__intrinsic_fold_i32_i32",
        "__intrinsic_map_option_i32_i32",
        "__intrinsic_any_i32",
        "__intrinsic_find_i32",
        "__intrinsic_split",
        "__intrinsic_join",
        "__intrinsic_push_char",
        "__intrinsic_fs_read_file",
        "__intrinsic_fs_write_file",
        "__intrinsic_memory_copy",
        "__intrinsic_memory_fill",
        "__intrinsic_clock_now",
        "__intrinsic_random_i32",
        "__intrinsic_map_String_String",
        "__intrinsic_filter_String",
        "__intrinsic_assert",
        "__intrinsic_assert_eq",
        "__intrinsic_assert_ne",
        "__intrinsic_assert_eq_i64",
        "__intrinsic_assert_eq_str",
        "__intrinsic_map_i64_i64",
        "__intrinsic_filter_i64",
        "__intrinsic_fold_i64_i64",
        "__intrinsic_map_f64_f64",
        "__intrinsic_filter_f64",
        "__intrinsic_contains_i32",
        "__intrinsic_contains_String",
        "__intrinsic_reverse_i32",
        "__intrinsic_reverse_String",
        "__intrinsic_remove_i32",
        "__intrinsic_args",
        "__intrinsic_arg_count",
        "__intrinsic_arg_at",
        "HashMap_i32_i32_new",
        "HashMap_i32_i32_insert",
        "HashMap_i32_i32_get",
        "HashMap_i32_i32_contains_key",
        "HashMap_i32_i32_len",
    ];

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_detects_duplicate_defs() {
        let module = ast::Module {
            docs: vec![],
            imports: vec![],
            items: vec![
                ast::Item::FnDef(ast::FnDef {
                    docs: vec![],
                    name: "foo".into(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![],
                    return_type: None,
                    body: ast::Block {
                        stmts: vec![],
                        tail_expr: None,
                        span: Span::dummy(),
                    },
                    is_pub: true,
                    span: Span::dummy(),
                }),
                ast::Item::FnDef(ast::FnDef {
                    docs: vec![],
                    name: "foo".into(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![],
                    return_type: None,
                    body: ast::Block {
                        stmts: vec![],
                        tail_expr: None,
                        span: Span::dummy(),
                    },
                    is_pub: true,
                    span: Span::dummy(),
                }),
            ],
        };
        let mut symbols = SymbolTable::new();
        let scope = symbols.create_scope(None);
        let mut sink = DiagnosticSink::new();
        bind_module(&module, &mut symbols, scope, &mut sink);
        assert!(sink.has_errors());
    }
}
