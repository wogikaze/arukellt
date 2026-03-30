//! Builtin function type signatures and prelude registration.

use crate::types::Type;

use super::{EnumInfo, FnSig, TypeChecker, VariantInfo};

impl TypeChecker {
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
            "__intrinsic_trim".into(),
            FnSig {
                name: "__intrinsic_trim".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_contains".into(),
            FnSig {
                name: "__intrinsic_contains".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String],
                ret: Type::Bool,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_char_at".into(),
            FnSig {
                name: "__intrinsic_char_at".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::I32],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_substring".into(),
            FnSig {
                name: "__intrinsic_substring".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::I32, Type::I32],
                ret: Type::String,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_replace".into(),
            FnSig {
                name: "__intrinsic_replace".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::String, Type::String],
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
            "__intrinsic_fs_write_bytes".into(),
            FnSig {
                name: "__intrinsic_fs_write_bytes".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::String, Type::Vec(Box::new(Type::I32))],
                ret: Type::Result(Box::new(Type::Unit), Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_memory_copy".into(),
            FnSig {
                name: "__intrinsic_memory_copy".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32, Type::I32],
                ret: Type::Unit,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_memory_fill".into(),
            FnSig {
                name: "__intrinsic_memory_fill".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32, Type::I32, Type::I32],
                ret: Type::Unit,
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
            "__intrinsic_Vec_new_i32_with_cap".into(),
            FnSig {
                name: "__intrinsic_Vec_new_i32_with_cap".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::Vec(Box::new(Type::I32)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_i64_with_cap".into(),
            FnSig {
                name: "__intrinsic_Vec_new_i64_with_cap".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::Vec(Box::new(Type::I64)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_Vec_new_f64_with_cap".into(),
            FnSig {
                name: "__intrinsic_Vec_new_f64_with_cap".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::Vec(Box::new(Type::F64)),
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

        // --- std::host::env builtins ---
        for name in &["args", "env::args"] {
            self.fn_sigs.insert(
                (*name).into(),
                FnSig {
                    name: (*name).into(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![],
                    ret: Type::Vec(Box::new(Type::String)),
                },
            );
        }
        for name in &["arg_count", "env::arg_count"] {
            self.fn_sigs.insert(
                (*name).into(),
                FnSig {
                    name: (*name).into(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![],
                    ret: Type::I32,
                },
            );
        }
        for name in &["arg_at", "env::arg_at"] {
            self.fn_sigs.insert(
                (*name).into(),
                FnSig {
                    name: (*name).into(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![Type::I32],
                    ret: Type::String,
                },
            );
        }
        for name in &["has_flag", "env::has_flag"] {
            self.fn_sigs.insert(
                (*name).into(),
                FnSig {
                    name: (*name).into(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![Type::String],
                    ret: Type::Bool,
                },
            );
        }
        for name in &["var", "env::var"] {
            self.fn_sigs.insert(
                (*name).into(),
                FnSig {
                    name: (*name).into(),
                    type_params: vec![],
                    type_param_bounds: vec![],
                    params: vec![Type::String],
                    ret: Type::Option(Box::new(Type::String)),
                },
            );
        }
        // __intrinsic_* counterparts for use in std/host/env.ark
        self.fn_sigs.insert(
            "__intrinsic_args".into(),
            FnSig {
                name: "__intrinsic_args".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::Vec(Box::new(Type::String)),
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_arg_count".into(),
            FnSig {
                name: "__intrinsic_arg_count".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![],
                ret: Type::I32,
            },
        );
        self.fn_sigs.insert(
            "__intrinsic_arg_at".into(),
            FnSig {
                name: "__intrinsic_arg_at".into(),
                type_params: vec![],
                type_param_bounds: vec![],
                params: vec![Type::I32],
                ret: Type::String,
            },
        );
    }
}
