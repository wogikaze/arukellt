//! Canonical ABI adapter function generation for Component Model exports.
//!
//! When a component-exported function uses GC reference types (struct refs,
//! enum subtype refs) in its parameters or return type, wasm-tools cannot
//! directly wrap the core module into a component. This module generates
//! adapter ("shim") functions with flat scalar signatures that bridge between
//! the canonical ABI representation and the GC-native representation.
//!
//! ## Supported adaptations
//!
//! | Pattern | Param adaptation | Return adaptation |
//! |---------|-----------------|-------------------|
//! | Unit enum (Color) | i32 → struct.new variant | br_on_cast → i32 |
//! | Scalar record (Point) | flat i32s → struct.new | struct.get fields → retptr |
//!
//! Adapter functions are appended after user functions in the code section
//! and exported under the kebab-case name instead of the original function.

use super::Ctx;
use ark_mir::mir::MirModule;
use wasm_encoder::{BlockType, CodeSection, Function, HeapType, Instruction, ValType};

/// Describes how a single parameter needs to be adapted.
#[derive(Debug, Clone)]
pub(super) enum ParamAdaptation {
    /// Pass-through: no conversion needed (already i32/i64/f32/f64).
    Scalar,
    /// Unit-variant enum: i32 discriminant → construct GC variant subtype ref.
    UnitEnum {
        /// Ordered variant GC type indices.
        variant_type_indices: Vec<u32>,
        /// Base enum GC type index (for the ref type).
        base_type_idx: u32,
    },
    /// All-scalar-field record: N flat scalars → struct.new.
    ScalarRecord {
        /// GC struct type index.
        type_idx: u32,
        /// Fields: (field_name, wasm ValType).
        fields: Vec<(String, ValType)>,
    },
    /// String: (i32 ptr, i32 len) in linear memory → GC (array (mut i8)).
    String {
        /// GC array type index for the string type.
        string_type_idx: u32,
    },
    /// List: (i32 ptr, i32 len) in linear memory → GC vec struct.
    List {
        /// GC vec struct type index (e.g. vec_i32_ty).
        vec_type_idx: u32,
        /// GC backing array type index (e.g. arr_i32_ty).
        arr_type_idx: u32,
        /// Size of each element in linear memory (4 for i32/f32, 8 for i64/f64).
        elem_size: u32,
        /// Wasm value type of each element (I32, I64, F64).
        elem_valtype: ValType,
    },
    /// Option: (i32 discriminant, T payload) → GC Option enum ref.
    /// discriminant 0 = None, 1 = Some.
    OptionType {
        /// GC base type index for Option enum.
        base_type_idx: u32,
        /// GC type index for Some variant.
        some_type_idx: u32,
        /// GC type index for None variant.
        none_type_idx: u32,
        /// Wasm value type of payload.
        payload_valtype: ValType,
    },
    /// Result: (i32 discriminant, T/E payload) → GC Result enum ref.
    /// discriminant 0 = Ok, 1 = Err.
    ResultType {
        /// GC base type index for Result enum.
        base_type_idx: u32,
        /// GC type index for Ok variant.
        ok_type_idx: u32,
        /// GC type index for Err variant.
        err_type_idx: u32,
        /// Wasm value type of ok payload.
        ok_valtype: ValType,
        /// Wasm value type of err payload (ignored when err_is_string).
        err_valtype: ValType,
        /// True when the Err payload is a String (needs linear memory lifting).
        err_is_string: bool,
    },
}

/// Describes how the return value needs to be adapted.
#[derive(Debug, Clone)]
pub(super) enum ReturnAdaptation {
    /// Pass-through: no conversion needed.
    Scalar,
    /// Unit-variant enum: GC ref → i32 discriminant via br_on_cast.
    UnitEnum {
        variant_type_indices: Vec<u32>,
        base_type_idx: u32,
    },
    /// All-scalar-field record → flat scalars.
    /// For single-field records, the field is returned directly.
    /// For multi-field records, fields are written to linear memory
    /// and an i32 pointer is returned (canonical ABI export convention).
    ScalarRecord {
        type_idx: u32,
        fields: Vec<(String, ValType)>,
    },
    /// String: GC (array (mut i8)) → (i32 ptr, i32 len) in linear memory.
    /// Uses retptr convention (canonical ABI MAX_FLAT_RESULTS=1 for exports):
    /// an extra i32 param is appended, and (ptr, len) is written to retptr.
    String {
        /// GC array type index for the string type.
        string_type_idx: u32,
    },
    /// List: GC vec struct → (i32 ptr, i32 len) in linear memory.
    List {
        /// GC vec struct type index.
        vec_type_idx: u32,
        /// GC backing array type index.
        arr_type_idx: u32,
        /// Size of each element in linear memory.
        elem_size: u32,
        /// Wasm value type of each element.
        elem_valtype: ValType,
    },
    /// Option: GC Option enum ref → (i32 discriminant, T payload).
    OptionType {
        /// GC base type index for Option enum.
        base_type_idx: u32,
        /// GC type index for Some variant.
        some_type_idx: u32,
        /// Wasm value type of payload.
        payload_valtype: ValType,
    },
    /// Result: GC Result enum ref → (i32 discriminant, T/E payload).
    ResultType {
        /// GC base type index for Result enum.
        base_type_idx: u32,
        /// GC type index for Ok variant.
        ok_type_idx: u32,
        /// GC type index for Err variant.
        err_type_idx: u32,
        /// Wasm value type of ok payload.
        ok_valtype: ValType,
        /// Wasm value type of err payload (ignored when err_is_string).
        err_valtype: ValType,
        /// True when the Err payload is a String (needs linear memory lowering).
        err_is_string: bool,
    },
}

/// Full description of one canonical ABI adapter function.
#[derive(Debug, Clone)]
pub(super) struct CabiAdapter {
    /// Kebab-case export name (e.g. "next-color").
    pub export_name: String,
    /// Function index of the original GC-typed function.
    pub original_fn_idx: u32,
    /// Wasm function index assigned to this adapter.
    pub adapter_fn_idx: u32,
    /// Function type index for the adapter's flat signature.
    pub adapter_type_idx: u32,
    /// Per-parameter adaptations.
    pub param_adaptations: Vec<ParamAdaptation>,
    /// Return adaptation (None = void).
    pub return_adaptation: Option<ReturnAdaptation>,
}

/// Map a MIR type name to a Wasm ValType.
fn type_name_to_valtype(name: &str) -> ValType {
    match name {
        "i32" | "bool" | "char" | "u8" | "u16" | "u32" => ValType::I32,
        "i64" | "u64" => ValType::I64,
        "f32" => ValType::F32,
        "f64" => ValType::F64,
        _ => ValType::I32, // fallback
    }
}

/// Return the wider of two value types (for result<T,E> payload flattening).
fn wider_valtype(a: ValType, b: ValType) -> ValType {
    fn rank(v: ValType) -> u32 {
        match v {
            ValType::I32 | ValType::F32 => 1,
            ValType::I64 | ValType::F64 => 2,
            _ => 0,
        }
    }
    if rank(b) > rank(a) { b } else { a }
}

