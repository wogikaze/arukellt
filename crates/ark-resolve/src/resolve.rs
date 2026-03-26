//! Module-level name resolution pass.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_lexer::Lexer;
use ark_parser::ast;
use ark_parser::parse;

use crate::scope::{ScopeId, SymbolKind, SymbolTable};

/// Result of name resolution: resolved module + symbol table.
#[derive(Debug)]
pub struct ResolvedModule {
    pub module: ast::Module,
    pub symbols: SymbolTable,
    pub global_scope: ScopeId,
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

/// Prelude names automatically available in every module.
const PRELUDE_TYPES: &[&str] = &["Option", "Result", "String", "Vec"];

const PRELUDE_VALUES: &[&str] = &["Some", "None", "Ok", "Err", "true", "false"];

const PRELUDE_FUNCTIONS: &[&str] = &[
    // Builtins NOT wrapped in prelude.ark (remain as direct builtins)
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
    "split",
    "join",
    "is_some",
    "is_none",
    "is_ok",
    "is_err",
    "parse_i64",
    "parse_f64",
    "Vec_with_capacity_i32",
    "Vec_with_capacity_String",
    "map_String_String",
    "filter_String",
    "sort_i64",
    "sort_f64",
    "as_slice",
    "push_char",
    "ok_or",
    "ok",
    "err",
    "expect",
    "map_option_String_String",
    "map_result_i32_i32",
    "Box_new",
    "unbox",
    "to_string",
    // Existing __intrinsic_* names (called from prelude.ark or user code)
    "__intrinsic_println",
    "__intrinsic_print",
    "__intrinsic_eprintln",
    "__intrinsic_string_from",
    "__intrinsic_string_eq",
    // New __intrinsic_* names for functions now defined in prelude.ark
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
    "__intrinsic_map_i32_i32",
    "__intrinsic_filter_i32",
    "__intrinsic_fold_i32_i32",
    "__intrinsic_map_option_i32_i32",
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

fn collect_module_items(
    module: &ast::Module,
    symbols: &mut SymbolTable,
    global_scope: ScopeId,
    sink: &mut DiagnosticSink,
) {
    collect_module_items_impl(module, symbols, global_scope, sink, false);
}

fn collect_module_items_pub_only(
    module: &ast::Module,
    symbols: &mut SymbolTable,
    global_scope: ScopeId,
    sink: &mut DiagnosticSink,
) {
    collect_module_items_impl(module, symbols, global_scope, sink, true);
}

fn collect_module_items_impl(
    module: &ast::Module,
    symbols: &mut SymbolTable,
    global_scope: ScopeId,
    sink: &mut DiagnosticSink,
    pub_only: bool,
) {
    for item in &module.items {
        match item {
            ast::Item::FnDef(f) => {
                if pub_only && !f.is_pub {
                    continue;
                }
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
                if pub_only && !s.is_pub {
                    continue;
                }
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
                    let vec_new_name = format!("Vec_new_{}", s.name);
                    symbols.define(global_scope, vec_new_name, SymbolKind::BuiltinFn, s.span);
                }
            }
            ast::Item::EnumDef(e) => {
                if pub_only && !e.is_pub {
                    continue;
                }
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
                            SymbolKind::EnumVariant {
                                enum_name: e.name.clone(),
                            },
                            e.span,
                        );
                    }
                }
            }
        }
    }
}

fn parse_module_file(path: &Path, sink: &mut DiagnosticSink) -> Result<ast::Module, String> {
    let source =
        std::fs::read_to_string(path).map_err(|e| format!("error: {}: {}", path.display(), e))?;
    let lexer = Lexer::new(0, &source);
    let tokens: Vec<_> = lexer.collect();
    Ok(parse(&tokens, sink))
}

