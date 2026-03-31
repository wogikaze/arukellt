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
            // For multi-field record returns, canonical ABI for exports requires
            // returning an i32 pointer (not an extra retptr param).
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
        // For UnitEnum return: one anyref local (to store call result for ref.test)
        // For ScalarRecord return with retptr: one struct ref local
        let mut locals: Vec<(u32, ValType)> = Vec::new();
        let flat_param_count = self.count_flat_params(adapter);

        match &adapter.return_adaptation {
            Some(ReturnAdaptation::UnitEnum { base_type_idx, .. }) => {
                locals.push((1, super::ref_nullable(*base_type_idx)));
            }
            Some(ReturnAdaptation::ScalarRecord { type_idx, .. }) => {
                // Need a local to store the GC struct ref
                locals.push((1, super::ref_nullable(*type_idx)));
            }
            _ => {}
        }

        let mut f = Function::new(locals);
        let mut flat_param_idx: u32 = 0;

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
            }
        }
        count
    }
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