impl Ctx {
    /// Identify component-exported functions that need canonical ABI adapters
    /// and compute the adapter metadata.
    ///
    /// Returns a list of adapters.  Each adapter will be appended after user
    /// functions in the function/code sections.
    pub(super) fn compute_cabi_adapters(
        &mut self,
        mir: &MirModule,
        next_fn_idx: u32,
    ) -> Vec<CabiAdapter> {
        let mut adapters = Vec::new();
        let mut idx = next_fn_idx;

        for func in &mir.functions {
            if !func.is_exported {
                continue;
            }
            let name = &func.name;
            if name.starts_with("__") || name == "_start" || name == "main" {
                continue;
            }
            if !super::is_component_export_candidate(name) {
                continue;
            }

            // Look up the function's accurate type signature from type_table
            let sig = match mir.type_table.fn_sigs.get(name.as_str()) {
                Some(sig) => sig,
                None => continue,
            };

            let original_fn_idx = match self.fn_map.get(name.as_str()) {
                Some(&idx) => idx,
                None => continue,
            };

            let mut needs_adapter = false;
            let mut param_adaptations = Vec::new();
            let mut flat_params: Vec<ValType> = Vec::new();

            for type_name in &sig.params {
                if is_scalar_type_name(type_name) {
                    param_adaptations.push(ParamAdaptation::Scalar);
                    flat_params.push(type_name_to_valtype(type_name));
                } else if type_name == "String" {
                    param_adaptations.push(ParamAdaptation::String {
                        string_type_idx: self.string_ty,
                    });
                    flat_params.push(ValType::I32); // ptr
                    flat_params.push(ValType::I32); // len
                    needs_adapter = true;
                } else if type_name.starts_with("Vec<") && type_name.ends_with('>') {
                    let inner = &type_name[4..type_name.len() - 1];
                    if let Some((vec_ty, arr_ty, elem_size, elem_vt)) =
                        self.resolve_vec_types(inner)
                    {
                        param_adaptations.push(ParamAdaptation::List {
                            vec_type_idx: vec_ty,
                            arr_type_idx: arr_ty,
                            elem_size,
                            elem_valtype: elem_vt,
                        });
                        flat_params.push(ValType::I32); // ptr
                        flat_params.push(ValType::I32); // len
                        needs_adapter = true;
                    } else {
                        param_adaptations.push(ParamAdaptation::Scalar);
                        flat_params.push(ValType::I32);
                    }
                } else if type_name.starts_with("Option<") && type_name.ends_with('>') {
                    let inner = &type_name[7..type_name.len() - 1];
                    if is_scalar_type_name(inner) {
                        if let Some((base_idx, some_idx, none_idx, payload_vt)) =
                            self.resolve_option_types(inner)
                        {
                            param_adaptations.push(ParamAdaptation::OptionType {
                                base_type_idx: base_idx,
                                some_type_idx: some_idx,
                                none_type_idx: none_idx,
                                payload_valtype: payload_vt,
                            });
                            flat_params.push(ValType::I32); // discriminant
                            flat_params.push(payload_vt);   // payload
                            needs_adapter = true;
                        } else {
                            param_adaptations.push(ParamAdaptation::Scalar);
                            flat_params.push(ValType::I32);
                        }
                    } else {
                        param_adaptations.push(ParamAdaptation::Scalar);
                        flat_params.push(ValType::I32);
                    }
                } else if type_name.starts_with("Result<") && type_name.ends_with('>') {
                    let inner = &type_name[7..type_name.len() - 1];
                    // Split on ", " to get ok and err types
                    if let Some((ok_str, err_str)) = inner.split_once(", ") {
                        if is_scalar_type_name(ok_str) && (is_scalar_type_name(err_str) || err_str == "String") {
                            if let Some((base_idx, ok_idx, err_idx, ok_vt, err_vt)) =
                                self.resolve_result_types(ok_str, err_str)
                            {
                                // Use the wider of ok/err types for the flat payload
                                let payload_vt = wider_valtype(ok_vt, err_vt);
                                param_adaptations.push(ParamAdaptation::ResultType {
                                    base_type_idx: base_idx,
                                    ok_type_idx: ok_idx,
                                    err_type_idx: err_idx,
                                    ok_valtype: ok_vt,
                                    err_valtype: err_vt,
                                    err_is_string: err_str == "String",
                                });
                                flat_params.push(ValType::I32); // discriminant
                                flat_params.push(payload_vt);   // payload
                                needs_adapter = true;
                            } else {
                                param_adaptations.push(ParamAdaptation::Scalar);
                                flat_params.push(ValType::I32);
                            }
                        } else {
                            param_adaptations.push(ParamAdaptation::Scalar);
                            flat_params.push(ValType::I32);
                        }
                    } else {
                        param_adaptations.push(ParamAdaptation::Scalar);
                        flat_params.push(ValType::I32);
                    }
                } else if let Some(variants) = mir.enum_defs.get(type_name.as_str()) {
                    let all_unit = variants.iter().all(|(_, p)| p.is_empty());
                    if all_unit {
                        let variant_type_indices: Vec<u32> = variants
                            .iter()
                            .map(|(vname, _)| {
                                self.enum_variant_types
                                    .get(type_name.as_str())
                                    .and_then(|vs: &std::collections::HashMap<String, u32>| {
                                        vs.get(vname.as_str())
                                    })
                                    .copied()
                                    .unwrap_or(0)
                            })
                            .collect();
                        let base_type_idx = self
                            .enum_base_types
                            .get(type_name.as_str())
                            .copied()
                            .unwrap_or(0);
                        param_adaptations.push(ParamAdaptation::UnitEnum {
                            variant_type_indices,
                            base_type_idx,
                        });
                        flat_params.push(ValType::I32);
                        needs_adapter = true;
                    } else {
                        // Non-unit enum — not yet supported
                        param_adaptations.push(ParamAdaptation::Scalar);
                        flat_params.push(ValType::I32);
                    }
                } else if let Some(fields) = mir.struct_defs.get(type_name.as_str()) {
                    let all_scalar = fields.iter().all(|(_, ft)| is_scalar_type_name(ft));
                    if all_scalar {
                        let wasm_fields: Vec<(String, ValType)> = fields
                            .iter()
                            .map(|(n, t)| (n.clone(), type_name_to_valtype(t)))
                            .collect();
                        let type_idx = self
                            .struct_gc_types
                            .get(type_name.as_str())
                            .copied()
                            .unwrap_or(0);
                        for (_, vt) in &wasm_fields {
                            flat_params.push(*vt);
                        }
                        param_adaptations.push(ParamAdaptation::ScalarRecord {
                            type_idx,
                            fields: wasm_fields,
                        });
                        needs_adapter = true;
                    } else {
                        param_adaptations.push(ParamAdaptation::Scalar);
                        flat_params.push(ValType::I32);
                    }
                } else {
                    param_adaptations.push(ParamAdaptation::Scalar);
                    flat_params.push(ValType::I32);
                }
            }

            // Return type adaptation
            let ret_name = &sig.ret;
            let return_adaptation = if ret_name == "()" || ret_name == "unit" {
                None
            } else if is_scalar_type_name(ret_name) {
                Some(ReturnAdaptation::Scalar)
            } else if ret_name == "String" {
                needs_adapter = true;
                Some(ReturnAdaptation::String {
                    string_type_idx: self.string_ty,
                })
            } else if ret_name.starts_with("Vec<") && ret_name.ends_with('>') {
                let inner = &ret_name[4..ret_name.len() - 1];
                if let Some((vec_ty, arr_ty, elem_size, elem_vt)) =
                    self.resolve_vec_types(inner)
                {
                    needs_adapter = true;
                    Some(ReturnAdaptation::List {
                        vec_type_idx: vec_ty,
                        arr_type_idx: arr_ty,
                        elem_size,
                        elem_valtype: elem_vt,
                    })
                } else {
                    Some(ReturnAdaptation::Scalar)
                }
            } else if ret_name.starts_with("Option<") && ret_name.ends_with('>') {
                let inner = &ret_name[7..ret_name.len() - 1];
                if is_scalar_type_name(inner) {
                    if let Some((base_idx, some_idx, _none_idx, payload_vt)) =
                        self.resolve_option_types(inner)
                    {
                        needs_adapter = true;
                        Some(ReturnAdaptation::OptionType {
                            base_type_idx: base_idx,
                            some_type_idx: some_idx,
                            payload_valtype: payload_vt,
                        })
                    } else {
                        Some(ReturnAdaptation::Scalar)
                    }
                } else {
                    Some(ReturnAdaptation::Scalar)
                }
            } else if ret_name.starts_with("Result<") && ret_name.ends_with('>') {
                let inner = &ret_name[7..ret_name.len() - 1];
                if let Some((ok_str, err_str)) = inner.split_once(", ") {
                    if is_scalar_type_name(ok_str) && (is_scalar_type_name(err_str) || err_str == "String") {
                        if let Some((base_idx, ok_idx, err_idx, ok_vt, err_vt)) =
                            self.resolve_result_types(ok_str, err_str)
                        {
                            needs_adapter = true;
                            Some(ReturnAdaptation::ResultType {
                                base_type_idx: base_idx,
                                ok_type_idx: ok_idx,
                                err_type_idx: err_idx,
                                ok_valtype: ok_vt,
                                err_valtype: err_vt,
                                err_is_string: err_str == "String",
                            })
                        } else {
                            Some(ReturnAdaptation::Scalar)
                        }
                    } else {
                        Some(ReturnAdaptation::Scalar)
                    }
                } else {
                    Some(ReturnAdaptation::Scalar)
                }
            } else if let Some(variants) = mir.enum_defs.get(ret_name.as_str()) {
                let all_unit = variants.iter().all(|(_, p)| p.is_empty());
                if all_unit {
                    let variant_type_indices: Vec<u32> = variants
                        .iter()
                        .map(|(vname, _)| {
                            self.enum_variant_types
                                .get(ret_name.as_str())
                                .and_then(|vs| vs.get(vname.as_str()))
                                .copied()
                                .unwrap_or(0)
                        })
                        .collect();
                    let base_type_idx = self
                        .enum_base_types
                        .get(ret_name.as_str())
                        .copied()
                        .unwrap_or(0);
                    needs_adapter = true;
                    Some(ReturnAdaptation::UnitEnum {
                        variant_type_indices,
                        base_type_idx,
                    })
                } else {
                    Some(ReturnAdaptation::Scalar)
                }
            } else if let Some(fields) = mir.struct_defs.get(ret_name.as_str()) {
                let all_scalar = fields.iter().all(|(_, ft)| is_scalar_type_name(ft));
                if all_scalar {
                    let wasm_fields: Vec<(String, ValType)> = fields
                        .iter()
                        .map(|(n, t)| (n.clone(), type_name_to_valtype(t)))
                        .collect();
                    let type_idx = self
                        .struct_gc_types
                        .get(ret_name.as_str())
                        .copied()
                        .unwrap_or(0);
                    needs_adapter = true;
                    Some(ReturnAdaptation::ScalarRecord {
                        type_idx,
                        fields: wasm_fields,
                    })
                } else {
                    Some(ReturnAdaptation::Scalar)
                }
            } else {
                Some(ReturnAdaptation::Scalar)
            };

            if !needs_adapter {
                continue;
            }

            // Compute adapter function type: flat params → flat results
            // String return uses retptr convention (flat_count=2 > MAX_FLAT_RESULTS=1).
            let adapter_params = flat_params.clone();
            let adapter_results: Vec<ValType> = match &return_adaptation {
                None => vec![],
                Some(ReturnAdaptation::Scalar) => {
                    vec![type_name_to_valtype(ret_name)]
                }
                Some(ReturnAdaptation::UnitEnum { .. }) => vec![ValType::I32],
                Some(ReturnAdaptation::ScalarRecord { fields, .. }) => {
                    if fields.len() == 1 {
                        // Single-field record: return the field directly
                        vec![fields[0].1]
                    } else {
                        // Multi-field record: return i32 pointer to linear memory
                        vec![ValType::I32]
                    }
                }
                Some(ReturnAdaptation::String { .. }) => {
                    // Canonical ABI: string return uses a single i32 pointer
                    // to a (ptr, len) pair in linear memory.
                    // wasm-tools expects: params → i32 (pointer to result)
                    vec![ValType::I32]
                }
                Some(ReturnAdaptation::List { .. }) => {
                    // Canonical ABI: list return uses a single i32 pointer
                    // to a (ptr, len) pair in linear memory.
                    vec![ValType::I32]
                }
                Some(ReturnAdaptation::OptionType { .. }) => {
                    // option<T> flattens to 2 flat values (discriminant, payload)
                    // Canonical ABI export MAX_FLAT_RESULTS=1, so use retptr convention:
                    // extra i32 param for output pointer, returns void
                    vec![ValType::I32]
                }
                Some(ReturnAdaptation::ResultType { .. }) => {
                    // result<T, E> flattens to 2 flat values (discriminant, payload)
                    // Canonical ABI export MAX_FLAT_RESULTS=1, so use retptr convention
                    vec![ValType::I32]
                }
            };

            let adapter_type_idx = self.types.add_func(&adapter_params, &adapter_results);

            let export_name = name.replace('_', "-");
            adapters.push(CabiAdapter {
                export_name,
                original_fn_idx,
                adapter_fn_idx: idx,
                adapter_type_idx,
                param_adaptations,
                return_adaptation,
            });
            idx += 1;
        }

        adapters
    }