fn resolve_import_path(current_path: &Path, module_name: &str, std_root: &Path) -> PathBuf {
    if module_name.starts_with("std") {
        let rel = module_name.replace("::", "/");
        std_root.join(format!("{}.ark", rel))
    } else {
        let rel = module_name.replace("::", "/");
        current_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{}.ark", rel))
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
        sink.emit(
            Diagnostic::new(DiagnosticCode::E0100)
                .with_message(format!("circular import detected at `{}`", path.display())),
        );
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
        let import_path = resolve_import_path(&path, &import.module_name, std_root);
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

pub fn resolve_program(
    entry_path: &Path,
    sink: &mut DiagnosticSink,
) -> Result<ResolvedProgram, String> {
    let std_root = entry_path
        .ancestors()
        .find(|p| p.join("std").is_dir())
        .map(|p| p.join("std"))
        .unwrap_or_else(|| PathBuf::from("std"));

    let entry_module = parse_module_file(entry_path, sink)?;

    let mut visiting = HashSet::new();
    let mut loaded = HashMap::new();

    for import in &entry_module.imports {
        let import_path = resolve_import_path(entry_path, &import.module_name, &std_root);
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

    let mut symbols = SymbolTable::new();
    let global_scope = symbols.create_scope(None);
    inject_prelude(&mut symbols, global_scope);
    collect_module_items(&entry_module, &mut symbols, global_scope, sink);
    for loaded_module in loaded.values() {
        collect_module_items_pub_only(&loaded_module.ast, &mut symbols, global_scope, sink);
    }

    Ok(ResolvedProgram {
        entry_module,
        modules: loaded.into_values().collect(),
        symbols,
        global_scope,
    })
}

/// Resolve names in a parsed module.
pub fn resolve_module(module: ast::Module, sink: &mut DiagnosticSink) -> ResolvedModule {
    let mut symbols = SymbolTable::new();
    let global_scope = symbols.create_scope(None);
    inject_prelude(&mut symbols, global_scope);
    collect_module_items(&module, &mut symbols, global_scope, sink);
    ResolvedModule {
        module,
        symbols,
        global_scope,
    }
}

pub fn resolved_program_to_module(program: &ResolvedProgram) -> ast::Module {
    let mut module = program.entry_module.clone();
    for loaded in &program.modules {
        // Only include pub items from imported modules
        for item in &loaded.ast.items {
            let is_pub = match item {
                ast::Item::FnDef(f) => f.is_pub,
                ast::Item::StructDef(s) => s.is_pub,
                ast::Item::EnumDef(e) => e.is_pub,
            };
            if is_pub {
                module.items.push(item.clone());
            }
        }
    }
    module
}

pub fn resolved_program_entry(program: ResolvedProgram) -> ResolvedModule {
    ResolvedModule {
        module: resolved_program_to_module(&program),
        symbols: program.symbols,
        global_scope: program.global_scope,
    }
}

pub fn intrinsic_prelude_module() -> ast::Module {
    use ast::*;
    let dummy = Span::dummy();
    let mk_param = |name: &str, ty: TypeExpr| Param {
        name: name.into(),
        ty,
        span: dummy,
    };
    let named = |name: &str| TypeExpr::Named {
        name: name.into(),
        span: dummy,
    };
    let mk_call = |callee: &str, args: Vec<Expr>| Expr::Call {
        callee: Box::new(Expr::Ident {
            name: callee.into(),
            span: dummy,
        }),
        type_args: vec![],
        args,
        span: dummy,
    };
    let wrappers = vec![
        Item::FnDef(FnDef {
            name: "println".into(),
            type_params: vec![],
            params: vec![mk_param("s", named("String"))],
            return_type: None,
            body: Block {
                stmts: vec![Stmt::Expr(mk_call(
                    "__intrinsic_println",
                    vec![Expr::Ident {
                        name: "s".into(),
                        span: dummy,
                    }],
                ))],
                tail_expr: None,
                span: dummy,
            },
            is_pub: true,
            span: dummy,
        }),
        Item::FnDef(FnDef {
            name: "print".into(),
            type_params: vec![],
            params: vec![mk_param("s", named("String"))],
            return_type: None,
            body: Block {
                stmts: vec![Stmt::Expr(mk_call(
                    "__intrinsic_print",
                    vec![Expr::Ident {
                        name: "s".into(),
                        span: dummy,
                    }],
                ))],
                tail_expr: None,
                span: dummy,
            },
            is_pub: true,
            span: dummy,
        }),
        Item::FnDef(FnDef {
            name: "eprintln".into(),
            type_params: vec![],
            params: vec![mk_param("s", named("String"))],
            return_type: None,
            body: Block {
                stmts: vec![Stmt::Expr(mk_call(
                    "__intrinsic_eprintln",
                    vec![Expr::Ident {
                        name: "s".into(),
                        span: dummy,
                    }],
                ))],
                tail_expr: None,
                span: dummy,
            },
            is_pub: true,
            span: dummy,
        }),
        Item::FnDef(FnDef {
            name: "String_from".into(),
            type_params: vec![],
            params: vec![mk_param("s", named("String"))],
            return_type: Some(named("String")),
            body: Block {
                stmts: vec![],
                tail_expr: Some(Box::new(mk_call(
                    "__intrinsic_string_from",
                    vec![Expr::Ident {
                        name: "s".into(),
                        span: dummy,
                    }],
                ))),
                span: dummy,
            },
            is_pub: true,
            span: dummy,
        }),
        Item::FnDef(FnDef {
            name: "eq".into(),
            type_params: vec![],
            params: vec![
                mk_param("a", named("String")),
                mk_param("b", named("String")),
            ],
            return_type: Some(named("bool")),
            body: Block {
                stmts: vec![],
                tail_expr: Some(Box::new(mk_call(
                    "__intrinsic_string_eq",
                    vec![
                        Expr::Ident {
                            name: "a".into(),
                            span: dummy,
                        },
                        Expr::Ident {
                            name: "b".into(),
                            span: dummy,
                        },
                    ],
                ))),
                span: dummy,
            },
            is_pub: true,
            span: dummy,
        }),
    ];
    Module {
        imports: vec![],
        items: wrappers,
    }
}

fn parse_prelude_module(sink: &mut DiagnosticSink) -> ast::Module {
    const PRELUDE_SRC: &str = include_str!("../../../std/prelude.ark");
    let lexer = Lexer::new(0, PRELUDE_SRC);
    let tokens: Vec<_> = lexer.collect();
    parse(&tokens, sink)
}

pub fn merge_prelude(program: &mut ResolvedProgram, sink: &mut DiagnosticSink) {
    let prelude = parse_prelude_module(sink);
    collect_module_items(&prelude, &mut program.symbols, program.global_scope, sink);
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
    let mut program = ResolvedProgram {
        entry_module: module,
        modules: vec![],
        symbols: SymbolTable::new(),
        global_scope: ScopeId(0),
    };
    program.global_scope = program.symbols.create_scope(None);
    inject_prelude(&mut program.symbols, program.global_scope);
    collect_module_items(
        &program.entry_module,
        &mut program.symbols,
        program.global_scope,
        sink,
    );
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
