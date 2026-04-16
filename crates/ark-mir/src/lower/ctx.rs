//! Lowering context: locals, type maps, and closure bookkeeping for MIR lowering.

use std::collections::{HashMap, HashSet};

use ark_parser::ast;

use crate::mir::*;

pub(crate) struct LowerCtx {
    pub(super) locals: Vec<(String, LocalId)>,
    pub(super) next_local: u32,
    pub(super) string_locals: HashSet<u32>,
    pub(super) f64_locals: HashSet<u32>,
    pub(super) f32_locals: HashSet<u32>,
    pub(super) i64_locals: HashSet<u32>,
    pub(super) bool_locals: HashSet<u32>,
    pub(super) char_locals: HashSet<u32>,
    pub(super) enum_tags: HashMap<String, i32>,
    /// enum name -> variant info: (variant_name, field_count)
    pub(super) enum_variants: HashMap<String, Vec<(String, usize)>>,
    /// "EnumName::Variant" -> enum name (for reverse lookup)
    pub(super) variant_to_enum: HashMap<String, String>,
    /// Also support bare names like "Some", "None", "Ok", "Err"
    pub(super) bare_variant_tags: HashMap<String, (String, i32, usize)>, // name -> (enum, tag, field_count)
    /// struct name -> ordered (field name, field type name)
    pub(super) struct_defs: HashMap<String, Vec<(String, String)>>,
    /// local id -> struct type name
    pub(super) struct_typed_locals: HashMap<u32, String>,
    /// local id -> enum type name
    pub(super) enum_typed_locals: HashMap<u32, String>,
    /// local id -> variant-level payload type info: (variant_idx, field_idx) -> is_string
    /// Maps local_id -> mapping from (variant_name, field_index) -> is_string
    pub(super) enum_local_payload_strings: HashMap<u32, HashSet<(String, u32)>>,
    /// local id -> specialized enum name for concrete generic types (e.g., "Result_i64_String")
    pub(super) enum_local_specialized: HashMap<u32, String>,
    /// enum name -> [(variant_name, [payload_type_names])]
    pub(super) enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    /// "EnumName::VariantName" -> ordered field names (for struct variants)
    pub(super) enum_variant_field_names: HashMap<String, Vec<String>>,
    /// Locals known to hold Vec<String> values.
    pub(super) vec_string_locals: HashSet<u32>,
    /// Locals known to hold Vec<i64> values.
    pub(super) vec_i64_locals: HashSet<u32>,
    /// Locals known to hold Vec<f64> values.
    pub(super) vec_f64_locals: HashSet<u32>,
    /// Locals known to hold Vec<i32> values.
    pub(super) vec_i32_locals: HashSet<u32>,
    /// Locals known to hold Vec<StructName> values: local_id -> struct_name.
    pub(super) vec_struct_locals: HashMap<u32, String>,
    /// (struct_name, field_name) -> inner element struct type for Vec<Struct> fields.
    pub(super) vec_struct_fields: HashMap<(String, String), String>,
    /// Local to assign break values to (for loop-as-expression).
    pub(super) loop_result_local: Option<LocalId>,
    /// Function name -> return type expression (for resolving generic enum payloads in match).
    pub(super) fn_return_types: HashMap<String, ast::TypeExpr>,
    /// Set of user-defined function names (for function references).
    pub(super) user_fn_names: HashSet<String>,
    /// Type parameters of the function being lowered (for generic tuple support).
    pub(super) type_params: Vec<String>,
    /// Set of generic function names (functions with type_params).
    pub(super) generic_fn_names: HashSet<String>,
    /// Closure info: local_id -> (synthetic function name, captured variable names)
    pub(super) closure_locals: HashMap<u32, (String, Vec<String>)>,
    /// Pending synthetic closure functions to add to the module.
    pub(super) pending_closures: Vec<MirFunction>,
    /// Counter for generating unique closure names.
    pub(super) closure_counter: u32,
    /// Synthetic closure function name -> captured variable names (for call-site injection).
    pub(super) closure_fn_captures: HashMap<String, Vec<String>>,
    /// Method call resolutions from type checker: span_start -> (mangled_name, struct_name)
    pub(super) method_resolutions: HashMap<u32, (String, String)>,
}

impl LowerCtx {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn new(
        enum_tags: HashMap<String, i32>,
        struct_defs: HashMap<String, Vec<(String, String)>>,
        enum_variants: HashMap<String, Vec<(String, usize)>>,
        variant_to_enum: HashMap<String, String>,
        bare_variant_tags: HashMap<String, (String, i32, usize)>,
        enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
        enum_variant_field_names: HashMap<String, Vec<String>>,
        fn_return_types: HashMap<String, ast::TypeExpr>,
        user_fn_names: HashSet<String>,
        method_resolutions: HashMap<u32, (String, String)>,
        type_params: Vec<String>,
        generic_fn_names: HashSet<String>,
        vec_struct_fields: HashMap<(String, String), String>,
    ) -> Self {
        Self {
            locals: Vec::new(),
            next_local: 0,
            string_locals: HashSet::new(),
            f64_locals: HashSet::new(),
            f32_locals: HashSet::new(),
            i64_locals: HashSet::new(),
            bool_locals: HashSet::new(),
            char_locals: HashSet::new(),
            enum_tags,
            enum_variants,
            variant_to_enum,
            bare_variant_tags,
            struct_defs,
            struct_typed_locals: HashMap::new(),
            enum_typed_locals: HashMap::new(),
            enum_local_payload_strings: HashMap::new(),
            enum_local_specialized: HashMap::new(),
            enum_defs,
            enum_variant_field_names,
            vec_string_locals: HashSet::new(),
            vec_i64_locals: HashSet::new(),
            vec_f64_locals: HashSet::new(),
            vec_i32_locals: HashSet::new(),
            vec_struct_locals: HashMap::new(),
            vec_struct_fields,
            loop_result_local: None,
            fn_return_types,
            user_fn_names,
            type_params,
            generic_fn_names,
            closure_locals: HashMap::new(),
            pending_closures: Vec::new(),
            closure_counter: 0,
            closure_fn_captures: HashMap::new(),
            method_resolutions,
        }
    }

    pub(in crate::lower) fn declare_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        self.locals.push((name.to_string(), id));
        id
    }

    /// Check if an expression is a call to a generic function.
    pub(in crate::lower) fn is_generic_call(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call { callee, .. } => {
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    return self.generic_fn_names.contains(name);
                }
                false
            }
            _ => false,
        }
    }

    pub(in crate::lower) fn new_temp(&mut self) -> LocalId {
        self.declare_local(&format!("__tmp_{}", self.next_local))
    }

    pub(in crate::lower) fn lookup_local(&self, name: &str) -> Option<LocalId> {
        self.locals
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, id)| *id)
    }
}