    /// Emit the code body for a canonical ABI adapter function.
    pub(super) fn emit_cabi_adapter_code(
        &mut self,
        codes: &mut CodeSection,
        adapter: &CabiAdapter,
    ) {
        // Pre-compute locals needed.
        let mut locals: Vec<(u32, ValType)> = Vec::new();
        let flat_param_count = self.count_flat_params(adapter);

        // Count extra locals for String/List params
        let mut extra_param_local_start = flat_param_count;
        let mut string_param_locals: Vec<(u32, u32)> = Vec::new(); // (arr_local, i_local)
        let mut list_param_locals: Vec<(u32, u32, u32)> = Vec::new(); // (vec_local, arr_local, i_local)
        for adaptation in &adapter.param_adaptations {
            match adaptation {
                ParamAdaptation::String { string_type_idx } => {
                    let arr_local = extra_param_local_start;
                    let i_local = extra_param_local_start + 1;
                    string_param_locals.push((arr_local, i_local));
                    locals.push((1, super::ref_nullable(*string_type_idx)));
                    locals.push((1, ValType::I32));
                    extra_param_local_start += 2;
                }
                ParamAdaptation::List {
                    vec_type_idx,
                    arr_type_idx,
                    ..
                } => {
                    let vec_local = extra_param_local_start;
                    let arr_local = extra_param_local_start + 1;
                    let i_local = extra_param_local_start + 2;
                    list_param_locals.push((vec_local, arr_local, i_local));
                    locals.push((1, super::ref_nullable(*vec_type_idx)));
                    locals.push((1, super::ref_nullable(*arr_type_idx)));
                    locals.push((1, ValType::I32));
                    extra_param_local_start += 3;
                }
                _ => {}
            }
        }

        let ret_local_start = extra_param_local_start;

        match &adapter.return_adaptation {
            Some(ReturnAdaptation::UnitEnum { base_type_idx, .. }) => {
                locals.push((1, super::ref_nullable(*base_type_idx)));
            }
            Some(ReturnAdaptation::ScalarRecord { type_idx, .. }) => {
                // Need a local to store the GC struct ref
                locals.push((1, super::ref_nullable(*type_idx)));
            }
            Some(ReturnAdaptation::String { string_type_idx }) => {
                // Need: arr ref local, len local, loop counter local
                locals.push((1, super::ref_nullable(*string_type_idx)));
                locals.push((1, ValType::I32)); // len
                locals.push((1, ValType::I32)); // loop counter i
            }
            Some(ReturnAdaptation::List { vec_type_idx, arr_type_idx, .. }) => {
                // Need: vec ref local, arr ref local, len local, loop counter local
                locals.push((1, super::ref_nullable(*vec_type_idx)));
                locals.push((1, super::ref_nullable(*arr_type_idx)));
                locals.push((1, ValType::I32)); // len
                locals.push((1, ValType::I32)); // loop counter i
            }
            Some(ReturnAdaptation::OptionType { base_type_idx, .. }) => {
                // Need a local to store the GC enum ref
                locals.push((1, super::ref_nullable(*base_type_idx)));
            }
            Some(ReturnAdaptation::ResultType { base_type_idx, err_is_string, .. }) => {
                // Need a local to store the GC enum ref
                locals.push((1, super::ref_nullable(*base_type_idx)));
                if *err_is_string {
                    // Extra locals for string lowering: string arr ref, len, loop counter
                    locals.push((1, super::ref_nullable(0))); // string arr ref (type 0 = i8 array)
                    locals.push((1, ValType::I32));            // len
                    locals.push((1, ValType::I32));            // loop counter
                }
            }
            _ => {}
        }

        let mut f = Function::new(locals);
        let mut flat_param_idx: u32 = 0;
        let mut string_param_idx: usize = 0;
        let mut list_param_idx: usize = 0;

        // Phase 1: Push adapted parameters onto the stack for the call
        for adaptation in &adapter.param_adaptations {
            match adaptation {
                ParamAdaptation::Scalar => {
                    f.instruction(&Instruction::LocalGet(flat_param_idx));
                    flat_param_idx += 1;
                }
                ParamAdaptation::UnitEnum {
                    variant_type_indices,
                    base_type_idx,
                } => {
                    emit_i32_to_enum_ref(
                        &mut f,
                        flat_param_idx,
                        variant_type_indices,
                        *base_type_idx,
                    );
                    flat_param_idx += 1;
                }
                ParamAdaptation::ScalarRecord { type_idx, fields } => {
                    for _ in fields.iter() {
                        f.instruction(&Instruction::LocalGet(flat_param_idx));
                        flat_param_idx += 1;
                    }
                    f.instruction(&Instruction::StructNew(*type_idx));
                }
                ParamAdaptation::String { string_type_idx } => {
                    let ptr_local = flat_param_idx;
                    let len_local = flat_param_idx + 1;
                    let (arr_local, i_local) = string_param_locals[string_param_idx];
                    string_param_idx += 1;
                    emit_linear_to_gc_string(
                        &mut f,
                        ptr_local,
                        len_local,
                        arr_local,
                        i_local,
                        *string_type_idx,
                    );
                    flat_param_idx += 2;
                }
                ParamAdaptation::List {
                    vec_type_idx,
                    arr_type_idx,
                    elem_size,
                    elem_valtype,
                } => {
                    let ptr_local = flat_param_idx;
                    let len_local = flat_param_idx + 1;
                    let (vec_local, arr_local, i_local) = list_param_locals[list_param_idx];
                    list_param_idx += 1;
                    emit_linear_to_gc_list(
                        &mut f,
                        ptr_local,
                        len_local,
                        vec_local,
                        arr_local,
                        i_local,
                        *vec_type_idx,
                        *arr_type_idx,
                        *elem_size,
                        *elem_valtype,
                    );
                    flat_param_idx += 2;
                }
                ParamAdaptation::OptionType {
                    base_type_idx,
                    some_type_idx,
                    none_type_idx,
                    payload_valtype: _,
                } => {
                    let disc_local = flat_param_idx;
                    let payload_local = flat_param_idx + 1;
                    // if disc == 0 → None, else → Some(payload)
                    f.instruction(&Instruction::LocalGet(disc_local));
                    f.instruction(&Instruction::I32Eqz);
                    f.instruction(&Instruction::If(BlockType::Result(
                        super::ref_nullable(*base_type_idx),
                    )));
                    // None branch
                    f.instruction(&Instruction::StructNew(*none_type_idx));
                    f.instruction(&Instruction::Else);
                    // Some branch
                    f.instruction(&Instruction::LocalGet(payload_local));
                    f.instruction(&Instruction::StructNew(*some_type_idx));
                    f.instruction(&Instruction::End);
                    flat_param_idx += 2;
                }
                ParamAdaptation::ResultType {
                    base_type_idx,
                    ok_type_idx,
                    err_type_idx,
                    ok_valtype,
                    err_valtype,
                    err_is_string: _,
                } => {
                    let disc_local = flat_param_idx;
                    let payload_local = flat_param_idx + 1;
                    // if disc == 0 → Ok(payload), else → Err(payload)
                    f.instruction(&Instruction::LocalGet(disc_local));
                    f.instruction(&Instruction::I32Eqz);
                    f.instruction(&Instruction::If(BlockType::Result(
                        super::ref_nullable(*base_type_idx),
                    )));
                    // Ok branch
                    f.instruction(&Instruction::LocalGet(payload_local));
                    // May need type conversion if ok_valtype != wider payload type
                    if *ok_valtype != *err_valtype {
                        // Truncate wider to narrower if needed
                        let wider = wider_valtype(*ok_valtype, *err_valtype);
                        if wider != *ok_valtype {
                            // payload is wider than ok — truncate
                            match (*ok_valtype, wider) {
                                (ValType::I32, ValType::I64) => {
                                    f.instruction(&Instruction::I32WrapI64);
                                }
                                _ => {} // other cases: keep as-is
                            }
                        }
                    }
                    f.instruction(&Instruction::StructNew(*ok_type_idx));
                    f.instruction(&Instruction::Else);
                    // Err branch
                    f.instruction(&Instruction::LocalGet(payload_local));
                    if *ok_valtype != *err_valtype {
                        let wider = wider_valtype(*ok_valtype, *err_valtype);
                        if wider != *err_valtype {
                            match (*err_valtype, wider) {
                                (ValType::I32, ValType::I64) => {
                                    f.instruction(&Instruction::I32WrapI64);
                                }
                                _ => {}
                            }
                        }
                    }
                    f.instruction(&Instruction::StructNew(*err_type_idx));
                    f.instruction(&Instruction::End);
                    flat_param_idx += 2;
                }
            }
        }

        // Phase 2: Call the original function
        f.instruction(&Instruction::Call(adapter.original_fn_idx));

        // Phase 3: Adapt the return value
        match &adapter.return_adaptation {
            None | Some(ReturnAdaptation::Scalar) => {}
            Some(ReturnAdaptation::UnitEnum {
                variant_type_indices,
                ..
            }) => {
                // Store call result in a local, then use ref.test chain
                let ref_local = flat_param_count;
                f.instruction(&Instruction::LocalSet(ref_local));
                emit_enum_ref_to_i32_via_ref_test(&mut f, ref_local, variant_type_indices);
            }
            Some(ReturnAdaptation::ScalarRecord {
                type_idx, fields, ..
            }) => {
                let ref_local = flat_param_count;
                f.instruction(&Instruction::LocalSet(ref_local));

                if fields.len() == 1 {
                    // Single-field record: return the field directly
                    f.instruction(&Instruction::LocalGet(ref_local));
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: *type_idx,
                        field_index: 0,
                    });
                } else {
                    // Multi-field record: write fields to linear memory, return pointer.
                    // Use heap_ptr global (global 0) as allocation point.
                    // global.get 0 → start pointer
                    // Write fields at start + offset
                    // Advance heap_ptr
                    // Return start pointer
                    f.instruction(&Instruction::GlobalGet(0)); // heap_ptr = start
                    let mut total_size: u64 = 0;
                    for (i, (_, vt)) in fields.iter().enumerate() {
                        // stack: [start_ptr]
                        // Duplicate start_ptr for the store address
                        f.instruction(&Instruction::GlobalGet(0));
                        if total_size > 0 {
                            f.instruction(&Instruction::I32Const(total_size as i32));
                            f.instruction(&Instruction::I32Add);
                        }
                        f.instruction(&Instruction::LocalGet(ref_local));
                        f.instruction(&Instruction::StructGet {
                            struct_type_index: *type_idx,
                            field_index: i as u32,
                        });
                        let size = match vt {
                            ValType::I32 => {
                                f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                                    offset: 0,
                                    align: 2,
                                    memory_index: 0,
                                }));
                                4u64
                            }
                            ValType::I64 => {
                                f.instruction(&Instruction::I64Store(wasm_encoder::MemArg {
                                    offset: 0,
                                    align: 3,
                                    memory_index: 0,
                                }));
                                8
                            }
                            ValType::F32 => {
                                f.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
                                    offset: 0,
                                    align: 2,
                                    memory_index: 0,
                                }));
                                4
                            }
                            ValType::F64 => {
                                f.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
                                    offset: 0,
                                    align: 3,
                                    memory_index: 0,
                                }));
                                8
                            }
                            _ => {
                                f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                                    offset: 0,
                                    align: 2,
                                    memory_index: 0,
                                }));
                                4
                            }
                        };
                        total_size += size;
                    }
                    // Advance heap_ptr: global.set 0 (global.get 0 + total_size)
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(total_size as i32));
                    f.instruction(&Instruction::I32Add);
                    f.instruction(&Instruction::GlobalSet(0));
                    // start_ptr is still on the stack from the initial global.get 0
                }
            }
            Some(ReturnAdaptation::String { string_type_idx }) => {
                // String return: copy GC array bytes to linear memory,
                // write (ptr, len) pair to a fresh allocation, return pointer to it.
                let arr_local = ret_local_start;
                let len_local = ret_local_start + 1;
                let i_local = ret_local_start + 2;
                f.instruction(&Instruction::LocalSet(arr_local));

                emit_gc_string_to_linear_return(
                    &mut f,
                    arr_local,
                    len_local,
                    i_local,
                    *string_type_idx,
                );
            }
            Some(ReturnAdaptation::List {
                vec_type_idx,
                arr_type_idx,
                elem_size,
                elem_valtype,
            }) => {
                // List return: extract array from GC vec struct, copy elements to
                // linear memory, write (ptr, len) pair, return pointer to pair.
                let vec_local = ret_local_start;
                let arr_local = ret_local_start + 1;
                let len_local = ret_local_start + 2;
                let i_local = ret_local_start + 3;
                f.instruction(&Instruction::LocalSet(vec_local));

                emit_gc_list_to_linear_return(
                    &mut f,
                    vec_local,
                    arr_local,
                    len_local,
                    i_local,
                    *vec_type_idx,
                    *arr_type_idx,
                    *elem_size,
                    *elem_valtype,
                );
            }
            Some(ReturnAdaptation::OptionType {
                base_type_idx: _,
                some_type_idx,
                payload_valtype,
            }) => {
                // Option return: GC ref → retptr pattern
                // Write (i32 discriminant, T payload) to linear memory, return pointer
                let ref_local = ret_local_start;
                f.instruction(&Instruction::LocalSet(ref_local));

                let payload_size = match payload_valtype {
                    ValType::I64 | ValType::F64 => 8u32,
                    _ => 4u32,
                };
                let total_size = 4 + payload_size; // disc (4) + payload

                // Save start pointer
                f.instruction(&Instruction::GlobalGet(0)); // heap_ptr → start

                // Try to cast to Some variant
                f.instruction(&Instruction::LocalGet(ref_local));
                f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(*some_type_idx)));
                f.instruction(&Instruction::If(BlockType::Empty));
                {
                    // It's Some: write disc=1, payload=value
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                        offset: 0, align: 2, memory_index: 0,
                    }));
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(4));
                    f.instruction(&Instruction::I32Add);
                    f.instruction(&Instruction::LocalGet(ref_local));
                    f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(*some_type_idx)));
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: *some_type_idx,
                        field_index: 0,
                    });
                    emit_store_valtype(&mut f, *payload_valtype);
                }
                f.instruction(&Instruction::Else);
                {
                    // It's None: write disc=0, payload=0
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                        offset: 0, align: 2, memory_index: 0,
                    }));
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(4));
                    f.instruction(&Instruction::I32Add);
                    emit_zero_valtype(&mut f, *payload_valtype);
                    emit_store_valtype(&mut f, *payload_valtype);
                }
                f.instruction(&Instruction::End);

                // Advance heap_ptr
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(total_size as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                // start pointer is still on stack from initial GlobalGet(0)
            }
            Some(ReturnAdaptation::ResultType {
                base_type_idx: _,
                ok_type_idx,
                err_type_idx,
                ok_valtype,
                err_valtype,
                err_is_string,
            }) => {
                // Result return: GC ref → retptr pattern
                let ref_local = ret_local_start;
                f.instruction(&Instruction::LocalSet(ref_local));

                if *err_is_string {
                    // Result<scalar, String>: retptr layout is
                    //   [disc:i32 @ 0, ptr:i32 @ 4, len:i32 @ 8] = 12 bytes
                    // String data goes after the struct in linear memory.
                    let str_arr_local = ret_local_start + 1;
                    let str_len_local = ret_local_start + 2;
                    let str_i_local = ret_local_start + 3;

                    // Save start pointer (retptr)
                    f.instruction(&Instruction::GlobalGet(0));

                    f.instruction(&Instruction::LocalGet(ref_local));
                    f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(*ok_type_idx)));
                    f.instruction(&Instruction::If(BlockType::Empty));
                    {
                        // Ok case: disc=0, ok_value at retptr+4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 0, align: 2, memory_index: 0,
                        }));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalGet(ref_local));
                        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(*ok_type_idx)));
                        f.instruction(&Instruction::StructGet {
                            struct_type_index: *ok_type_idx,
                            field_index: 0,
                        });
                        emit_store_valtype(&mut f, *ok_valtype);
                        // Advance heap_ptr by 12 (reserve full struct even for Ok)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    f.instruction(&Instruction::Else);
                    {
                        // Err case: disc=1, then lower string to linear memory
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 0, align: 2, memory_index: 0,
                        }));
                        // Get string ref from Err variant
                        f.instruction(&Instruction::LocalGet(ref_local));
                        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(*err_type_idx)));
                        f.instruction(&Instruction::StructGet {
                            struct_type_index: *err_type_idx,
                            field_index: 0,
                        });
                        f.instruction(&Instruction::LocalSet(str_arr_local));
                        // Get string length
                        f.instruction(&Instruction::LocalGet(str_arr_local));
                        f.instruction(&Instruction::ArrayLen);
                        f.instruction(&Instruction::LocalSet(str_len_local));
                        // String data starts at retptr + 12
                        // Copy bytes from GC array to linear memory
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalSet(str_i_local));
                        f.instruction(&Instruction::Block(BlockType::Empty));
                        f.instruction(&Instruction::Loop(BlockType::Empty));
                        f.instruction(&Instruction::LocalGet(str_i_local));
                        f.instruction(&Instruction::LocalGet(str_len_local));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // memory[retptr + 12 + i] = arr[i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalGet(str_i_local));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalGet(str_arr_local));
                        f.instruction(&Instruction::LocalGet(str_i_local));
                        f.instruction(&Instruction::ArrayGetU(0)); // type 0 = i8 array
                        f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
                            offset: 0, align: 0, memory_index: 0,
                        }));
                        f.instruction(&Instruction::LocalGet(str_i_local));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalSet(str_i_local));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Write str_ptr = retptr + 12 at retptr+4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 0, align: 2, memory_index: 0,
                        }));
                        // Write str_len at retptr+8
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalGet(str_len_local));
                        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 0, align: 2, memory_index: 0,
                        }));
                        // Advance heap_ptr by 12 + len (aligned to 4)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalGet(str_len_local));
                        f.instruction(&Instruction::I32Add);
                        // Align to 4 bytes: (ptr + 3) & ~3
                        f.instruction(&Instruction::I32Const(3));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(-4i32));
                        f.instruction(&Instruction::I32And);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    f.instruction(&Instruction::End);
                    // start pointer is still on stack from initial GlobalGet(0)
                } else {
                    // Result<scalar, scalar>: retptr layout is
                    //   [disc:i32 @ 0, wider(T,E) @ 4]
                    let payload_vt = wider_valtype(*ok_valtype, *err_valtype);

                    let payload_size = match payload_vt {
                        ValType::I64 | ValType::F64 => 8u32,
                        _ => 4u32,
                    };
                    let total_size = 4 + payload_size;

                    // Save start pointer
                    f.instruction(&Instruction::GlobalGet(0));

                    // Try to cast to Ok variant
                    f.instruction(&Instruction::LocalGet(ref_local));
                    f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(*ok_type_idx)));
                    f.instruction(&Instruction::If(BlockType::Empty));
                    {
                        // It's Ok: write disc=0, payload=ok_value
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 0, align: 2, memory_index: 0,
                        }));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalGet(ref_local));
                        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(*ok_type_idx)));
                        f.instruction(&Instruction::StructGet {
                            struct_type_index: *ok_type_idx,
                            field_index: 0,
                        });
                        if *ok_valtype != payload_vt {
                            if let (ValType::I32, ValType::I64) = (*ok_valtype, payload_vt) {
                                f.instruction(&Instruction::I64ExtendI32S);
                            }
                        }
                        emit_store_valtype(&mut f, payload_vt);
                    }
                    f.instruction(&Instruction::Else);
                    {
                        // It's Err: write disc=1, payload=err_value
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 0, align: 2, memory_index: 0,
                        }));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::LocalGet(ref_local));
                        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(*err_type_idx)));
                        f.instruction(&Instruction::StructGet {
                            struct_type_index: *err_type_idx,
                            field_index: 0,
                        });
                        if *err_valtype != payload_vt {
                            if let (ValType::I32, ValType::I64) = (*err_valtype, payload_vt) {
                                f.instruction(&Instruction::I64ExtendI32S);
                            }
                        }
                        emit_store_valtype(&mut f, payload_vt);
                    }
                    f.instruction(&Instruction::End);

                    // Advance heap_ptr
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(total_size as i32));
                    f.instruction(&Instruction::I32Add);
                    f.instruction(&Instruction::GlobalSet(0));
                    // start pointer on stack
                }
            }
        }

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Count the total number of flat params (including retptr if applicable).
    fn count_flat_params(&self, adapter: &CabiAdapter) -> u32 {
        let mut count: u32 = 0;
        for adaptation in &adapter.param_adaptations {
            match adaptation {
                ParamAdaptation::Scalar => count += 1,
                ParamAdaptation::UnitEnum { .. } => count += 1,
                ParamAdaptation::ScalarRecord { fields, .. } => count += fields.len() as u32,
                ParamAdaptation::String { .. } => count += 2, // ptr + len
                ParamAdaptation::List { .. } => count += 2,   // ptr + len
                ParamAdaptation::OptionType { .. } => count += 2, // discriminant + payload
                ParamAdaptation::ResultType { .. } => count += 2, // discriminant + payload
            }
        }
        // String return uses i32 result pointer, no extra param needed
        count
    }

    /// Resolve Vec<inner> to its GC type indices and element layout info.
    /// Returns (vec_type_idx, arr_type_idx, elem_size, elem_valtype).
    fn resolve_vec_types(&self, inner: &str) -> Option<(u32, u32, u32, ValType)> {
        match inner {
            "i32" => Some((self.vec_i32_ty, self.arr_i32_ty, 4, ValType::I32)),
            "i64" => Some((self.vec_i64_ty, self.arr_i64_ty, 8, ValType::I64)),
            "f64" => Some((self.vec_f64_ty, self.arr_f64_ty, 8, ValType::F64)),
            _ => None,
        }
    }

    /// Resolve Option<T> GC types: (base_idx, some_idx, none_idx, payload_valtype).
    fn resolve_option_types(&self, inner: &str) -> Option<(u32, u32, u32, ValType)> {
        // Option is always the enum named "Option" (or "Option_String" for String payload)
        let enum_name = if inner == "String" {
            "Option_String"
        } else {
            "Option"
        };
        let base_idx = self.enum_base_types.get(enum_name).copied()?;
        let variants = self.enum_variant_types.get(enum_name)?;
        let some_idx = variants.get("Some").copied()?;
        let none_idx = variants.get("None").copied()?;
        let payload_vt = type_name_to_valtype(inner);
        Some((base_idx, some_idx, none_idx, payload_vt))
    }

    /// Resolve Result<T, E> GC types: (base_idx, ok_idx, err_idx, ok_valtype, err_valtype).
    fn resolve_result_types(
        &self,
        ok_inner: &str,
        err_inner: &str,
    ) -> Option<(u32, u32, u32, ValType, ValType)> {
        // Result may have specialized names like "Result_i64_String"
        let enum_name = if ok_inner == "i32" && err_inner == "String" {
            "Result"
        } else {
            // Try specialized name
            let specialized = format!("Result_{}_{}", ok_inner, err_inner);
            if self.enum_base_types.contains_key(specialized.as_str()) {
                // Found specialized name — use it
                return self.resolve_result_types_named(&specialized, ok_inner, err_inner);
            }
            "Result"
        };
        self.resolve_result_types_named(enum_name, ok_inner, err_inner)
    }

    fn resolve_result_types_named(
        &self,
        enum_name: &str,
        ok_inner: &str,
        err_inner: &str,
    ) -> Option<(u32, u32, u32, ValType, ValType)> {
        let base_idx = self.enum_base_types.get(enum_name).copied()?;
        let variants = self.enum_variant_types.get(enum_name)?;
        let ok_idx = variants.get("Ok").copied()?;
        let err_idx = variants.get("Err").copied()?;
        let ok_vt = type_name_to_valtype(ok_inner);
        let err_vt = type_name_to_valtype(err_inner);
        Some((base_idx, ok_idx, err_idx, ok_vt, err_vt))
    }
}

