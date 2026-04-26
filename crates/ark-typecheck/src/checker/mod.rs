//! Type checker implementation with bidirectional inference.

mod builtins;
mod check_expr;
mod check_fn;
mod check_stmt;
mod infer;

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_parser::ast;
use ark_resolve::ResolvedModule;

use ark_hir::validate_program;

use crate::build_corehir::{CoreHirBundle, build_core_hir_program};
use crate::typed_ast::{NodeIdAllocator, TypedAstMap};
use crate::types::{Type, TypeId};
use std::collections::{HashMap, HashSet};

/// Type environment for tracking variable types.
#[derive(Debug)]
pub struct TypeEnv {
    pub(crate) bindings: HashMap<String, Type>,
    pub(crate) mutable_vars: HashSet<String>,
    pub(crate) parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            mutable_vars: HashSet::new(),
            parent: None,
        }
    }

    pub fn child(&self) -> Self {
        Self {
            bindings: HashMap::new(),
            mutable_vars: HashSet::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    pub fn bind(&mut self, name: String, ty: Type) {
        self.bindings.insert(name, ty);
    }

    pub fn bind_mut(&mut self, name: String, ty: Type) {
        self.bindings.insert(name.clone(), ty);
        self.mutable_vars.insert(name);
    }

    pub fn is_mutable(&self, name: &str) -> bool {
        self.mutable_vars.contains(name) || self.parent.as_ref().is_some_and(|p| p.is_mutable(name))
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }
}

impl Clone for TypeEnv {
    fn clone(&self) -> Self {
        Self {
            bindings: self.bindings.clone(),
            mutable_vars: self.mutable_vars.clone(),
            parent: self.parent.clone(),
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a struct definition.
#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<(String, Type)>,
    pub type_id: TypeId,
}

/// Information about an enum definition.
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<VariantInfo>,
    pub type_params: Vec<String>,
    pub type_id: TypeId,
}

/// Variant info.
#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<Type>,
    pub field_names: Vec<String>,
}

/// Information about a function signature.
#[derive(Debug, Clone)]
pub struct FnSig {
    pub name: String,
    pub type_params: Vec<String>,
    pub type_param_bounds: Vec<(String, Vec<String>)>,
    pub params: Vec<Type>,
    pub ret: Type,
}

#[derive(Debug, Clone)]
pub struct CheckOutput {
    pub core_hir: CoreHirBundle,
}

impl CheckOutput {
    pub fn program(&self) -> &ark_hir::Program {
        &self.core_hir.program
    }

    fn new(core_hir: CoreHirBundle) -> Self {
        Self { core_hir }
    }
}

/// The main type checker.
#[derive(Debug)]
pub struct TypeChecker {
    pub(crate) struct_defs: HashMap<String, StructInfo>,
    pub(crate) enum_defs: HashMap<String, EnumInfo>,
    pub(crate) fn_sigs: HashMap<String, FnSig>,
    /// Maps (struct_name, method_name) to the mangled function name
    pub(crate) method_table: HashMap<(String, String), String>,
    /// Trait definitions: trait_name -> list of (method_name, params_types, return_type)
    pub(crate) trait_defs: HashMap<String, Vec<(String, Vec<Type>, Type)>>,
    /// Maps call expression span start to (mangled_fn_name, self_type_name)
    pub(crate) method_resolutions: HashMap<u32, (String, String)>,
    /// Maps type_name -> set of implemented trait names
    pub(crate) trait_impls: HashMap<String, Vec<String>>,
    pub(crate) node_ids: NodeIdAllocator,
    pub(crate) typed_ast_map: TypedAstMap,
    pub(crate) latest_core_hir: Option<CoreHirBundle>,
    next_type_id: u32,
    next_type_var: u32,
    pub(crate) current_fn_return_type: Option<Type>,
    /// Names of private functions from imported (non-entry) modules.
    /// Used to block the `QualifiedIdent` fallback lookup for these names.
    pub(crate) private_imported_fns: HashSet<String>,
    /// True when checking a function body from the entry module (not an imported module).
    /// Used to scope visibility enforcement: private imports are only blocked for entry code.
    pub(crate) checking_entry_module: bool,
    /// Function names defined in the entry module (not imported). Used to determine
    /// when to enforce cross-module visibility.
    pub(crate) entry_fn_names: HashSet<String>,
    /// Module qualifier names that have been loaded via `use std::...`.
    /// Used to distinguish "module not found" from "symbol not found in module"
    /// when resolving `QualifiedIdent` expressions.
    pub(crate) known_modules: HashSet<String>,
    /// Qualified names that came from non-`pub` item imports
    /// (`use module::item`) and therefore must not resolve via plain-name fallback.
    pub(crate) blocked_nonpub_item_qualified: HashSet<String>,
}

/// Immutable semantic model produced by type checking.
/// Downstream consumers should prefer this over accessing TypeChecker directly.
#[derive(Debug)]
pub struct SemanticModel {
    pub struct_defs: HashMap<String, StructInfo>,
    pub enum_defs: HashMap<String, EnumInfo>,
    pub fn_sigs: HashMap<String, FnSig>,
    pub method_table: HashMap<(String, String), String>,
    pub trait_defs: HashMap<String, Vec<(String, Vec<Type>, Type)>>,
    pub method_resolutions: HashMap<u32, (String, String)>,
    pub trait_impls: HashMap<String, Vec<String>>,
    typed_ast: TypedAstMap,
    core_hir: Option<CoreHirBundle>,
}

impl SemanticModel {
    pub fn core_hir(&self) -> Option<&CoreHirBundle> {
        self.core_hir.as_ref()
    }
}

impl TypeChecker {
    pub fn register_extern_function(&mut self, name: String, params: Vec<Type>, ret: Type) {
        self.fn_sigs.insert(
            name.clone(),
            FnSig {
                name,
                type_params: vec![],
                type_param_bounds: vec![],
                params,
                ret,
            },
        );
    }

    pub fn latest_core_hir(&self) -> Option<&CoreHirBundle> {
        self.latest_core_hir.as_ref()
    }

    pub fn check_core_hir_module(
        &mut self,
        resolved: &ResolvedModule,
        sink: &mut DiagnosticSink,
    ) -> CheckOutput {
        self.check_module(resolved, sink);
        let bundle = build_core_hir_program(
            self,
            "main",
            &resolved.module.imports,
            &resolved.module.items,
        );
        if let Err(errors) = validate_program(&bundle.program) {
            for error in errors {
                sink.emit(
                    Diagnostic::new(DiagnosticCode::E0200)
                        .with_message(format!("invalid CoreHIR: {}", error.message)),
                );
            }
        }
        self.latest_core_hir = Some(bundle.clone());
        CheckOutput::new(bundle)
    }
}

impl SemanticModel {
    pub fn struct_info(&self, name: &str) -> Option<&StructInfo> {
        self.struct_defs.get(name)
    }

    pub fn enum_info(&self, name: &str) -> Option<&EnumInfo> {
        self.enum_defs.get(name)
    }

    pub fn fn_sig(&self, name: &str) -> Option<&FnSig> {
        self.fn_sigs.get(name)
    }

    pub fn fn_sigs_iter(&self) -> impl Iterator<Item = (&String, &FnSig)> {
        self.fn_sigs.iter()
    }

    pub fn method_resolution(&self, span_start: u32) -> Option<&(String, String)> {
        self.method_resolutions.get(&span_start)
    }

    pub fn method_resolutions_snapshot(&self) -> HashMap<u32, (String, String)> {
        self.method_resolutions.clone()
    }

    pub fn method_fn_name(&self, struct_name: &str, method_name: &str) -> Option<&String> {
        self.method_table
            .get(&(struct_name.to_string(), method_name.to_string()))
    }

    pub fn typed_ast(&self) -> &TypedAstMap {
        &self.typed_ast
    }
}

impl TypeChecker {
    // -- Immutable accessors for downstream consumers --

    pub fn struct_info(&self, name: &str) -> Option<&StructInfo> {
        self.struct_defs.get(name)
    }

    pub fn enum_info(&self, name: &str) -> Option<&EnumInfo> {
        self.enum_defs.get(name)
    }

    pub fn fn_sig(&self, name: &str) -> Option<&FnSig> {
        self.fn_sigs.get(name)
    }

    pub fn fn_sigs_iter(&self) -> impl Iterator<Item = (&String, &FnSig)> {
        self.fn_sigs.iter()
    }

    pub fn struct_defs_iter(&self) -> impl Iterator<Item = (&String, &StructInfo)> {
        self.struct_defs.iter()
    }

    pub fn enum_defs_iter(&self) -> impl Iterator<Item = (&String, &EnumInfo)> {
        self.enum_defs.iter()
    }

    pub fn method_resolution(&self, span_start: u32) -> Option<&(String, String)> {
        self.method_resolutions.get(&span_start)
    }

    pub fn method_resolutions_snapshot(&self) -> HashMap<u32, (String, String)> {
        self.method_resolutions.clone()
    }

    pub fn method_fn_name(&self, struct_name: &str, method_name: &str) -> Option<&String> {
        self.method_table
            .get(&(struct_name.to_string(), method_name.to_string()))
    }

    /// Consume the checker and produce an immutable semantic model.
    pub fn into_semantic_model(self) -> SemanticModel {
        SemanticModel {
            struct_defs: self.struct_defs,
            enum_defs: self.enum_defs,
            fn_sigs: self.fn_sigs,
            method_table: self.method_table,
            trait_defs: self.trait_defs,
            method_resolutions: self.method_resolutions,
            trait_impls: self.trait_impls,
            typed_ast: self.typed_ast_map,
            core_hir: self.latest_core_hir,
        }
    }

    pub fn new() -> Self {
        Self {
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            fn_sigs: HashMap::new(),
            method_table: HashMap::new(),
            trait_defs: HashMap::new(),
            method_resolutions: HashMap::new(),
            trait_impls: HashMap::new(),
            node_ids: NodeIdAllocator::new(),
            typed_ast_map: TypedAstMap::new(),
            latest_core_hir: None,
            next_type_id: 0,
            next_type_var: 0,
            current_fn_return_type: None,
            private_imported_fns: HashSet::new(),
            checking_entry_module: false,
            entry_fn_names: HashSet::new(),
            known_modules: HashSet::new(),
            blocked_nonpub_item_qualified: HashSet::new(),
        }
    }

    pub(crate) fn fresh_type_id(&mut self) -> TypeId {
        let id = TypeId(self.next_type_id);
        self.next_type_id += 1;
        id
    }

    pub(crate) fn fresh_type_var(&mut self) -> Type {
        let id = self.next_type_var;
        self.next_type_var += 1;
        Type::TypeVar(id)
    }

    /// Type check a multi-module program.
    ///
    /// Accepts a `ResolvedProgram` preserving module boundaries. The current
    /// implementation still flattens internally via `resolved_program_to_module`,
    /// but the public API contract is program-aware so downstream callers migrate
    /// away from the flattened `ResolvedModule` representation.
    pub fn check_program(
        &mut self,
        program: &ark_resolve::ResolvedProgram,
        sink: &mut DiagnosticSink,
    ) {
        // Collect entry-module function names so visibility checks only apply
        // to code in the user's entry module, not inside imported modules.
        for item in &program.entry_module.items {
            if let ast::Item::FnDef(f) = item {
                self.entry_fn_names.insert(f.name.clone());
            }
            if let ast::Item::ImplBlock(ib) = item {
                for method in &ib.methods {
                    self.entry_fn_names.insert(method.name.clone());
                }
            }
        }

        // Visibility diagnostic: detect references to private symbols across
        // module boundaries.
        self.check_cross_module_visibility(program, sink);

        // Register qualified fn signatures (e.g. `string::split`) so that
        // the primary lookup path in QualifiedIdent type-checking resolves
        // without relying solely on the plain-name fallback.
        // This is the slice-3 (#039) typecheck fix: the resolver already
        // inserts qualified names into the symbol table; here we mirror that
        // in the fn_sigs type table so type inference sees the canonical key.
        self.register_qualified_module_sigs(&program.modules);
        self.register_nonpub_item_import_fallback_blocks(&program.modules);
        self.register_pub_use_reexport_sigs(&program.modules);

        #[allow(deprecated)]
        let flat = ark_resolve::resolved_program_to_module(program);
        let resolved = ark_resolve::ResolvedModule {
            module: flat,
            symbols: program.symbols.clone(),
            global_scope: program.global_scope,
            private_imported_names: self.private_imported_fns.clone(),
            entry_fn_names: self.entry_fn_names.clone(),
            loaded_module_names: self.known_modules.clone(),
            pub_use_reexport_fn_aliases: std::collections::HashMap::new(),
            nonpub_item_import_blocked_qualified: self.blocked_nonpub_item_qualified.clone(),
        };
        self.check_module(&resolved, sink);
    }

    /// For each loaded module, register every `pub fn` under
    /// `qualifier::fn_name` in `fn_sigs`.  The qualifier is the module's
    /// leaf name (e.g. `"string"` for `std/text/string.ark`).
    ///
    /// This ensures that `QualifiedIdent { module: "string", name: "split" }`
    /// resolves via the primary key `"string::split"` rather than relying on
    /// the plain-name fallback, which can shadow or be shadowed by same-named
    /// functions in other modules.
    fn register_qualified_module_sigs(&mut self, modules: &[ark_resolve::LoadedModule]) {
        for loaded in modules {
            let qualifier = &loaded.name;
            self.known_modules.insert(qualifier.clone());
            for item in &loaded.ast.items {
                if let ast::Item::FnDef(f) = item {
                    if !f.is_pub {
                        continue;
                    }
                    let qualified_key = format!("{}::{}", qualifier, f.name);
                    // Only insert if not already present (avoids overwriting a
                    // more-specific entry that may have been set earlier).
                    if !self.fn_sigs.contains_key(&qualified_key) {
                        let params: Vec<crate::types::Type> = f
                            .params
                            .iter()
                            .map(|p| self.resolve_type_expr(&p.ty))
                            .collect();
                        let ret = f
                            .return_type
                            .as_ref()
                            .map(|t| self.resolve_type_expr(t))
                            .unwrap_or(crate::types::Type::Unit);
                        self.fn_sigs.insert(
                            qualified_key.clone(),
                            FnSig {
                                name: qualified_key,
                                type_params: f.type_params.clone(),
                                type_param_bounds: f.type_param_bounds.clone(),
                                params,
                                ret,
                            },
                        );
                    }
                }
            }
        }
    }

    fn register_nonpub_item_import_fallback_blocks(
        &mut self,
        modules: &[ark_resolve::LoadedModule],
    ) {
        let loaded_names: HashSet<&str> = modules.iter().map(|m| m.name.as_str()).collect();
        for module in modules {
            for import in &module.ast.imports {
                if !matches!(import.kind, ast::ImportKind::ModulePath) {
                    continue;
                }

                let Some((_, source_item)) = import.module_name.rsplit_once("::") else {
                    continue;
                };

                let import_leaf = import
                    .alias
                    .as_deref()
                    .unwrap_or_else(|| import.module_name.rsplit("::").next().unwrap_or(""));

                // Module import path (`use a::b`) loads a module named `b` (or alias).
                // Item import path fallback (`use a::b::item`) does not load a module
                // under `item`; detect that shape to block cross-module fallback.
                if loaded_names.contains(import_leaf) {
                    continue;
                }

                let exported_name = import.alias.as_deref().unwrap_or(source_item);
                self.blocked_nonpub_item_qualified
                    .insert(format!("{}::{}", module.name, exported_name));
            }
        }
    }

    /// Register `pub use source::item` re-exports as
    /// `reexporter_module::item` (or alias) in `fn_sigs`.
    fn register_pub_use_reexport_sigs(&mut self, modules: &[ark_resolve::LoadedModule]) {
        let by_name: HashMap<&str, &ark_resolve::LoadedModule> =
            modules.iter().map(|m| (m.name.as_str(), m)).collect();

        for reexporter in modules {
            for import in &reexporter.ast.imports {
                if !matches!(import.kind, ast::ImportKind::PublicModulePath) {
                    continue;
                }

                let Some((source_module, source_item)) = import.module_name.rsplit_once("::")
                else {
                    continue;
                };

                let Some(source_loaded) = by_name.get(source_module) else {
                    continue;
                };

                let Some(source_fn) = source_loaded.ast.items.iter().find_map(|item| match item {
                    ast::Item::FnDef(f) if f.is_pub && f.name == source_item => Some(f),
                    _ => None,
                }) else {
                    continue;
                };

                let exported_name = import.alias.as_deref().unwrap_or(source_item);
                let qualified_key = format!("{}::{}", reexporter.name, exported_name);
                if self.fn_sigs.contains_key(&qualified_key) {
                    continue;
                }

                let params: Vec<crate::types::Type> = source_fn
                    .params
                    .iter()
                    .map(|p| self.resolve_type_expr(&p.ty))
                    .collect();
                let ret = source_fn
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_type_expr(t))
                    .unwrap_or(crate::types::Type::Unit);

                self.fn_sigs.insert(
                    qualified_key.clone(),
                    FnSig {
                        name: qualified_key,
                        type_params: source_fn.type_params.clone(),
                        type_param_bounds: source_fn.type_param_bounds.clone(),
                        params,
                        ret,
                    },
                );
            }
        }
    }

    fn register_reexported_fn_sig_aliases(&mut self, aliases: &HashMap<String, String>) {
        for (qualified_export, source_plain_name) in aliases {
            if self.fn_sigs.contains_key(qualified_export) {
                continue;
            }
            let Some(source_sig) = self.fn_sigs.get(source_plain_name).cloned() else {
                continue;
            };
            self.fn_sigs.insert(
                qualified_export.clone(),
                FnSig {
                    name: qualified_export.clone(),
                    ..source_sig
                },
            );
        }
    }

    /// Emit E0102 for any use of a private symbol from another module.
    ///
    /// Currently a no-op because all declarations default to `Visibility::Public`.
    // TODO(MODULE-02): wire into per-item visibility once pub/priv keywords are parsed
    fn check_cross_module_visibility(
        &self,
        program: &ark_resolve::ResolvedProgram,
        sink: &mut DiagnosticSink,
    ) {
        for loaded in &program.modules {
            for item in &loaded.ast.items {
                let (name, is_pub, span) = match item {
                    ast::Item::FnDef(f) => (&f.name, f.is_pub, f.span),
                    ast::Item::StructDef(s) => (&s.name, s.is_pub, s.span),
                    ast::Item::EnumDef(e) => (&e.name, e.is_pub, e.span),
                    ast::Item::TraitDef(t) => (&t.name, t.is_pub, t.span),
                    ast::Item::ImplBlock(_) => continue,
                };
                if !is_pub {
                    // The symbol was imported but is private — emit a diagnostic.
                    // In practice this does not fire yet because
                    // `collect_module_items_pub_only` already filters private items
                    // during resolution, so they never enter the symbol table.
                    if program.symbols.lookup(program.global_scope, name).is_some() {
                        sink.emit(Diagnostic::new(DiagnosticCode::E0102).with_label(
                            span,
                            format!(
                                "cannot access private {} `{}` from module `{}`",
                                match item {
                                    ast::Item::FnDef(_) => "function",
                                    ast::Item::StructDef(_) => "struct",
                                    ast::Item::EnumDef(_) => "enum",
                                    ast::Item::TraitDef(_) => "trait",
                                    ast::Item::ImplBlock(_) => unreachable!(),
                                },
                                name,
                                loaded.name,
                            ),
                        ));
                    }
                }
            }
        }
    }

    /// Type check a module.
    pub fn check_module(&mut self, resolved: &ResolvedModule, sink: &mut DiagnosticSink) {
        // Propagate private imported names from the resolved module so that
        // QualifiedIdent fallback lookups respect cross-module privacy.
        for name in &resolved.private_imported_names {
            self.private_imported_fns.insert(name.clone());
        }
        // Propagate entry-module function names for visibility scoping.
        for name in &resolved.entry_fn_names {
            self.entry_fn_names.insert(name.clone());
        }
        // Propagate loaded module qualifier names so that QualifiedIdent
        // diagnostics can distinguish "module not found" (E0104) from
        // "symbol not found in module" (E0501).
        for name in &resolved.loaded_module_names {
            self.known_modules.insert(name.clone());
        }
        for name in &resolved.nonpub_item_import_blocked_qualified {
            self.blocked_nonpub_item_qualified.insert(name.clone());
        }
        // Register user-defined structs and enums in two passes to support
        // self-referential and mutually-recursive type definitions.
        // Pass 1: allocate type_ids for all structs/enums with empty fields
        // so forward references resolve correctly in pass 2.
        for item in &resolved.module.items {
            match item {
                ast::Item::StructDef(s) => {
                    let type_id = self.fresh_type_id();
                    self.struct_defs.insert(
                        s.name.clone(),
                        StructInfo {
                            name: s.name.clone(),
                            type_params: s.type_params.clone(),
                            fields: vec![],
                            type_id,
                        },
                    );
                }
                ast::Item::EnumDef(e) => {
                    let type_id = self.fresh_type_id();
                    self.enum_defs.insert(
                        e.name.clone(),
                        EnumInfo {
                            name: e.name.clone(),
                            variants: vec![],
                            type_params: e.type_params.clone(),
                            type_id,
                        },
                    );
                }
                _ => {}
            }
        }
        // Pass 2: resolve field types (all struct/enum names are now registered)
        for item in &resolved.module.items {
            match item {
                ast::Item::StructDef(s) => {
                    let fields: Vec<(String, Type)> = s
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.resolve_type_expr(&f.ty)))
                        .collect();
                    if let Some(info) = self.struct_defs.get_mut(&s.name) {
                        info.fields = fields;
                    }
                }
                ast::Item::EnumDef(e) => {
                    let variants: Vec<VariantInfo> = e
                        .variants
                        .iter()
                        .map(|v| match v {
                            ast::Variant::Unit { name, .. } => VariantInfo {
                                name: name.clone(),
                                fields: vec![],
                                field_names: vec![],
                            },
                            ast::Variant::Tuple { name, fields, .. } => VariantInfo {
                                name: name.clone(),
                                fields: fields.iter().map(|t| self.resolve_type_expr(t)).collect(),
                                field_names: vec![],
                            },
                            ast::Variant::Struct { name, fields, .. } => VariantInfo {
                                name: name.clone(),
                                fields: fields
                                    .iter()
                                    .map(|f| self.resolve_type_expr(&f.ty))
                                    .collect(),
                                field_names: fields.iter().map(|f| f.name.clone()).collect(),
                            },
                        })
                        .collect();
                    if let Some(info) = self.enum_defs.get_mut(&e.name) {
                        info.variants = variants;
                    }
                }
                ast::Item::FnDef(f) => {
                    let params: Vec<Type> = f
                        .params
                        .iter()
                        .map(|p| self.resolve_type_expr(&p.ty))
                        .collect();
                    let ret = f
                        .return_type
                        .as_ref()
                        .map(|t| self.resolve_type_expr(t))
                        .unwrap_or(Type::Unit);
                    self.fn_sigs.insert(
                        f.name.clone(),
                        FnSig {
                            name: f.name.clone(),
                            type_params: f.type_params.clone(),
                            type_param_bounds: f.type_param_bounds.clone(),
                            params,
                            ret,
                        },
                    );
                }
                ast::Item::TraitDef(t) => {
                    let mut methods = Vec::new();
                    for m in &t.methods {
                        let params: Vec<Type> = m
                            .params
                            .iter()
                            .map(|p| self.resolve_type_expr(&p.ty))
                            .collect();
                        let ret = m
                            .return_type
                            .as_ref()
                            .map(|r| self.resolve_type_expr(r))
                            .unwrap_or(Type::Unit);
                        methods.push((m.name.clone(), params, ret));
                    }
                    self.trait_defs.insert(t.name.clone(), methods);
                }
                ast::Item::ImplBlock(ib) => {
                    // Track trait implementations
                    if let Some(ref trait_name) = ib.trait_name {
                        self.trait_impls
                            .entry(ib.target_type.clone())
                            .or_default()
                            .push(trait_name.clone());
                    }
                    // Register each method with mangled name
                    for method in &ib.methods {
                        let mangled = format!("{}__{}", ib.target_type, method.name);
                        let params: Vec<Type> = method
                            .params
                            .iter()
                            .map(|p| self.resolve_type_expr(&p.ty))
                            .collect();
                        let ret = method
                            .return_type
                            .as_ref()
                            .map(|t| self.resolve_type_expr(t))
                            .unwrap_or(Type::Unit);
                        self.fn_sigs.insert(
                            mangled.clone(),
                            FnSig {
                                name: mangled.clone(),
                                type_params: vec![],
                                type_param_bounds: vec![],
                                params,
                                ret,
                            },
                        );
                        self.method_table
                            .insert((ib.target_type.clone(), method.name.clone()), mangled);
                    }
                }
            }
        }

        self.register_reexported_fn_sig_aliases(&resolved.pub_use_reexport_fn_aliases);

        // Type check function bodies
        for item in &resolved.module.items {
            if let ast::Item::FnDef(f) = item {
                self.check_function(f, sink);
            }
        }

        // Type check impl method bodies
        for item in &resolved.module.items {
            if let ast::Item::ImplBlock(ib) = item {
                for method in &ib.methods {
                    self.check_function(method, sink);
                }
            }
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_diagnostics::{DiagnosticSink, Span};
    use ark_parser::ast;
    use ark_resolve::LoadedModule;

    /// Build a minimal `LoadedModule` with one `pub fn` for testing.
    fn make_loaded_module(module_name: &str, fn_name: &str) -> LoadedModule {
        let span = Span::dummy();
        LoadedModule {
            name: module_name.to_string(),
            path: std::path::PathBuf::from(format!("<test/{}.ark>", module_name)),
            ast: ast::Module {
                docs: vec![],
                imports: vec![],
                items: vec![ast::Item::FnDef(ast::FnDef {
                    docs: vec![],
                    name: fn_name.to_string(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![
                        ast::Param {
                            name: "s".to_string(),
                            ty: ast::TypeExpr::Named {
                                name: "String".to_string(),
                                span,
                            },
                            span,
                        },
                        ast::Param {
                            name: "sep".to_string(),
                            ty: ast::TypeExpr::Named {
                                name: "String".to_string(),
                                span,
                            },
                            span,
                        },
                    ],
                    return_type: Some(ast::TypeExpr::Named {
                        name: "i32".to_string(),
                        span,
                    }),
                    body: ast::Block {
                        stmts: vec![],
                        tail_expr: Some(Box::new(ast::Expr::IntLit {
                            value: 0,
                            suffix: None,
                            span,
                        })),
                        span,
                    },
                    is_pub: true,
                    span,
                })],
            },
        }
    }

    /// Slice-3 (#039): `register_qualified_module_sigs` inserts
    /// `qualifier::fn_name` into `fn_sigs` for every `pub fn` in a loaded module.
    #[test]
    fn register_qualified_module_sigs_inserts_qualified_key() {
        let mut checker = TypeChecker::new();
        let modules = vec![make_loaded_module("string", "split")];
        checker.register_qualified_module_sigs(&modules);

        // Primary qualified key must be present.
        assert!(
            checker.fn_sigs.contains_key("string::split"),
            "expected `string::split` in fn_sigs after register_qualified_module_sigs"
        );
        // Plain key is NOT inserted by this method (check_module handles it separately).
        assert!(
            !checker.fn_sigs.contains_key("split"),
            "register_qualified_module_sigs should not insert plain key `split`"
        );
    }

    /// Slice-3 (#039): qualified key resolves to correct signature (params, ret).
    #[test]
    fn register_qualified_module_sigs_correct_signature() {
        let mut checker = TypeChecker::new();
        let modules = vec![make_loaded_module("string", "split")];
        checker.register_qualified_module_sigs(&modules);

        let sig = checker
            .fn_sigs
            .get("string::split")
            .expect("`string::split` sig must be present");
        assert_eq!(sig.params.len(), 2, "split takes 2 params");
        assert_eq!(sig.params[0], crate::types::Type::String);
        assert_eq!(sig.params[1], crate::types::Type::String);
        assert_eq!(sig.ret, crate::types::Type::I32);
    }

    /// Slice-3 (#039): a second module with the same plain fn name gets its own
    /// qualified key without collision.
    #[test]
    fn register_qualified_module_sigs_no_collision_between_modules() {
        let mut checker = TypeChecker::new();
        let modules = vec![
            make_loaded_module("string", "split"),
            make_loaded_module("url", "split"),
        ];
        checker.register_qualified_module_sigs(&modules);

        assert!(
            checker.fn_sigs.contains_key("string::split"),
            "`string::split` must be present"
        );
        assert!(
            checker.fn_sigs.contains_key("url::split"),
            "`url::split` must be present"
        );
    }

    /// Slice-3 (#039): private functions are NOT registered under qualified key.
    #[test]
    fn register_qualified_module_sigs_skips_private_fns() {
        let span = Span::dummy();
        let mut checker = TypeChecker::new();
        let module = LoadedModule {
            name: "string".to_string(),
            path: std::path::PathBuf::from("<test/string.ark>"),
            ast: ast::Module {
                docs: vec![],
                imports: vec![],
                items: vec![ast::Item::FnDef(ast::FnDef {
                    docs: vec![],
                    name: "internal_helper".to_string(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![],
                    return_type: None,
                    body: ast::Block {
                        stmts: vec![],
                        tail_expr: Some(Box::new(ast::Expr::IntLit {
                            value: 0,
                            suffix: None,
                            span,
                        })),
                        span,
                    },
                    is_pub: false, // private
                    span,
                })],
            },
        };
        checker.register_qualified_module_sigs(&[module]);
        assert!(
            !checker.fn_sigs.contains_key("string::internal_helper"),
            "private fn must not be registered under qualified key"
        );
    }

    #[test]
    fn register_pub_use_reexport_sigs_inserts_reexported_qualified_key() {
        let span = Span::dummy();
        let mut checker = TypeChecker::new();

        let mut facade = make_loaded_module("facade", "local");
        facade.ast.imports.push(ast::Import {
            module_name: "source::split".to_string(),
            alias: None,
            kind: ast::ImportKind::PublicModulePath,
            span,
        });

        let source = make_loaded_module("source", "split");
        let modules = vec![facade, source];

        checker.register_qualified_module_sigs(&modules);
        checker.register_pub_use_reexport_sigs(&modules);

        assert!(
            checker.fn_sigs.contains_key("facade::split"),
            "expected pub use re-export to register `facade::split`"
        );
    }

    #[test]
    fn register_pub_use_reexport_sigs_skips_non_pub_use() {
        let span = Span::dummy();
        let mut checker = TypeChecker::new();

        let mut facade = make_loaded_module("facade", "local");
        facade.ast.imports.push(ast::Import {
            module_name: "source::split".to_string(),
            alias: None,
            kind: ast::ImportKind::ModulePath,
            span,
        });

        let source = make_loaded_module("source", "split");
        let modules = vec![facade, source];

        checker.register_qualified_module_sigs(&modules);
        checker.register_pub_use_reexport_sigs(&modules);

        assert!(
            !checker.fn_sigs.contains_key("facade::split"),
            "non-pub use must not register `facade::split`"
        );
    }

    #[test]
    fn register_nonpub_item_import_fallback_blocks_item_import_pair() {
        let span = Span::dummy();
        let mut checker = TypeChecker::new();

        let mut facade = make_loaded_module("facade", "local");
        facade.ast.imports.push(ast::Import {
            module_name: "source::split".to_string(),
            alias: None,
            kind: ast::ImportKind::ModulePath,
            span,
        });
        let source = make_loaded_module("source", "split");

        checker.register_nonpub_item_import_fallback_blocks(&[facade, source]);

        assert!(
            checker
                .blocked_nonpub_item_qualified
                .contains("facade::split"),
            "non-pub item import must block `facade::split` plain-name fallback"
        );
    }

    #[test]
    fn register_nonpub_item_import_fallback_skips_module_import_shape() {
        let span = Span::dummy();
        let mut checker = TypeChecker::new();

        let mut user = make_loaded_module("user", "local");
        user.ast.imports.push(ast::Import {
            module_name: "std::host::stdio".to_string(),
            alias: None,
            kind: ast::ImportKind::ModulePath,
            span,
        });
        let stdio = make_loaded_module("stdio", "println");

        checker.register_nonpub_item_import_fallback_blocks(&[user, stdio]);

        assert!(
            !checker
                .blocked_nonpub_item_qualified
                .contains("user::stdio"),
            "module import shape must not block `user::stdio`"
        );
    }

    /// Slice-3 (#039): `QualifiedIdent` synthesize resolves via primary
    /// qualified key when `string::split` is in fn_sigs.
    #[test]
    fn synthesize_qualified_ident_resolves_via_primary_key() {
        use ark_diagnostics::Span;
        let mut checker = TypeChecker::new();
        // Pre-populate fn_sigs with the qualified key (as register_qualified_module_sigs would do).
        checker.fn_sigs.insert(
            "string::split".to_string(),
            FnSig {
                name: "string::split".to_string(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![crate::types::Type::String, crate::types::Type::String],
                ret: crate::types::Type::I32,
            },
        );

        let mut env = TypeEnv::new();
        let mut sink = DiagnosticSink::new();
        let expr = ast::Expr::QualifiedIdent {
            module: "string".to_string(),
            name: "split".to_string(),
            span: Span::dummy(),
        };
        let ty = checker.synthesize_expr(&expr, &mut env, &mut sink);
        assert!(
            sink.diagnostics().is_empty(),
            "should not emit any diagnostics for resolved qualified ident, got: {:?}",
            sink.diagnostics()
        );
        assert!(
            matches!(ty, crate::types::Type::Function { .. }),
            "expected Function type, got {:?}",
            ty
        );
    }

    /// Slice-3 continuation (#039): `QualifiedIdent` with an unknown module
    /// emits E0104 "module not found".
    #[test]
    fn qualified_ident_unknown_module_emits_e0104() {
        use ark_diagnostics::Span;
        let mut checker = TypeChecker::new();
        // Register "string" as a known module but NOT "nonexistent"
        checker.known_modules.insert("string".to_string());

        let mut env = TypeEnv::new();
        let mut sink = DiagnosticSink::new();
        let expr = ast::Expr::QualifiedIdent {
            module: "nonexistent".to_string(),
            name: "foo".to_string(),
            span: Span::dummy(),
        };
        let ty = checker.synthesize_expr(&expr, &mut env, &mut sink);
        assert_eq!(ty, crate::types::Type::Error);
        assert_eq!(sink.diagnostics().len(), 1);
        assert_eq!(
            sink.diagnostics()[0].code,
            DiagnosticCode::E0104,
            "expected E0104 for unknown module, got {:?}",
            sink.diagnostics()[0].code
        );
    }

    /// Slice-3 continuation (#039): `QualifiedIdent` with a known module but
    /// unknown symbol emits E0501 "symbol not found in module".
    #[test]
    fn qualified_ident_unknown_symbol_emits_e0501() {
        use ark_diagnostics::Span;
        let mut checker = TypeChecker::new();
        // Register "string" as a known module with one fn
        checker.known_modules.insert("string".to_string());
        checker.fn_sigs.insert(
            "string::split".to_string(),
            FnSig {
                name: "string::split".to_string(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![crate::types::Type::String, crate::types::Type::String],
                ret: crate::types::Type::I32,
            },
        );

        let mut env = TypeEnv::new();
        let mut sink = DiagnosticSink::new();
        let expr = ast::Expr::QualifiedIdent {
            module: "string".to_string(),
            name: "nonexistent_fn".to_string(),
            span: Span::dummy(),
        };
        let ty = checker.synthesize_expr(&expr, &mut env, &mut sink);
        assert_eq!(ty, crate::types::Type::Error);
        assert_eq!(sink.diagnostics().len(), 1);
        assert_eq!(
            sink.diagnostics()[0].code,
            DiagnosticCode::E0501,
            "expected E0501 for unknown symbol in known module, got {:?}",
            sink.diagnostics()[0].code
        );
    }
}
