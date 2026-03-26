//! Type checker implementation with bidirectional inference.

use ark_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use ark_parser::ast;
use ark_resolve::ResolvedModule;

use crate::types::{Type, TypeId};
use std::collections::{HashMap, HashSet};

/// Type environment for tracking variable types.
#[derive(Debug)]
pub struct TypeEnv {
    bindings: HashMap<String, Type>,
    mutable_vars: HashSet<String>,
    parent: Option<Box<TypeEnv>>,
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
}

/// Information about a function signature.
#[derive(Debug, Clone)]
pub struct FnSig {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Type>,
    pub ret: Type,
}

/// The main type checker.
#[derive(Debug)]
pub struct TypeChecker {
    pub struct_defs: HashMap<String, StructInfo>,
    pub enum_defs: HashMap<String, EnumInfo>,
    pub fn_sigs: HashMap<String, FnSig>,
    next_type_id: u32,
    next_type_var: u32,
    current_fn_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            fn_sigs: HashMap::new(),
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

    fn fresh_type_var(&mut self) -> Type {
        let id = self.next_type_var;
        self.next_type_var += 1;
        Type::TypeVar(id)
    }

    /// Register builtin types (Option, Result, Vec, String).
    pub fn register_builtins(&mut self) {
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
                    },
                    VariantInfo {
                        name: "Some".into(),
                        fields: vec![Type::TypeVar(0)],
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
                    },
                    VariantInfo {
                        name: "Err".into(),
                        fields: vec![Type::TypeVar(1)],
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
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_print".into(),
            FnSig {
                name: "__intrinsic_print".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_eprintln".into(),
            FnSig {
                name: "__intrinsic_eprintln".into(),
                type_params: vec![],
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
                params: vec![],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_concat".into(),
            FnSig {
                name: "__intrinsic_concat".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_string_clone".into(),
            FnSig {
                name: "__intrinsic_string_clone".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_starts_with".into(),
            FnSig {
                name: "__intrinsic_starts_with".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_ends_with".into(),
            FnSig {
                name: "__intrinsic_ends_with".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_to_lower".into(),
            FnSig {
                name: "__intrinsic_to_lower".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_to_upper".into(),
            FnSig {
                name: "__intrinsic_to_upper".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_string_slice".into(),
            FnSig {
                name: "__intrinsic_string_slice".into(),
                type_params: vec![],
                params: vec![Type::String, Type::I32, Type::I32],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_i32_to_string".into(),
            FnSig {
                name: "__intrinsic_i32_to_string".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_i64_to_string".into(),
            FnSig {
                name: "__intrinsic_i64_to_string".into(),
                type_params: vec![],
                params: vec![Type::I64],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_f64_to_string".into(),
            FnSig {
                name: "__intrinsic_f64_to_string".into(),
                type_params: vec![],
                params: vec![Type::F64],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_bool_to_string".into(),
            FnSig {
                name: "__intrinsic_bool_to_string".into(),
                type_params: vec![],
                params: vec![Type::Bool],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_char_to_string".into(),
            FnSig {
                name: "__intrinsic_char_to_string".into(),
                type_params: vec![],
                params: vec![Type::Char],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_parse_i32".into(),
            FnSig {
                name: "__intrinsic_parse_i32".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_sqrt".into(),
            FnSig {
                name: "__intrinsic_sqrt".into(),
                type_params: vec![],
                params: vec![Type::F64],
                ret: Type::F64,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_abs".into(),
            FnSig {
                name: "__intrinsic_abs".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_min".into(),
            FnSig {
                name: "__intrinsic_min".into(),
                type_params: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_max".into(),
            FnSig {
                name: "__intrinsic_max".into(),
                type_params: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_panic".into(),
            FnSig {
                name: "__intrinsic_panic".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_i32".into(),
            FnSig {
                name: "__intrinsic_Vec_new_i32".into(),
                type_params: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_i64".into(),
            FnSig {
                name: "__intrinsic_Vec_new_i64".into(),
                type_params: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::I64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_f64".into(),
            FnSig {
                name: "__intrinsic_Vec_new_f64".into(),
                type_params: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::F64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_String".into(),
            FnSig {
                name: "__intrinsic_Vec_new_String".into(),
                type_params: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_sort_i32".into(),
            FnSig {
                name: "__intrinsic_sort_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_sort_String".into(),
            FnSig {
                name: "__intrinsic_sort_String".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::String))],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_map_i32_i32".into(),
            FnSig {
                name: "__intrinsic_map_i32_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_filter_i32".into(),
            FnSig {
                name: "__intrinsic_filter_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_fold_i32_i32".into(),
            FnSig {
                name: "__intrinsic_fold_i32_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_map_option_i32_i32".into(),
            FnSig {
                name: "__intrinsic_map_option_i32_i32".into(),
                type_params: vec![],
                params: vec![Type::Option(Box::new(Type::I32)), Type::I32],
                ret: Type::Option(Box::new(Type::I32)),
            },
        );

        // Conversion functions (user-facing; still needed as fallback until AST sigs take over)
        self.fn_sigs.insert(
            "i32_to_string".into(),
            FnSig {
                name: "i32_to_string".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "i64_to_string".into(),
            FnSig {
                name: "i64_to_string".into(),
                type_params: vec![],
                params: vec![Type::I64],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "bool_to_string".into(),
            FnSig {
                name: "bool_to_string".into(),
                type_params: vec![],
                params: vec![Type::Bool],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_string_from".into(),
            FnSig {
                name: "__intrinsic_string_from".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "String_from".into(),
            FnSig {
                name: "String_from".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "char_to_string".into(),
            FnSig {
                name: "char_to_string".into(),
                type_params: vec![],
                params: vec![Type::Char],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "f64_to_string".into(),
            FnSig {
                name: "f64_to_string".into(),
                type_params: vec![],
                params: vec![Type::F64],
                ret: Type::String,
            },
        );
        // Polymorphic to_string (accepts any type, dispatched in MIR/emitter)
        self.fn_sigs.insert(
            "to_string".into(),
            FnSig {
                name: "to_string".into(),
                type_params: vec![],
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
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "eq".into(),
            FnSig {
                name: "eq".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        // String concatenation
        self.fn_sigs.insert(
            "concat".into(),
            FnSig {
                name: "concat".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::String,
            },
        );
        // parse_i32: String -> Result<i32, String>
        self.fn_sigs.insert(
            "parse_i32".into(),
            FnSig {
                name: "parse_i32".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::I32,
            },
        );
        // Enum variant constructors (treated as functions for type checking)
        self.fn_sigs.insert(
            "Some".into(),
            FnSig {
                name: "Some".into(),
                type_params: vec![],
                params: vec![Type::I32], // generic but we treat as i32 for now
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "Ok".into(),
            FnSig {
                name: "Ok".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "Err".into(),
            FnSig {
                name: "Err".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );

        // Vec stdlib
        self.fn_sigs.insert(
            "Vec_new_i32".into(),
            FnSig {
                name: "Vec_new_i32".into(),
                type_params: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "push".into(),
            FnSig {
                name: "push".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "pop".into(),
            FnSig {
                name: "pop".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::Option(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "len".into(),
            FnSig {
                name: "len".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "get".into(),
            FnSig {
                name: "get".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::Option(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "get_unchecked".into(),
            FnSig {
                name: "get_unchecked".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "set".into(),
            FnSig {
                name: "set".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32, Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "sort_i32".into(),
            FnSig {
                name: "sort_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32))],
                ret: Type::Unit,
            },
        );

        // String stdlib
        self.fn_sigs.insert(
            "String_new".into(),
            FnSig {
                name: "String_new".into(),
                type_params: vec![],
                params: vec![],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "is_empty".into(),
            FnSig {
                name: "is_empty".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "slice".into(),
            FnSig {
                name: "slice".into(),
                type_params: vec![],
                params: vec![Type::String, Type::I32, Type::I32],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "starts_with".into(),
            FnSig {
                name: "starts_with".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "ends_with".into(),
            FnSig {
                name: "ends_with".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "split".into(),
            FnSig {
                name: "split".into(),
                type_params: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "join".into(),
            FnSig {
                name: "join".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::String)), Type::String],
                ret: Type::String,
            },
        );

        // Vec<String>
        self.fn_sigs.insert(
            "Vec_new_String".into(),
            FnSig {
                name: "Vec_new_String".into(),
                type_params: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        // Higher-order Vec functions
        self.fn_sigs.insert(
            "map_i32_i32".into(),
            FnSig {
                name: "map_i32_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32], // vec, fn ptr
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "filter_i32".into(),
            FnSig {
                name: "filter_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32], // vec, fn ptr
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "fold_i32_i32".into(),
            FnSig {
                name: "fold_i32_i32".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::I32)), Type::I32, Type::I32], // vec, init, fn ptr
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "map_option_i32_i32".into(),
            FnSig {
                name: "map_option_i32_i32".into(),
                type_params: vec![],
                params: vec![Type::Option(Box::new(Type::I32)), Type::I32], // option, fn ptr
                ret: Type::Option(Box::new(Type::I32)),
            },
        );

        // Box builtins (Box<T> is represented as an i32 pointer)
        self.fn_sigs.insert(
            "Box_new".into(),
            FnSig {
                name: "Box_new".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "unbox".into(),
            FnSig {
                name: "unbox".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "unwrap".into(),
            FnSig {
                name: "unwrap".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "unwrap_or".into(),
            FnSig {
                name: "unwrap_or".into(),
                type_params: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "is_some".into(),
            FnSig {
                name: "is_some".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "is_none".into(),
            FnSig {
                name: "is_none".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::Bool,
            },
        );

        // --- Missing stdlib: math builtins ---
        self.fn_sigs.insert(
            "sqrt".into(),
            FnSig {
                name: "sqrt".into(),
                type_params: vec![],
                params: vec![Type::F64],
                ret: Type::F64,
            },
        );
        self.fn_sigs.insert(
            "abs".into(),
            FnSig {
                name: "abs".into(),
                type_params: vec![],
                params: vec![Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "min".into(),
            FnSig {
                name: "min".into(),
                type_params: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "max".into(),
            FnSig {
                name: "max".into(),
                type_params: vec![],
                params: vec![Type::I32, Type::I32],
                ret: Type::I32,
            },
        );

        // --- Missing stdlib: panic ---
        self.fn_sigs.insert(
            "panic".into(),
            FnSig {
                name: "panic".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::Unit,
            },
        );

        // --- Missing stdlib: clone ---
        self.fn_sigs.insert(
            "clone".into(),
            FnSig {
                name: "clone".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );

        // --- Missing stdlib: string functions ---
        self.fn_sigs.insert(
            "push_char".into(),
            FnSig {
                name: "push_char".into(),
                type_params: vec![],
                params: vec![Type::String, Type::Char],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "to_lower".into(),
            FnSig {
                name: "to_lower".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "to_upper".into(),
            FnSig {
                name: "to_upper".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );

        // --- Missing stdlib: parse functions ---
        self.fn_sigs.insert(
            "parse_i64".into(),
            FnSig {
                name: "parse_i64".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::I64,
            },
        );
        self.fn_sigs.insert(
            "parse_f64".into(),
            FnSig {
                name: "parse_f64".into(),
                type_params: vec![],
                params: vec![Type::String],
                ret: Type::F64,
            },
        );

        // --- Missing stdlib: sort_String ---
        self.fn_sigs.insert(
            "sort_String".into(),
            FnSig {
                name: "sort_String".into(),
                type_params: vec![],
                params: vec![Type::Vec(Box::new(Type::String))],
                ret: Type::Unit,
            },
        );

        // --- Missing stdlib: f32_to_string ---
        self.fn_sigs.insert(
            "f32_to_string".into(),
            FnSig {
                name: "f32_to_string".into(),
                type_params: vec![],
                params: vec![Type::F32],
                ret: Type::String,
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
                            },
                            ast::Variant::Tuple { name, fields, .. } => VariantInfo {
                                name: name.clone(),
                                fields: fields.iter().map(|t| self.resolve_type_expr(t)).collect(),
                            },
                            ast::Variant::Struct { name, fields, .. } => VariantInfo {
                                name: name.clone(),
                                fields: fields
                                    .iter()
                                    .map(|f| self.resolve_type_expr(&f.ty))
                                    .collect(),
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
                            params,
                            ret,
                        },
                    );
                }
            }
        }

        // Type check function bodies
        for item in &resolved.module.items {
            if let ast::Item::FnDef(f) = item {
                self.check_function(f, sink);
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
                };
                let mut child_env = env.child();
                child_env.bind_mut(target.clone(), elem_ty);
                self.check_block(body, &mut child_env, &Type::Unit, sink);
            }
        }
    }

    /// Synthesize the type of an expression.
    fn synthesize_expr(
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
                    sink.emit(
                        Diagnostic::new(DiagnosticCode::E0100)
                            .with_label(*span, format!("unresolved name `{}`", name)),
                    );
                    Type::Error
                }
            }
            ast::Expr::Binary {
                left, op, right, ..
            } => {
                let left_ty = self.synthesize_expr(left, env, sink);
                let right_ty = self.synthesize_expr(right, env, sink);
                self.check_binary_op(op, &left_ty, &right_ty, sink)
            }
            ast::Expr::Unary { op, operand, .. } => {
                let operand_ty = self.synthesize_expr(operand, env, sink);
                self.check_unary_op(op, &operand_ty, sink)
            }
            ast::Expr::Call { callee, args, .. } => {
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
                                    // Polymorphic: accepts any primitive, returns String
                                    Type::String
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
                                            if let ast::Pattern::Ident { name, .. } = field_pat {
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
                        _ => {}
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
                            .with_label(*span, "used here".to_string()),
                    );
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
            _ => {
                // TODO: handle remaining expression types
                Type::Error
            }
        }
    }

    fn check_binary_op(
        &self,
        op: &ast::BinOp,
        left: &Type,
        right: &Type,
        sink: &mut DiagnosticSink,
    ) -> Type {
        use ast::BinOp::*;
        if *left == Type::Error || *right == Type::Error {
            return Type::Error;
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