/// Emit a store instruction for the given value type.
/// Assumes the address and value are already on the stack.
fn emit_store_valtype(f: &mut Function, vt: ValType) {
    match vt {
        ValType::I64 => {
            f.instruction(&Instruction::I64Store(wasm_encoder::MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        ValType::F64 => {
            f.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        ValType::F32 => {
            f.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
        }
        _ => {
            f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
        }
    }
}

/// Emit a zero constant for the given value type.
fn emit_zero_valtype(f: &mut Function, vt: ValType) {
    match vt {
        ValType::I64 => {
            f.instruction(&Instruction::I64Const(0));
        }
        ValType::F64 => {
            f.instruction(&Instruction::F64Const(0.0));
        }
        ValType::F32 => {
            f.instruction(&Instruction::F32Const(0.0));
        }
        _ => {
            f.instruction(&Instruction::I32Const(0));
        }
    }
}

/// Emit instructions to lift a string from linear memory (ptr, len) into a GC array.
///
/// Creates a new `(array (mut i8))` of the given length, then copies bytes
/// from linear memory one at a time:
/// ```text
/// local.get $len
/// i32.const 0           ;; fill value
/// array.new $string     ;; arr = new i8[len]
/// local.set $arr
/// i32.const 0
/// local.set $i
/// block $break
///   loop $loop
///     local.get $i
///     local.get $len
///     i32.ge_u
///     br_if $break
///     local.get $arr      ;; target array
///     local.get $i        ;; index
///     local.get $ptr
///     local.get $i
///     i32.add
///     i32.load8_u         ;; byte from linear memory
///     array.set $string
///     local.get $i
///     i32.const 1
///     i32.add
///     local.set $i
///     br $loop
///   end
/// end
/// local.get $arr          ;; leave arr ref on stack
/// ```
fn emit_linear_to_gc_string(
    f: &mut Function,
    ptr_local: u32,
    len_local: u32,
    arr_local: u32,
    i_local: u32,
    string_type_idx: u32,
) {
    // arr = array.new $string (fill=0, len)
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32Const(0)); // fill value
    f.instruction(&Instruction::ArrayNew(string_type_idx));
    f.instruction(&Instruction::LocalSet(arr_local));

    // i = 0
    f.instruction(&Instruction::I32Const(0));
    f.instruction(&Instruction::LocalSet(i_local));

    // block $break
    f.instruction(&Instruction::Block(BlockType::Empty));
    // loop $loop
    f.instruction(&Instruction::Loop(BlockType::Empty));

    // if i >= len, break
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32GeU);
    f.instruction(&Instruction::BrIf(1)); // br $break

    // arr[i] = memory[ptr + i]
    f.instruction(&Instruction::LocalGet(arr_local));
    f.instruction(&Instruction::LocalGet(i_local));
    // load byte: i32.load8_u (ptr + i)
    f.instruction(&Instruction::LocalGet(ptr_local));
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&Instruction::ArraySet(string_type_idx));

    // i++
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalSet(i_local));

    f.instruction(&Instruction::Br(0)); // br $loop
    f.instruction(&Instruction::End); // end loop
    f.instruction(&Instruction::End); // end block

    // Leave arr ref on stack for the call
    f.instruction(&Instruction::LocalGet(arr_local));
}

