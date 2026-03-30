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
        // Visibility diagnostic: detect references to private symbols across
        // module boundaries.  Everything is currently Public (scaffolding),
        // so this loop is a no-op until the resolver populates Visibility.
        self.check_cross_module_visibility(program, sink);

        #[allow(deprecated)]
        let flat = ark_resolve::resolved_program_to_module(program);
        let resolved = ark_resolve::ResolvedModule {
            module: flat,
            symbols: program.symbols.clone(),
            global_scope: program.global_scope,
            private_imported_names: self.private_imported_fns.clone(),
        };
        self.check_module(&resolved, sink);
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
        // Register user-defined structs and enums
        for item in &resolved.module.items {
            match item {
                ast::Item::StructDef(s) => {
                    let type_id = self.fresh_type_id();
                    let fields: Vec<(String, Type)> = s
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.resolve_type_expr(&f.ty)))
                        .collect();
                    self.struct_defs.insert(
                        s.name.clone(),
                        StructInfo {
                            name: s.name.clone(),
                            type_params: s.type_params.clone(),
                            fields,
                            type_id,
                        },
                    );
                }
                ast::Item::EnumDef(e) => {
                    let type_id = self.fresh_type_id();
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
                    self.enum_defs.insert(
                        e.name.clone(),
                        EnumInfo {
                            name: e.name.clone(),
                            variants,
                            type_params: e.type_params.clone(),
                            type_id,
                        },
                    );
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
