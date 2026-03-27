//! Type checker implementation with bidirectional inference.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, Span};
use ark_parser::ast;
use ark_resolve::ResolvedModule;

use ark_hir::validate_program;

use crate::build_corehir::{CoreHirBundle, build_core_hir_program};
use crate::typed_ast::{NodeIdAllocator, TypedAstMap, TypedExprInfo};
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
        }
    }

    fn fresh_type_id(&mut self) -> TypeId {
        let id = TypeId(self.next_type_id);
        self.next_type_id += 1;
        id
    }

    pub(crate) fn fresh_type_var(&mut self) -> Type {
        let id = self.next_type_var;
        self.next_type_var += 1;
        Type::TypeVar(id)
    }

    /// Register builtin types (Option, Result, Vec, String).
    pub fn register_builtins(&mut self) {
        // Architecture note: FnSig entries exist for BOTH public names (e.g., "println")
        // and their __intrinsic_* counterparts (e.g., "__intrinsic_println").
        // Public names serve as a defensive fallback when prelude.ark's AST definitions
        // are unavailable. The emitter normalizes __intrinsic_* → public via
        // normalize_intrinsic_name() before dispatch. This dual registration is intentional.

        // Option<T> is an enum with None and Some(T)
        let opt_id = self.fresh_type_id();
        self.enum_defs.insert(
            "Option".into(),
            EnumInfo {
                name: "Option".into(),
                variants: vec![
                    VariantInfo {
                        name: "None".into(),
                        fields: vec![],
                        field_names: vec![],
                    },
                    VariantInfo {
                        name: "Some".into(),
                        fields: vec![Type::TypeVar(0)],
                        field_names: vec![],
                    },
                ],
                type_params: vec!["T".into()],
                type_id: opt_id,
            },
        );

        // Result<T, E>
        let res_id = self.fresh_type_id();
        self.enum_defs.insert(
            "Result".into(),
            EnumInfo {
                name: "Result".into(),
                variants: vec![
                    VariantInfo {
                        name: "Ok".into(),
                        fields: vec![Type::TypeVar(0)],
                        field_names: vec![],
                    },
                    VariantInfo {
                        name: "Err".into(),
                        fields: vec![Type::TypeVar(1)],
                        field_names: vec![],
                    },
                ],
                type_params: vec!["T".into(), "E".into()],
                type_id: res_id,
            },
        );

        // Builtin I/O intrinsics
        self.fn_sigs.insert(
            "__intrinsic_println".into(),
            FnSig {
                name: "__intrinsic_println".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_print".into(),
            FnSig {
                name: "__intrinsic_print".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_eprintln".into(),
            FnSig {
                name: "__intrinsic_eprintln".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );

        // New __intrinsic_* sigs for functions now defined in prelude.ark
        self.fn_sigs.insert(
            "__intrinsic_string_new".into(),
            FnSig {
                name: "__intrinsic_string_new".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_concat".into(),
            FnSig {
                name: "__intrinsic_concat".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_string_clone".into(),
            FnSig {
                name: "__intrinsic_string_clone".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_starts_with".into(),
            FnSig {
                name: "__intrinsic_starts_with".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_ends_with".into(),
            FnSig {
                name: "__intrinsic_ends_with".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_to_lower".into(),
            FnSig {
                name: "__intrinsic_to_lower".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_to_upper".into(),
            FnSig {
                name: "__intrinsic_to_upper".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_string_slice".into(),
            FnSig {
                name: "__intrinsic_string_slice".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::I32, Type::I32],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_i32_to_string".into(),
            FnSig {
                name: "__intrinsic_i32_to_string".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_i64_to_string".into(),
            FnSig {
                name: "__intrinsic_i64_to_string".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I64],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_f64_to_string".into(),
            FnSig {
                name: "__intrinsic_f64_to_string".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::F64],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_bool_to_string".into(),
            FnSig {
                name: "__intrinsic_bool_to_string".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Bool],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_char_to_string".into(),
            FnSig {
                name: "__intrinsic_char_to_string".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Char],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_parse_i32".into(),
            FnSig {
                name: "__intrinsic_parse_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Result(Box::new(Type::I32), Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_parse_i64".into(),
            FnSig {
                name: "__intrinsic_parse_i64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Result(Box::new(Type::I64), Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_parse_f64".into(),
            FnSig {
                name: "__intrinsic_parse_f64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Result(Box::new(Type::F64), Box::new(Type::String)),
            },
        );
        // I/O intrinsics
        self.fn_sigs.insert(
            "__intrinsic_fs_read_file".into(),
            FnSig {
                name: "__intrinsic_fs_read_file".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Result(Box::new(Type::String), Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_fs_write_file".into(),
            FnSig {
                name: "__intrinsic_fs_write_file".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Result(Box::new(Type::Unit), Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_clock_now".into(),
            FnSig {
                name: "__intrinsic_clock_now".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::I64,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_random_i32".into(),
            FnSig {
                name: "__intrinsic_random_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::I32,
            },
        );
        // String intrinsics
        self.fn_sigs.insert(
            "__intrinsic_split".into(),
            FnSig {
                name: "__intrinsic_split".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_join".into(),
            FnSig {
                name: "__intrinsic_join".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::String)), Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_push_char".into(),
            FnSig {
                name: "__intrinsic_push_char".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::Char],
                ret: Type::Unit,
            },
        );
        // Sort intrinsics
        self.fn_sigs.insert(
            "__intrinsic_sort_i64".into(),
            FnSig {
                name: "__intrinsic_sort_i64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I64))],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_sort_f64".into(),
            FnSig {
                name: "__intrinsic_sort_f64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::F64))],
                ret: Type::Unit,
            },
        );
        // HOF intrinsics (String variants)
        self.fn_sigs.insert(
            "__intrinsic_map_String_String".into(),
            FnSig {
                name: "__intrinsic_map_String_String".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::String)), Type::I32],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_filter_String".into(),
            FnSig {
                name: "__intrinsic_filter_String".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::String)), Type::I32],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        // Assert intrinsics
        self.fn_sigs.insert(
            "__intrinsic_assert".into(),
            FnSig {
                name: "__intrinsic_assert".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Bool],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_assert_eq".into(),
            FnSig {
                name: "__intrinsic_assert_eq".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_assert_ne".into(),
            FnSig {
                name: "__intrinsic_assert_ne".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_assert_eq_i64".into(),
            FnSig {
                name: "__intrinsic_assert_eq_i64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I64, Type::I64],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_assert_eq_str".into(),
            FnSig {
                name: "__intrinsic_assert_eq_str".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Unit,
            },
        );
        // HOF i64/f64 intrinsics
        self.fn_sigs.insert(
            "__intrinsic_map_i64_i64".into(),
            FnSig {
                name: "__intrinsic_map_i64_i64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I64)), Type::I32],
                ret: Type::Vec(Box::new(Type::I64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_filter_i64".into(),
            FnSig {
                name: "__intrinsic_filter_i64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I64)), Type::I32],
                ret: Type::Vec(Box::new(Type::I64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_fold_i64_i64".into(),
            FnSig {
                name: "__intrinsic_fold_i64_i64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I64)), Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_map_f64_f64".into(),
            FnSig {
                name: "__intrinsic_map_f64_f64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::F64)), Type::I32],
                ret: Type::Vec(Box::new(Type::F64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_filter_f64".into(),
            FnSig {
                name: "__intrinsic_filter_f64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::F64)), Type::I32],
                ret: Type::Vec(Box::new(Type::F64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_contains_i32".into(),
            FnSig {
                name: "__intrinsic_contains_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_contains_String".into(),
            FnSig {
                name: "__intrinsic_contains_String".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::String)), Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_reverse_i32".into(),
            FnSig {
                name: "__intrinsic_reverse_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_reverse_String".into(),
            FnSig {
                name: "__intrinsic_reverse_String".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::String))],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_remove_i32".into(),
            FnSig {
                name: "__intrinsic_remove_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_sqrt".into(),
            FnSig {
                name: "__intrinsic_sqrt".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::F64],
                ret: Type::F64,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_abs".into(),
            FnSig {
                name: "__intrinsic_abs".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_min".into(),
            FnSig {
                name: "__intrinsic_min".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_max".into(),
            FnSig {
                name: "__intrinsic_max".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_panic".into(),
            FnSig {
                name: "__intrinsic_panic".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_i32".into(),
            FnSig {
                name: "__intrinsic_Vec_new_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_i64".into(),
            FnSig {
                name: "__intrinsic_Vec_new_i64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::I64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_f64".into(),
            FnSig {
                name: "__intrinsic_Vec_new_f64".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::F64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_String".into(),
            FnSig {
                name: "__intrinsic_Vec_new_String".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_sort_i32".into(),
            FnSig {
                name: "__intrinsic_sort_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_sort_String".into(),
            FnSig {
                name: "__intrinsic_sort_String".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::String))],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_map_i32_i32".into(),
            FnSig {
                name: "__intrinsic_map_i32_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_filter_i32".into(),
            FnSig {
                name: "__intrinsic_filter_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_fold_i32_i32".into(),
            FnSig {
                name: "__intrinsic_fold_i32_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_map_option_i32_i32".into(),
            FnSig {
                name: "__intrinsic_map_option_i32_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Option(Box::new(Type::I32)), Type::I32],
                ret: Type::Option(Box::new(Type::I32)),
            },
        );
        // any_i32(Vec<i32>, fn_idx) -> bool
        self.fn_sigs.insert(
            "__intrinsic_any_i32".into(),
            FnSig {
                name: "__intrinsic_any_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Bool,
            },
        );
        // find_i32(Vec<i32>, fn_idx) -> Option<i32>
        self.fn_sigs.insert(
            "__intrinsic_find_i32".into(),
            FnSig {
                name: "__intrinsic_find_i32".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::I32,
            },
        );

        self.fn_sigs.insert(
            "__intrinsic_string_from".into(),
            FnSig {
                name: "__intrinsic_string_from".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        // Polymorphic to_string (accepts any type, dispatched in MIR/emitter)
        self.fn_sigs.insert(
            "to_string".into(),
            FnSig {
                name: "to_string".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32], // placeholder; overridden in call handling
                ret: Type::String,
            },
        );
        // String equality
        self.fn_sigs.insert(
            "__intrinsic_string_eq".into(),
            FnSig {
                name: "__intrinsic_string_eq".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        // Enum variant constructors (treated as functions for type checking)
        self.fn_sigs.insert(
            "Some".into(),
            FnSig {
                name: "Some".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32], // generic but we treat as i32 for now
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "Ok".into(),
            FnSig {
                name: "Ok".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "Err".into(),
            FnSig {
                name: "Err".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );

        self.fn_sigs.insert(
            "push".into(),
            FnSig {
                name: "push".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "pop".into(),
            FnSig {
                name: "pop".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::Option(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "len".into(),
            FnSig {
                name: "len".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "get".into(),
            FnSig {
                name: "get".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Option(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "get_unchecked".into(),
            FnSig {
                name: "get_unchecked".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "set".into(),
            FnSig {
                name: "set".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32, Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "is_empty".into(),
            FnSig {
                name: "is_empty".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::Bool,
            },
        );
        // Box builtins (Box<T> is represented as an i32 pointer)
        self.fn_sigs.insert(
            "Box_new".into(),
            FnSig {
                name: "Box_new".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "unbox".into(),
            FnSig {
                name: "unbox".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "unwrap".into(),
            FnSig {
                name: "unwrap".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "unwrap_or".into(),
            FnSig {
                name: "unwrap_or".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "is_some".into(),
            FnSig {
                name: "is_some".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "is_none".into(),
            FnSig {
                name: "is_none".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::Bool,
            },
        );

        // --- Missing stdlib: f32_to_string ---
        self.fn_sigs.insert(
            "f32_to_string".into(),
            FnSig {
                name: "f32_to_string".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::F32],
                ret: Type::String,
            },
        );

        // --- HashMap<i32, i32> ---
        self.fn_sigs.insert(
            "HashMap_i32_i32_new".into(),
            FnSig {
                name: "HashMap_i32_i32_new".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "HashMap_i32_i32_insert".into(),
            FnSig {
                name: "HashMap_i32_i32_insert".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32, Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "HashMap_i32_i32_get".into(),
            FnSig {
                name: "HashMap_i32_i32_get".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::Option(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "HashMap_i32_i32_contains_key".into(),
            FnSig {
                name: "HashMap_i32_i32_contains_key".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "HashMap_i32_i32_len".into(),
            FnSig {
                name: "HashMap_i32_i32_len".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
    }

    /// Resolve a type expression to a Type.
    pub fn resolve_type_expr(&self, ty: &ast::TypeExpr) -> Type {
        match ty {
            ast::TypeExpr::Named { name, .. } => match name.as_str() {
                "i32" => Type::I32,
                "i64" => Type::I64,
                "f32" => Type::F32,
                "f64" => Type::F64,
                "bool" => Type::Bool,
                "char" => Type::Char,
                "String" => Type::String,
                _ => {
                    if let Some(info) = self.struct_defs.get(name) {
                        Type::Struct(info.type_id)
                    } else if let Some(info) = self.enum_defs.get(name) {
                        Type::Enum(info.type_id)
                    } else {
                        Type::Error
                    }
                }
            },
            ast::TypeExpr::Generic { name, args, .. } => {
                let resolved_args: Vec<Type> =
                    args.iter().map(|a| self.resolve_type_expr(a)).collect();
                match name.as_str() {
                    "Vec" if resolved_args.len() == 1 => {
                        Type::Vec(Box::new(resolved_args[0].clone()))
                    }
                    "Option" if resolved_args.len() == 1 => {
                        Type::Option(Box::new(resolved_args[0].clone()))
                    }
                    "Result" if resolved_args.len() == 2 => Type::Result(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    "Box" if resolved_args.len() == 1 => Type::I32, // Box is a pointer
                    _ => Type::Error,
                }
            }
            ast::TypeExpr::Tuple(types, _) => {
                Type::Tuple(types.iter().map(|t| self.resolve_type_expr(t)).collect())
            }
            ast::TypeExpr::Array { elem, size, .. } => {
                Type::Array(Box::new(self.resolve_type_expr(elem)), *size)
            }
            ast::TypeExpr::Slice { elem, .. } => {
                Type::Slice(Box::new(self.resolve_type_expr(elem)))
            }
            ast::TypeExpr::Function { params, ret, .. } => Type::Function {
                params: params.iter().map(|p| self.resolve_type_expr(p)).collect(),
                ret: Box::new(self.resolve_type_expr(ret)),
            },
            ast::TypeExpr::Unit(_) => Type::Unit,
            ast::TypeExpr::Qualified { .. } => {
                // e.g., io.Capabilities — treated as opaque for now
                Type::Error
            }
        }
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

    fn check_function(&mut self, f: &ast::FnDef, sink: &mut DiagnosticSink) {
        let mut env = TypeEnv::new();

        // Bind parameters
        for param in &f.params {
            let ty = self.resolve_type_expr(&param.ty);
            env.bind(param.name.clone(), ty);
        }

        // Bind type parameters
        for tp in &f.type_params {
            env.bind(tp.clone(), self.fresh_type_var());
        }

        let expected_ret = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_type_expr(t))
            .unwrap_or(Type::Unit);

        self.current_fn_return_type = Some(expected_ret.clone());

        // Check body block
        let _body_type = self.check_block(&f.body, &mut env, &expected_ret, sink);

        self.current_fn_return_type = None;
    }

    fn check_block(
        &mut self,
        block: &ast::Block,
        env: &mut TypeEnv,
        _expected: &Type,
        sink: &mut DiagnosticSink,
    ) -> Type {
        for stmt in &block.stmts {
            self.check_stmt(stmt, env, sink);
        }

        if let Some(tail) = &block.tail_expr {
            self.synthesize_expr(tail, env, sink)
        } else {
            Type::Unit
        }
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt, env: &mut TypeEnv, sink: &mut DiagnosticSink) {
        match stmt {
            ast::Stmt::Let {
                name,
                ty,
                init,
                is_mut,
                pattern,
                ..
            } => {
                // Handle tuple destructuring: let (a, b) = expr
                if let Some(ast::Pattern::Tuple { elements, .. }) = pattern {
                    let init_type = self.synthesize_expr(init, env, sink);
                    // Bind each element name as i32 (our universal runtime type)
                    for elem in elements {
                        if let ast::Pattern::Ident {
                            name: elem_name, ..
                        } = elem
                        {
                            env.bind(elem_name.clone(), Type::I32);
                        }
                    }
                    let _ = init_type;
                    return;
                }
                let init_type = if let Some(type_expr) = ty {
                    let declared_type = self.resolve_type_expr(type_expr);
                    // For numeric literals, allow them to match declared type
                    let init_type = match (init, &declared_type) {
                        (ast::Expr::IntLit { .. }, Type::I64) => Type::I64,
                        (ast::Expr::IntLit { .. }, Type::F64) => Type::F64,
                        (ast::Expr::IntLit { .. }, Type::F32) => Type::F32,
                        _ => self.synthesize_expr(init, env, sink),
                    };
                    if !self.types_compatible(&init_type, &declared_type) {
                        sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                            "expected `{}`, found `{}`",
                            declared_type, init_type
                        )));
                    }
                    if *is_mut {
                        env.bind_mut(name.clone(), declared_type.clone());
                    } else {
                        env.bind(name.clone(), declared_type.clone());
                    }
                    declared_type
                } else {
                    let init_type = self.synthesize_expr(init, env, sink);
                    if *is_mut {
                        env.bind_mut(name.clone(), init_type.clone());
                    } else {
                        env.bind(name.clone(), init_type.clone());
                    }
                    init_type
                };
                let _ = init_type;
                // W0001: warn on aliasing mutable reference types
                if let ast::Expr::Ident {
                    name: src_name,
                    span: src_span,
                    ..
                } = init
                {
                    if env.is_mutable(src_name) {
                        let src_ty = env.lookup(src_name).cloned().unwrap_or(Type::Error);
                        if matches!(src_ty, Type::Vec(_)) {
                            sink.emit(
                                Diagnostic::new(DiagnosticCode::W0001)
                                    .with_label(*src_span, format!(
                                        "mutable `{}` is shared by reference; mutations affect both variables",
                                        src_name
                                    )),
                            );
                        }
                    }
                }
            }
            ast::Stmt::Expr(expr) => {
                self.synthesize_expr(expr, env, sink);
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond_ty = self.synthesize_expr(cond, env, sink);
                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "while condition must be `bool`, found `{}`",
                        cond_ty
                    )));
                }
                self.check_block(body, env, &Type::Unit, sink);
            }
            ast::Stmt::Loop { body, .. } => {
                self.check_block(body, env, &Type::Unit, sink);
            }
            ast::Stmt::For {
                target, iter, body, ..
            } => {
                let elem_ty = match iter {
                    ast::ForIter::Range { start, end, .. } => {
                        let start_ty = self.synthesize_expr(start, env, sink);
                        let end_ty = self.synthesize_expr(end, env, sink);
                        if start_ty != Type::I32 && start_ty != Type::I64 && start_ty != Type::Error
                        {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                format!("range start must be integer, found `{}`", start_ty),
                            ));
                        }
                        if end_ty != Type::I32 && end_ty != Type::I64 && end_ty != Type::Error {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                format!("range end must be integer, found `{}`", end_ty),
                            ));
                        }
                        start_ty
                    }
                    ast::ForIter::Values(expr) => {
                        let vec_ty = self.synthesize_expr(expr, env, sink);
                        // Element type from Vec<T> — currently always i32
                        match &vec_ty {
                            Type::Vec(inner) => *inner.clone(),
                            _ => Type::I32,
                        }
                    }
                    ast::ForIter::Iter(expr) => {
                        let iter_ty = self.synthesize_expr(expr, env, sink);
                        // Check if the type has a `next` method returning Option<T>
                        if let Type::Struct(type_id) = &iter_ty {
                            let struct_name = self
                                .struct_defs
                                .values()
                                .find(|s| s.type_id == *type_id)
                                .map(|s| s.name.clone());
                            if let Some(ref sname) = struct_name {
                                let mangled = format!("{}__next", sname);
                                if let Some(sig) = self.fn_sigs.get(&mangled).cloned() {
                                    // Extract T from Option<T> return type
                                    match &sig.ret {
                                        Type::Option(inner) => *inner.clone(),
                                        _ => {
                                            sink.emit(
                                                Diagnostic::new(DiagnosticCode::E0200)
                                                    .with_message(format!(
                                                    "`{}`::next() must return Option<T>, found `{}`",
                                                    sname, sig.ret
                                                )),
                                            );
                                            Type::Error
                                        }
                                    }
                                } else {
                                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                        format!(
                                            "`{}` does not implement Iterator (no `next` method)",
                                            sname
                                        ),
                                    ));
                                    Type::Error
                                }
                            } else {
                                sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                    "for..in requires a type that implements Iterator",
                                ));
                                Type::Error
                            }
                        } else {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(
                                format!("for..in requires an Iterator type, found `{}`", iter_ty),
                            ));
                            Type::Error
                        }
                    }
                };
                let mut child_env = env.child();
                child_env.bind_mut(target.clone(), elem_ty);
                self.check_block(body, &mut child_env, &Type::Unit, sink);
            }
        }
    }

    /// Synthesize the type of an expression.
    pub(crate) fn synthesize_expr(
        &mut self,
        expr: &ast::Expr,
        env: &mut TypeEnv,
        sink: &mut DiagnosticSink,
    ) -> Type {
        match expr {
            ast::Expr::IntLit { .. } => Type::I32,
            ast::Expr::FloatLit { .. } => Type::F64,
            ast::Expr::StringLit { .. } => Type::String,
            ast::Expr::CharLit { .. } => Type::Char,
            ast::Expr::BoolLit { .. } => Type::Bool,
            ast::Expr::Ident { name, span } => {
                if let Some(ty) = env.lookup(name) {
                    ty.clone()
                } else if self.fn_sigs.contains_key(name) {
                    // Function reference
                    let sig = self.fn_sigs[name].clone();
                    Type::Function {
                        params: sig.params,
                        ret: Box::new(sig.ret),
                    }
                } else if name == "None" || name == "true" || name == "false" {
                    // Known prelude values — don't emit an error
                    Type::I32
                } else if name.starts_with("Vec_new_") {
                    // Dynamic Vec constructor for any type (e.g., Vec_new_Point)
                    Type::Function {
                        params: vec![],
                        ret: Box::new(Type::I32),
                    }
                } else {
                    // "Did you mean?" suggestion
                    let candidates: Vec<&str> = self
                        .fn_sigs
                        .keys()
                        .map(|s| s.as_str())
                        .chain(env.bindings.keys().map(|s| s.as_str()))
                        .chain(self.struct_defs.keys().map(|s| s.as_str()))
                        .collect();
                    let mut best_dist = usize::MAX;
                    let mut best_name = "";
                    for c in &candidates {
                        if c.starts_with("__") {
                            continue;
                        }
                        let d = edit_distance(name, c);
                        if d < best_dist && d <= 2 {
                            best_dist = d;
                            best_name = c;
                        }
                    }
                    let mut diag = Diagnostic::new(DiagnosticCode::E0100)
                        .with_label(*span, format!("unresolved name `{}`", name));
                    if !best_name.is_empty() {
                        diag = diag.with_suggestion(format!("did you mean `{}`?", best_name));
                    }
                    sink.emit(diag);
                    Type::Error
                }
            }
            ast::Expr::Binary {
                left,
                op,
                right,
                span,
                ..
            } => {
                let left_ty = self.synthesize_expr(left, env, sink);
                let right_ty = self.synthesize_expr(right, env, sink);
                self.check_binary_op(op, &left_ty, &right_ty, *span, sink)
            }
            ast::Expr::Unary { op, operand, .. } => {
                let operand_ty = self.synthesize_expr(operand, env, sink);
                self.check_unary_op(op, &operand_ty, sink)
            }
            ast::Expr::Call {
                callee, args, span, ..
            } => {
                // Check for method call: x.method(args)
                if let ast::Expr::FieldAccess { object, field, .. } = callee.as_ref() {
                    let obj_ty = self.synthesize_expr(object, env, sink);
                    if let Type::Struct(type_id) = &obj_ty {
                        // Find struct name from type_id
                        let struct_name = self
                            .struct_defs
                            .values()
                            .find(|s| s.type_id == *type_id)
                            .map(|s| s.name.clone());
                        if let Some(sname) = struct_name {
                            let mangled = format!("{}__{}", sname, field);
                            if let Some(sig) = self.fn_sigs.get(&mangled).cloned() {
                                // Method found — type check args (skip self param)
                                let expected_params = if sig.params.len() > 1 {
                                    &sig.params[1..]
                                } else {
                                    &[]
                                };
                                if args.len() != expected_params.len() {
                                    sink.emit(Diagnostic::new(DiagnosticCode::E0202).with_message(
                                        format!(
                                            "method `{}` expected {} argument(s), found {}",
                                            field,
                                            expected_params.len(),
                                            args.len()
                                        ),
                                    ));
                                }
                                for a in args {
                                    self.synthesize_expr(a, env, sink);
                                }
                                // Record method resolution for MIR lowering
                                self.method_resolutions
                                    .insert(span.start, (mangled.clone(), sname.clone()));
                                let expr_id = self.node_ids.fresh_expr();
                                self.typed_ast_map.register_span(span.start, expr_id);
                                self.typed_ast_map.insert_expr(
                                    expr_id,
                                    TypedExprInfo {
                                        ty: sig.ret.clone(),
                                        method_resolution: Some((mangled, sname)),
                                    },
                                );
                                return sig.ret;
                            }
                        }
                    }
                    // Not a method call — synthesize args but return I32 (struct field or error)
                    for a in args {
                        self.synthesize_expr(a, env, sink);
                    }
                    return Type::I32;
                }

                let callee_ty = self.synthesize_expr(callee, env, sink);
                match callee_ty {
                    Type::Function { params, ret } => {
                        if args.len() != params.len() {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0202).with_message(
                                format!(
                                    "expected {} argument(s), found {}",
                                    params.len(),
                                    args.len()
                                ),
                            ));
                        }
                        // Generic Vec function inference: if the declared param is Vec<i32>
                        // but actual arg is Vec<String>, adjust the return type accordingly.
                        let callee_name = match callee.as_ref() {
                            ast::Expr::Ident { name, .. } => Some(name.as_str()),
                            _ => None,
                        };
                        if let Some(fname) = callee_name {
                            let arg_types: Vec<Type> = args
                                .iter()
                                .map(|a| self.synthesize_expr(a, env, sink))
                                .collect();
                            match fname {
                                "len" | "sort_i32" => {
                                    // Always returns i32/Unit regardless of element type
                                    *ret
                                }
                                "get_unchecked" => {
                                    if let Some(Type::Vec(elem_ty)) = arg_types.first() {
                                        *elem_ty.clone()
                                    } else {
                                        *ret
                                    }
                                }
                                "get" => {
                                    if let Some(Type::Vec(elem_ty)) = arg_types.first() {
                                        Type::Option(elem_ty.clone())
                                    } else {
                                        *ret
                                    }
                                }
                                "push" | "set" => Type::Unit,
                                "pop" => {
                                    if let Some(Type::Vec(elem_ty)) = arg_types.first() {
                                        Type::Option(elem_ty.clone())
                                    } else {
                                        *ret
                                    }
                                }
                                "to_string" => {
                                    // Polymorphic: accepts any displayable type, returns String
                                    // For struct types, check Display trait impl
                                    if let Some(arg_ty) = arg_types.first() {
                                        match arg_ty {
                                            Type::I32
                                            | Type::I64
                                            | Type::F64
                                            | Type::Bool
                                            | Type::Char
                                            | Type::String => Type::String,
                                            Type::Struct(tid) => {
                                                // Look up struct name from type_id
                                                let sname = self
                                                    .struct_defs
                                                    .iter()
                                                    .find(|(_, info)| info.type_id == *tid)
                                                    .map(|(name, _)| name.clone());
                                                if let Some(ref name) = sname {
                                                    let has_display = self
                                                        .trait_impls
                                                        .get(name)
                                                        .is_some_and(|traits| {
                                                            traits.contains(&"Display".to_string())
                                                        });
                                                    if !has_display {
                                                        sink.emit(
                                                            Diagnostic::new(DiagnosticCode::E0200)
                                                                .with_message(format!(
                                                                    "type `{}` does not implement `Display` trait; add `impl Display for {} {{ fn to_string(self) -> String {{ ... }} }}`",
                                                                    name, name
                                                                )),
                                                        );
                                                    }
                                                }
                                                Type::String
                                            }
                                            _ => Type::String,
                                        }
                                    } else {
                                        Type::String
                                    }
                                }
                                _ => *ret,
                            }
                        } else {
                            *ret
                        }
                    }
                    Type::Error => Type::Error,
                    _ => {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0200)
                                .with_message(format!("expected function, found `{}`", callee_ty)),
                        );
                        Type::Error
                    }
                }
            }
            ast::Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let cond_ty = self.synthesize_expr(cond, env, sink);
                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                            "if condition must be `bool`, found `{}`",
                            cond_ty
                        )),
                    );
                }
                let then_ty = self.check_block(then_block, env, &Type::Unit, sink);
                if let Some(else_blk) = else_block {
                    let else_ty = self.check_block(else_blk, env, &then_ty, sink);
                    if !self.types_compatible(&then_ty, &else_ty) {
                        sink.emit(Diagnostic::new(DiagnosticCode::E0205).with_message(format!(
                            "if/else branches have incompatible types: `{}` vs `{}`",
                            then_ty, else_ty
                        )));
                    }
                    then_ty
                } else {
                    Type::Unit
                }
            }
            ast::Expr::Block(block) => self.check_block(block, env, &Type::Unit, sink),
            ast::Expr::Tuple { elements, .. } => Type::Tuple(
                elements
                    .iter()
                    .map(|e| self.synthesize_expr(e, env, sink))
                    .collect(),
            ),
            ast::Expr::Array { elements, .. } => {
                if elements.is_empty() {
                    Type::Error // need type annotation
                } else {
                    let first = self.synthesize_expr(&elements[0], env, sink);
                    Type::Array(Box::new(first), elements.len() as u64)
                }
            }
            ast::Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.synthesize_expr(val, env, sink);
                }
                Type::Never
            }
            ast::Expr::Break { value, .. } => {
                if let Some(val) = value {
                    self.synthesize_expr(val, env, sink);
                }
                Type::Never
            }
            ast::Expr::Continue { .. } => Type::Never,
            ast::Expr::Loop { body, .. } => {
                self.check_block(body, env, &Type::Unit, sink);
                // Loop as expression: type comes from break value
                Type::I32
            }
            ast::Expr::Assign { target, value, .. } => {
                let val_ty = self.synthesize_expr(value, env, sink);
                if let ast::Expr::Ident { name, span } = target.as_ref() {
                    // Check mutability
                    if env.lookup(name).is_some() && !env.is_mutable(name) {
                        sink.emit(
                            Diagnostic::new(DiagnosticCode::E0207)
                                .with_message(format!(
                                    "cannot assign to immutable variable `{}`",
                                    name
                                ))
                                .with_label(*span, "cannot assign to immutable variable"),
                        );
                    }
                    if let Some(target_ty) = env.lookup(name) {
                        let target_ty = target_ty.clone();
                        if !self.types_compatible(&val_ty, &target_ty) {
                            sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_label(
                                *span,
                                format!("expected `{}`, found `{}`", target_ty, val_ty),
                            ));
                        }
                    }
                }
                Type::Unit
            }
            ast::Expr::Match {
                scrutinee,
                arms,
                span,
            } => {
                let scrutinee_ty = self.synthesize_expr(scrutinee, env, sink);
                let mut result_ty: Option<Type> = None;
                let mut has_wildcard = false;
                let mut covered_variants: Vec<String> = Vec::new();
                for arm in arms {
                    // Create a child env for each arm to bind pattern variables
                    let mut arm_env = env.clone();

                    // Track pattern coverage and bind pattern variables
                    match &arm.pattern {
                        ast::Pattern::Wildcard(_) => {
                            has_wildcard = true;
                        }
                        ast::Pattern::Ident { name, .. } => {
                            has_wildcard = true;
                            arm_env.bind(name.clone(), scrutinee_ty.clone());
                        }
                        ast::Pattern::Enum {
                            path,
                            variant,
                            fields,
                            ..
                        } => {
                            covered_variants.push(if path.is_empty() {
                                variant.clone()
                            } else {
                                format!("{}::{}", path, variant)
                            });
                            // Bind fields from enum variant payload
                            // Handle Option<T> and Result<T,E> specially since they use
                            // Type::Option / Type::Result, not Type::Enum
                            let mut bound_from_builtin = false;
                            if variant == "Some" {
                                if let Type::Option(ref inner) = scrutinee_ty {
                                    if let Some(ast::Pattern::Ident { name, .. }) = fields.first() {
                                        arm_env.bind(name.clone(), *inner.clone());
                                        bound_from_builtin = true;
                                    }
                                }
                            } else if variant == "Ok" {
                                if let Type::Result(ref ok_ty, _) = scrutinee_ty {
                                    if let Some(ast::Pattern::Ident { name, .. }) = fields.first() {
                                        arm_env.bind(name.clone(), *ok_ty.clone());
                                        bound_from_builtin = true;
                                    }
                                }
                            } else if variant == "Err" {
                                if let Type::Result(_, ref err_ty) = scrutinee_ty {
                                    if let Some(ast::Pattern::Ident { name, .. }) = fields.first() {
                                        arm_env.bind(name.clone(), *err_ty.clone());
                                        bound_from_builtin = true;
                                    }
                                }
                            }
                            if !bound_from_builtin {
                                let enum_name = if path.is_empty() {
                                    // Try to infer from scrutinee type
                                    if let Type::Enum(ref tid) = scrutinee_ty {
                                        self.enum_defs
                                            .iter()
                                            .find(|(_, info)| &info.type_id == tid)
                                            .map(|(n, _)| n.clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    Some(path.clone())
                                };
                                if let Some(ref ename) = enum_name {
                                    if let Some(info) = self.enum_defs.get(ename) {
                                        if let Some(vinfo) =
                                            info.variants.iter().find(|v| v.name == *variant)
                                        {
                                            for (i, field_pat) in fields.iter().enumerate() {
                                                if let ast::Pattern::Ident { name, .. } = field_pat
                                                {
                                                    let field_ty = vinfo
                                                        .fields
                                                        .get(i)
                                                        .cloned()
                                                        .unwrap_or(Type::Error);
                                                    arm_env.bind(name.clone(), field_ty);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ast::Pattern::Struct {
                            name: sname,
                            fields: sfields,
                            ..
                        } => {
                            // Check if this is an enum struct variant pattern: "EnumName::VariantName"
                            if let Some((enum_name, variant_name)) = sname.split_once("::") {
                                covered_variants.push(sname.clone());
                                if let Some(einfo) = self.enum_defs.get(enum_name) {
                                    if let Some(vinfo) =
                                        einfo.variants.iter().find(|v| v.name == variant_name)
                                    {
                                        for (fname, fpat) in sfields {
                                            let binding_name = match fpat {
                                                Some(ast::Pattern::Ident { name: n, .. }) => {
                                                    n.clone()
                                                }
                                                None => fname.clone(),
                                                _ => fname.clone(),
                                            };
                                            let field_ty = vinfo
                                                .field_names
                                                .iter()
                                                .position(|n| n == fname)
                                                .and_then(|idx| vinfo.fields.get(idx))
                                                .cloned()
                                                .unwrap_or(Type::Error);
                                            arm_env.bind(binding_name, field_ty);
                                        }
                                    }
                                }
                            } else {
                                has_wildcard = true;
                                if let Some(sinfo) = self.struct_defs.get(sname.as_str()).cloned() {
                                    for (fname, fpat) in sfields {
                                        let binding_name = match fpat {
                                            Some(ast::Pattern::Ident { name: n, .. }) => n.clone(),
                                            None => fname.clone(),
                                            _ => fname.clone(),
                                        };
                                        let field_ty = sinfo
                                            .fields
                                            .iter()
                                            .find(|(n, _)| n == fname)
                                            .map(|(_, t)| t.clone())
                                            .unwrap_or(Type::Error);
                                        arm_env.bind(binding_name, field_ty);
                                    }
                                }
                            }
                        }
                        ast::Pattern::Or { patterns, .. } => {
                            // Bind variables from the first sub-pattern
                            for pat in patterns.iter().take(1) {
                                match pat {
                                    ast::Pattern::Ident { name, .. } => {
                                        has_wildcard = true;
                                        arm_env.bind(name.clone(), scrutinee_ty.clone());
                                    }
                                    ast::Pattern::Wildcard(_) => {
                                        has_wildcard = true;
                                    }
                                    ast::Pattern::Enum { path, variant, .. } => {
                                        covered_variants.push(if path.is_empty() {
                                            variant.clone()
                                        } else {
                                            format!("{}::{}", path, variant)
                                        });
                                    }
                                    _ => {}
                                }
                            }
                            // Track coverage for remaining or-patterns
                            for pat in patterns.iter().skip(1) {
                                if let ast::Pattern::Enum { path, variant, .. } = pat {
                                    covered_variants.push(if path.is_empty() {
                                        variant.clone()
                                    } else {
                                        format!("{}::{}", path, variant)
                                    });
                                }
                            }
                        }
                        ast::Pattern::IntLit { .. }
                        | ast::Pattern::StringLit { .. }
                        | ast::Pattern::BoolLit { .. }
                        | ast::Pattern::CharLit { .. }
                        | ast::Pattern::FloatLit { .. }
                        | ast::Pattern::Tuple { .. } => {}
                    }
                    // Check guard expression if present
                    if let Some(ref guard) = arm.guard {
                        self.synthesize_expr(guard, &mut arm_env, sink);
                    }
                    // Synthesize arm body type with the arm's env
                    let arm_ty = self.synthesize_expr(&arm.body, &mut arm_env, sink);
                    if let Some(ref first) = result_ty {
                        if !self.types_compatible(&arm_ty, first)
                            && arm_ty != Type::Error
                            && *first != Type::Error
                        {
                            sink.emit(
                                Diagnostic::new(DiagnosticCode::E0205)
                                    .with_message("mismatched match arm types")
                                    .with_label(
                                        *span,
                                        format!("expected `{}`, found `{}`", first, arm_ty),
                                    ),
                            );
                        }
                    } else {
                        result_ty = Some(arm_ty);
                    }
                }
                // Check exhaustiveness for enum types
                if !has_wildcard {
                    if let Type::Enum(ref type_id) = scrutinee_ty {
                        // Find enum info by TypeId
                        let enum_entry = self
                            .enum_defs
                            .iter()
                            .find(|(_, info)| &info.type_id == type_id);
                        if let Some((enum_name, info)) = enum_entry {
                            let enum_name = enum_name.clone();
                            let all_variants: Vec<String> = info
                                .variants
                                .iter()
                                .map(|v| format!("{}::{}", enum_name, v.name))
                                .collect();
                            let missing: Vec<&String> = all_variants
                                .iter()
                                .filter(|v| !covered_variants.contains(v))
                                .collect();
                            if !missing.is_empty() {
                                sink.emit(
                                    Diagnostic::new(DiagnosticCode::E0204)
                                        .with_message("non-exhaustive match")
                                        .with_label(
                                            *span,
                                            format!(
                                                "missing patterns: {}",
                                                missing
                                                    .iter()
                                                    .map(|s| s.as_str())
                                                    .collect::<Vec<_>>()
                                                    .join(", ")
                                            ),
                                        ),
                                );
                            }
                        }
                    }
                }
                result_ty.unwrap_or(Type::Unit)
            }
            ast::Expr::Try { expr, span } => {
                let inner_ty = self.synthesize_expr(expr, env, sink);
                // Check that the current function returns Result
                let fn_returns_result =
                    matches!(&self.current_fn_return_type, Some(Type::Result(_, _)));
                if !fn_returns_result {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0210)
                            .with_message("? operator requires function to return Result")
                            .with_label(*span, "used here".to_string())
                            .with_note("add `-> Result[T, E]` to the function signature"),
                    );
                }
                // Check error type compatibility with From trait support
                if let (Type::Result(_, src_err), Some(Type::Result(_, dst_err))) =
                    (&inner_ty, &self.current_fn_return_type)
                {
                    // Use strict equality (not lenient types_compatible) for ? error checking
                    if src_err != dst_err {
                        // Resolve the actual type name for the destination error
                        let dst_name = self.type_name(dst_err);
                        let src_name = self.type_name(src_err);
                        let from_fn = format!("{}__from", dst_name);
                        let has_from = self
                            .method_table
                            .contains_key(&(dst_name.clone(), "from".to_string()));
                        if has_from {
                            // Record that this ? needs From conversion
                            self.method_resolutions
                                .insert(span.start, (from_fn.clone(), dst_name.clone()));
                            let expr_id = self.node_ids.fresh_expr();
                            self.typed_ast_map.register_span(span.start, expr_id);
                            self.typed_ast_map.insert_expr(
                                expr_id,
                                TypedExprInfo {
                                    ty: inner_ty.clone(),
                                    method_resolution: Some((from_fn, dst_name)),
                                },
                            );
                        } else {
                            sink.emit(
                                Diagnostic::new(DiagnosticCode::E0210)
                                    .with_message(format!(
                                        "? operator: error type `{}` is not compatible with function return error type `{}`",
                                        src_name, dst_name
                                    ))
                                    .with_label(*span, "error type mismatch".to_string())
                                    .with_note(format!(
                                        "implement `From<{}>` for `{}` to enable automatic conversion",
                                        src_name, dst_name
                                    )),
                            );
                        }
                    }
                }
                // The type of ? on Result<T, E> is T
                match inner_ty {
                    Type::Result(ok, _) => *ok,
                    _ => Type::Error,
                }
            }
            ast::Expr::Index { object, index, .. } => {
                let obj_ty = self.synthesize_expr(object, env, sink);
                let _idx_ty = self.synthesize_expr(index, env, sink);
                match obj_ty {
                    Type::Array(elem, _) => *elem,
                    _ => Type::I32,
                }
            }
            ast::Expr::ArrayRepeat { value, count, .. } => {
                let elem_ty = self.synthesize_expr(value, env, sink);
                if let ast::Expr::IntLit { value: n, .. } = count.as_ref() {
                    Type::Array(Box::new(elem_ty), *n as u64)
                } else {
                    Type::Error
                }
            }
            ast::Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.synthesize_expr(object, env, sink);
                if let Type::Struct(type_id) = &obj_ty {
                    if let Some(info) = self.struct_defs.values().find(|s| s.type_id == *type_id) {
                        if let Some((_, field_ty)) =
                            info.fields.iter().find(|(name, _)| name == field)
                        {
                            return field_ty.clone();
                        }
                    }
                }
                // Struct field access at Wasm level is always i32 (pointer)
                Type::I32
            }
            ast::Expr::StructInit { name, fields, .. } => {
                // Check if this is an enum struct variant: "EnumName::VariantName"
                if let Some((enum_name, _variant_name)) = name.split_once("::") {
                    let enum_tid = self.enum_defs.get(enum_name).map(|e| e.type_id);
                    if let Some(tid) = enum_tid {
                        for (_fname, fexpr) in fields {
                            self.synthesize_expr(fexpr, env, sink);
                        }
                        return Type::Enum(tid);
                    }
                }
                let type_id = self.struct_defs.get(name).map(|info| info.type_id);
                for (_fname, fexpr) in fields {
                    self.synthesize_expr(fexpr, env, sink);
                }
                if let Some(tid) = type_id {
                    Type::Struct(tid)
                } else {
                    Type::I32
                }
            }
            ast::Expr::Closure { params, body, .. } => {
                let mut param_types = Vec::new();
                let mut child_env = env.child();
                for p in params {
                    let ty = if let Some(ty_expr) = &p.ty {
                        self.resolve_type_expr(ty_expr)
                    } else {
                        Type::I32
                    };
                    child_env.bind(p.name.clone(), ty.clone());
                    param_types.push(ty);
                }
                let ret_ty = self.synthesize_expr(body, &mut child_env, sink);
                Type::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                }
            }
            ast::Expr::QualifiedIdent { module, name, .. } => {
                // Qualified enum variant or module function reference
                let qualified = format!("{}::{}", module, name);
                if let Some(sig) = self.fn_sigs.get(&qualified).cloned() {
                    Type::Function {
                        params: sig.params,
                        ret: Box::new(sig.ret),
                    }
                } else if let Some(sig) = self.fn_sigs.get(name).cloned() {
                    Type::Function {
                        params: sig.params,
                        ret: Box::new(sig.ret),
                    }
                } else if let Some(info) = self.enum_defs.get(module.as_str()) {
                    let variant = info.variants.iter().find(|v| v.name == *name);
                    match variant {
                        Some(v) if v.fields.is_empty() => {
                            // Unit variant: Direction::South → Type::Enum(Direction)
                            Type::Enum(info.type_id)
                        }
                        Some(v) => {
                            // Tuple variant constructor: Color::Rgb → fn(fields...) -> Enum
                            Type::Function {
                                params: v.fields.clone(),
                                ret: Box::new(Type::Enum(info.type_id)),
                            }
                        }
                        None => Type::Enum(info.type_id),
                    }
                } else {
                    // Qualified names not yet resolvable
                    Type::Error
                }
            }
        }
    }

    fn check_binary_op(
        &mut self,
        op: &ast::BinOp,
        left: &Type,
        right: &Type,
        span: Span,
        sink: &mut DiagnosticSink,
    ) -> Type {
        use ast::BinOp::*;
        if *left == Type::Error || *right == Type::Error {
            return Type::Error;
        }
        // Check for operator overloading on struct types
        if let (Type::Struct(left_id), Type::Struct(right_id)) = (left, right) {
            if left_id == right_id {
                let struct_name = self
                    .struct_defs
                    .values()
                    .find(|s| s.type_id == *left_id)
                    .map(|s| s.name.clone());
                if let Some(sname) = struct_name {
                    let op_method = match op {
                        Add => "add",
                        Sub => "sub",
                        Mul => "mul",
                        Div => "div",
                        Mod => "rem",
                        Eq | Ne => "eq",
                        Lt | Le | Gt | Ge => "cmp",
                        _ => "",
                    };
                    if !op_method.is_empty() {
                        let mangled = format!("{}__{}", sname, op_method);
                        if let Some(sig) = self.fn_sigs.get(&mangled).cloned() {
                            // Record method resolution for MIR lowering
                            self.method_resolutions
                                .insert(span.start, (mangled.clone(), sname.clone()));
                            let ret_ty = match op {
                                Eq | Ne | Lt | Le | Gt | Ge => Type::Bool,
                                _ => sig.ret,
                            };
                            let expr_id = self.node_ids.fresh_expr();
                            self.typed_ast_map.register_span(span.start, expr_id);
                            self.typed_ast_map.insert_expr(
                                expr_id,
                                TypedExprInfo {
                                    ty: ret_ty.clone(),
                                    method_resolution: Some((mangled, sname)),
                                },
                            );
                            // For comparison ops, return Bool
                            return ret_ty;
                        }
                    }
                }
            }
        }
        // Promote i32 to wider numeric types for mixed operations
        let (left, right) = match (left, right) {
            (Type::I64, Type::I32) | (Type::I32, Type::I64) => (&Type::I64, &Type::I64),
            (Type::F64, Type::I32) | (Type::I32, Type::F64) => (&Type::F64, &Type::F64),
            (Type::F64, Type::I64) | (Type::I64, Type::F64) => (&Type::F64, &Type::F64),
            _ => (left, right),
        };
        match op {
            Add | Sub | Mul | Div | Mod => {
                if left.is_numeric() && left == right {
                    left.clone()
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "cannot apply arithmetic operator to `{}` and `{}`",
                        left, right
                    )));
                    Type::Error
                }
            }
            Eq | Ne => {
                if left == right {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("cannot compare `{}` with `{}`", left, right)),
                    );
                    Type::Error
                }
            }
            Lt | Le | Gt | Ge => {
                if left.is_numeric() && left == right {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("cannot order `{}` and `{}`", left, right)),
                    );
                    Type::Error
                }
            }
            And | Or => {
                if *left == Type::Bool && *right == Type::Bool {
                    Type::Bool
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "logical operators require `bool`, found `{}` and `{}`",
                        left, right
                    )));
                    Type::Error
                }
            }
            BitAnd | BitOr | BitXor | Shl | Shr => {
                if left.is_integer() && left == right {
                    left.clone()
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "bitwise operators require integer types, found `{}` and `{}`",
                        left, right
                    )));
                    Type::Error
                }
            }
        }
    }

    fn check_unary_op(&self, op: &ast::UnaryOp, operand: &Type, sink: &mut DiagnosticSink) -> Type {
        if *operand == Type::Error {
            return Type::Error;
        }
        match op {
            ast::UnaryOp::Neg => {
                if operand.is_numeric() {
                    operand.clone()
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200)
                            .with_message(format!("cannot negate `{}`", operand)),
                    );
                    Type::Error
                }
            }
            ast::UnaryOp::Not => {
                if *operand == Type::Bool {
                    Type::Bool
                } else {
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                            "logical NOT requires `bool`, found `{}`",
                            operand
                        )),
                    );
                    Type::Error
                }
            }
            ast::UnaryOp::BitNot => {
                if operand.is_integer() {
                    operand.clone()
                } else {
                    sink.emit(Diagnostic::new(DiagnosticCode::E0200).with_message(format!(
                        "bitwise NOT requires integer, found `{}`",
                        operand
                    )));
                    Type::Error
                }
            }
        }
    }

    /// Get the user-visible name of a type (e.g., "AppError" instead of "enum#3").
    fn type_name(&self, ty: &Type) -> String {
        match ty {
            Type::Struct(id) => self
                .struct_defs
                .iter()
                .find(|(_, info)| info.type_id == *id)
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| format!("{}", ty)),
            Type::Enum(id) => self
                .enum_defs
                .iter()
                .find(|(_, info)| info.type_id == *id)
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| format!("{}", ty)),
            _ => format!("{}", ty),
        }
    }

    fn types_compatible(&self, a: &Type, b: &Type) -> bool {
        if *a == Type::Error || *b == Type::Error || *a == Type::Never || *b == Type::Never {
            return true;
        }
        if a == b {
            return true;
        }
        // Generic enum types (Option<T>, Result<T,E>) are represented as
        // Option/Result/Enum variants. Constructors produce I32 (pointer).
        // Structs are also heap pointers (i32). Be lenient for these compound types.
        if matches!(
            a,
            Type::Enum(_) | Type::Option(_) | Type::Result(_, _) | Type::Vec(_) | Type::Struct(_)
        ) || matches!(
            b,
            Type::Enum(_) | Type::Option(_) | Type::Result(_, _) | Type::Vec(_) | Type::Struct(_)
        ) {
            return true;
        }
        false
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}