/// Emit instructions to lower a GC string array to linear memory and return
/// an i32 pointer to a (ptr, len) pair.
///
/// The GC array ref is already in `arr_local`. This function:
/// 1. Gets the array length
/// 2. Copies bytes from GC array to linear memory (from heap_ptr)
/// 3. Allocates 8 more bytes for the (ptr, len) result pair
/// 4. Writes ptr and len to the pair
/// 5. Returns the pair pointer
fn emit_gc_string_to_linear_return(
    f: &mut Function,
    arr_local: u32,
    len_local: u32,
    i_local: u32,
    string_type_idx: u32,
) {
    // len = array.len(arr)
    f.instruction(&Instruction::LocalGet(arr_local));
    f.instruction(&Instruction::ArrayLen);
    f.instruction(&Instruction::LocalSet(len_local));

    // string_data_ptr = heap_ptr (saved in i_local temporarily)
    f.instruction(&Instruction::GlobalGet(0));
    f.instruction(&Instruction::LocalSet(i_local));

    // Copy loop: memory[string_data_ptr + idx] = arr[idx]
    // We'll reuse len_local as loop bound and manage a counter on the stack.
    // Actually, let's use a separate approach: loop with i_local as counter
    // after saving string_data_ptr. We need i_local for two purposes, so
    // let's save string_data_ptr, then reset i_local for the loop.

    // Save string_data_ptr on the stack, use i_local for loop
    // Actually, we can derive string_data_ptr from (global.get 0 - len) after the loop.
    // Simpler: save it, loop, then restore.

    // Reset i_local to 0 for loop counter
    f.instruction(&Instruction::I32Const(0));
    f.instruction(&Instruction::LocalSet(i_local));

    f.instruction(&Instruction::Block(BlockType::Empty));
    f.instruction(&Instruction::Loop(BlockType::Empty));

    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32GeU);
    f.instruction(&Instruction::BrIf(1)); // break

    // memory[heap_ptr + i] = arr[i]
    f.instruction(&Instruction::GlobalGet(0)); // heap_ptr
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalGet(arr_local));
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::ArrayGetU(string_type_idx));
    f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));

    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalSet(i_local));

    f.instruction(&Instruction::Br(0));
    f.instruction(&Instruction::End); // end loop
    f.instruction(&Instruction::End); // end block

    // string_data_ptr was saved before loop, but we used i_local for loop.
    // Recalculate: string_data_ptr = heap_ptr (unchanged during loop)
    // Actually heap_ptr hasn't changed — we only wrote TO linear memory,
    // we didn't advance heap_ptr yet. So global.get 0 still = string_data_ptr.

    // Advance heap_ptr past string bytes
    f.instruction(&Instruction::GlobalGet(0));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::GlobalSet(0));

    // result_ptr = new heap_ptr (where we'll write the pair)
    f.instruction(&Instruction::GlobalGet(0));
    f.instruction(&Instruction::LocalSet(i_local)); // i_local = result_ptr

    // Write string_data_ptr at result_ptr+0
    // string_data_ptr = result_ptr - len = global.get(0) - len
    // But simpler: result_ptr - len = old heap_ptr before advance
    // old_heap_ptr = result_ptr - len
    f.instruction(&Instruction::LocalGet(i_local)); // addr = result_ptr
    f.instruction(&Instruction::LocalGet(i_local)); // value = result_ptr
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32Sub); // value = result_ptr - len = string_data_ptr
    f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));

    // Write len at result_ptr+4
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));

    // Advance heap_ptr by 8
    f.instruction(&Instruction::GlobalGet(0));
    f.instruction(&Instruction::I32Const(8));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::GlobalSet(0));

    // Return result_ptr
    f.instruction(&Instruction::LocalGet(i_local));
}

/// Emit instructions to convert i32 discriminant to a GC enum variant ref.
///
/// Uses `br_table` for efficient dispatch:
/// ```text
/// block $done (result (ref null $base))
///   block $v2
///     block $v1
///       block $v0
///         local.get $disc
///         br_table $v0 $v1 $v2
///       end
///       struct.new_default $Variant0
///       br $done
///     end
///     struct.new_default $Variant1
///     br $done
///   end
///   struct.new_default $VariantN
/// end
/// ```
fn emit_i32_to_enum_ref(
    f: &mut Function,
    disc_local: u32,
    variant_type_indices: &[u32],
    base_type_idx: u32,
) {
    let n = variant_type_indices.len();
    if n == 0 {
        f.instruction(&Instruction::Unreachable);
        return;
    }
    if n == 1 {
        // Only one variant — always construct it
        f.instruction(&Instruction::StructNewDefault(variant_type_indices[0]));
        return;
    }

    // Outer block producing the enum base ref
    f.instruction(&Instruction::Block(BlockType::Result(super::ref_nullable(
        base_type_idx,
    ))));

    // Inner blocks for each variant (reversed nesting)
    for _ in 0..n {
        f.instruction(&Instruction::Block(BlockType::Empty));
    }

    // br_table dispatch
    f.instruction(&Instruction::LocalGet(disc_local));
    let targets: Vec<u32> = (0..n as u32).collect();
    f.instruction(&Instruction::BrTable(
        std::borrow::Cow::Borrowed(&targets[..n - 1]),
        (n - 1) as u32,
    ));

    // Each variant block: end, construct, br to $done
    for (i, &vty) in variant_type_indices.iter().enumerate() {
        f.instruction(&Instruction::End); // end inner block
        f.instruction(&Instruction::StructNewDefault(vty));
        if i < n - 1 {
            f.instruction(&Instruction::Br((n - 1 - i) as u32));
        }
    }

    // End outer $done block
    f.instruction(&Instruction::End);
}

/// Emit instructions to convert a GC enum variant ref to i32 discriminant.
///
/// Uses `ref.test` + `if/else` chain. The enum ref is stored in a local
/// so it can be re-tested for each variant.
fn emit_enum_ref_to_i32_via_ref_test(
    f: &mut Function,
    ref_local: u32,
    variant_type_indices: &[u32],
) {
    let n = variant_type_indices.len();
    if n == 0 {
        f.instruction(&Instruction::I32Const(0));
        return;
    }
    if n == 1 {
        f.instruction(&Instruction::I32Const(0));
        return;
    }

    // Nested if-else: ref.test $V0 ? 0 : ref.test $V1 ? 1 : ... : n-1
    for (i, &vty) in variant_type_indices.iter().enumerate() {
        if i == n - 1 {
            // Last variant: default case
            f.instruction(&Instruction::I32Const(i as i32));
        } else {
            f.instruction(&Instruction::LocalGet(ref_local));
            f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(vty)));
            f.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
            f.instruction(&Instruction::I32Const(i as i32));
            f.instruction(&Instruction::Else);
        }
    }
    // Close all if/else blocks
    for _ in 0..n - 1 {
        f.instruction(&Instruction::End);
    }
}

/// Returns true if the type name is a scalar (no GC ref conversion needed).
fn is_scalar_type_name(name: &str) -> bool {
    matches!(
        name,
        "i32" | "i64" | "f32" | "f64" | "bool" | "char" | "u8" | "u16" | "u32" | "u64"
    )
}

/// Emit instructions to lift a list from linear memory (ptr, len) into a GC vec struct.
///
/// Creates a new GC array of `len` elements, copies each element from linear memory,
/// then wraps in a vec struct (array_ref, len):
/// ```text
/// arr = array.new $arr_T (fill=0, len)
/// i = 0
/// loop: if i >= len break
///   arr[i] = load_T(ptr + i * elem_size)
///   i++
/// vec = struct.new $vec_T (arr, len)
/// ```
fn emit_linear_to_gc_list(
    f: &mut Function,
    ptr_local: u32,
    len_local: u32,
    _vec_local: u32,
    arr_local: u32,
    i_local: u32,
    vec_type_idx: u32,
    arr_type_idx: u32,
    elem_size: u32,
    elem_valtype: ValType,
) {
    // arr = array.new $arr_T (fill=default, len)
    f.instruction(&Instruction::LocalGet(len_local));
    match elem_valtype {
        ValType::I64 => f.instruction(&Instruction::I64Const(0)),
        ValType::F64 => f.instruction(&Instruction::F64Const(0.0)),
        _ => f.instruction(&Instruction::I32Const(0)),
    };
    f.instruction(&Instruction::ArrayNew(arr_type_idx));
    f.instruction(&Instruction::LocalSet(arr_local));

    // i = 0
    f.instruction(&Instruction::I32Const(0));
    f.instruction(&Instruction::LocalSet(i_local));

    // block $break
    f.instruction(&Instruction::Block(BlockType::Empty));
    // loop $loop
    f.instruction(&Instruction::Loop(BlockType::Empty));

    // if i >= len, break
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32GeU);
    f.instruction(&Instruction::BrIf(1));

    // arr[i] = load_T(ptr + i * elem_size)
    f.instruction(&Instruction::LocalGet(arr_local));
    f.instruction(&Instruction::LocalGet(i_local));
    // address = ptr + i * elem_size
    f.instruction(&Instruction::LocalGet(ptr_local));
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Const(elem_size as i32));
    f.instruction(&Instruction::I32Mul);
    f.instruction(&Instruction::I32Add);

    let mem0 = wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    };
    match elem_valtype {
        ValType::I32 => {
            f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
        }
        ValType::I64 => {
            f.instruction(&Instruction::I64Load(wasm_encoder::MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        ValType::F64 => {
            f.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        _ => {
            f.instruction(&Instruction::I32Load(mem0));
        }
    }
    f.instruction(&Instruction::ArraySet(arr_type_idx));

    // i++
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalSet(i_local));

    f.instruction(&Instruction::Br(0)); // br $loop
    f.instruction(&Instruction::End); // end loop
    f.instruction(&Instruction::End); // end block

    // vec = struct.new $vec_T (arr, len)
    f.instruction(&Instruction::LocalGet(arr_local));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::StructNew(vec_type_idx));
}

/// Emit instructions to lower a GC vec struct to linear memory and return
/// an i32 pointer to a (ptr, len) pair.
///
/// The GC vec struct ref is already in `vec_local`. This function:
/// 1. Extracts the backing array and length from the vec struct
/// 2. Copies elements from GC array to linear memory starting at heap_ptr
/// 3. Allocates 8 bytes for the (ptr, len) result pair
/// 4. Returns the pair pointer
fn emit_gc_list_to_linear_return(
    f: &mut Function,
    vec_local: u32,
    arr_local: u32,
    len_local: u32,
    i_local: u32,
    vec_type_idx: u32,
    arr_type_idx: u32,
    elem_size: u32,
    elem_valtype: ValType,
) {
    // Extract arr and len from vec struct
    // arr = struct.get $vec_T 0 (vec)
    f.instruction(&Instruction::LocalGet(vec_local));
    f.instruction(&Instruction::StructGet {
        struct_type_index: vec_type_idx,
        field_index: 0,
    });
    f.instruction(&Instruction::LocalSet(arr_local));

    // len = struct.get $vec_T 1 (vec)
    f.instruction(&Instruction::LocalGet(vec_local));
    f.instruction(&Instruction::StructGet {
        struct_type_index: vec_type_idx,
        field_index: 1,
    });
    f.instruction(&Instruction::LocalSet(len_local));

    // data_start_ptr = heap_ptr
    // (We'll remember this implicitly: after the copy loop, heap_ptr hasn't changed)

    // i = 0
    f.instruction(&Instruction::I32Const(0));
    f.instruction(&Instruction::LocalSet(i_local));

    f.instruction(&Instruction::Block(BlockType::Empty));
    f.instruction(&Instruction::Loop(BlockType::Empty));

    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32GeU);
    f.instruction(&Instruction::BrIf(1));

    // memory[heap_ptr + i * elem_size] = arr[i]
    f.instruction(&Instruction::GlobalGet(0)); // heap_ptr
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Const(elem_size as i32));
    f.instruction(&Instruction::I32Mul);
    f.instruction(&Instruction::I32Add);

    f.instruction(&Instruction::LocalGet(arr_local));
    f.instruction(&Instruction::LocalGet(i_local));

    match elem_valtype {
        ValType::I32 => {
            f.instruction(&Instruction::ArrayGet(arr_type_idx));
            f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
        }
        ValType::I64 => {
            f.instruction(&Instruction::ArrayGet(arr_type_idx));
            f.instruction(&Instruction::I64Store(wasm_encoder::MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        ValType::F64 => {
            f.instruction(&Instruction::ArrayGet(arr_type_idx));
            f.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
                offset: 0,
                align: 3,
                memory_index: 0,
            }));
        }
        _ => {
            f.instruction(&Instruction::ArrayGet(arr_type_idx));
            f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
        }
    }

    // i++
    f.instruction(&Instruction::LocalGet(i_local));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalSet(i_local));

    f.instruction(&Instruction::Br(0));
    f.instruction(&Instruction::End); // end loop
    f.instruction(&Instruction::End); // end block

    // data_start_ptr = heap_ptr (unchanged during copy)
    // total_data_bytes = len * elem_size
    // Advance heap_ptr past data
    f.instruction(&Instruction::GlobalGet(0)); // save data_start_ptr
    f.instruction(&Instruction::LocalSet(i_local)); // i_local = data_start_ptr

    f.instruction(&Instruction::GlobalGet(0));
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32Const(elem_size as i32));
    f.instruction(&Instruction::I32Mul);
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::GlobalSet(0)); // heap_ptr += len * elem_size

    // Allocate 8 bytes for (ptr, len) result pair at new heap_ptr
    f.instruction(&Instruction::GlobalGet(0)); // result_ptr
    f.instruction(&Instruction::LocalGet(i_local)); // data_start_ptr
    f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));

    f.instruction(&Instruction::GlobalGet(0)); // result_ptr
    f.instruction(&Instruction::LocalGet(len_local));
    f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));

    // Save result_ptr before advancing heap_ptr
    f.instruction(&Instruction::GlobalGet(0));
    f.instruction(&Instruction::LocalSet(i_local)); // i_local = result_ptr

    // Advance heap_ptr by 8
    f.instruction(&Instruction::GlobalGet(0));
    f.instruction(&Instruction::I32Const(8));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::GlobalSet(0));

    // Return result_ptr
    f.instruction(&Instruction::LocalGet(i_local));
}
