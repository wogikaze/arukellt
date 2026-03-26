//! T3 `wasm32-wasi-p2` backend — Wasm GC emitter.
//!
//! Generates a Wasm module using GC types (struct/array/ref)
//! instead of linear memory for heap objects.
//! WASI Preview 1 imports are used as a compatibility bridge
//! until WASI Preview 2 canonical ABI is available.
//!
//! Currently delegates to T1 emitter. GC type definitions and
//! infrastructure are scaffolded for incremental development.

#![allow(dead_code, unused_imports)]

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use std::collections::{HashMap, HashSet};
use wasm_encoder::{
    ArrayType, CodeSection, CompositeInnerType, CompositeType, DataSection, ExportKind,
    ExportSection, FieldType, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, MemArg, MemorySection, MemoryType, Module, RefType as WasmRefType, StorageType,
    StructType, SubType, TypeSection, ValType,
};

// --- Type indices (assigned during type section construction) ---
// These are determined dynamically as types are registered.

/// WASI p1 function indices (imported).
const FN_FD_WRITE: u32 = 0;

/// GC type index allocation tracker.
struct TypeAlloc {
    next_idx: u32,
    /// Maps language type name → Wasm type index.
    type_map: HashMap<String, u32>,
    /// Wasm type section being built.
    types: TypeSection,
}

impl TypeAlloc {
    fn new() -> Self {
        Self {
            next_idx: 0,
            type_map: HashMap::new(),
            types: TypeSection::new(),
        }
    }

    /// Register a function type and return its index.
    fn add_func_type(&mut self, params: &[ValType], results: &[ValType]) -> u32 {
        let idx = self.next_idx;
        self.types
            .ty()
            .function(params.iter().copied(), results.iter().copied());
        self.next_idx += 1;
        idx
    }

    /// Register a GC struct type and return its index.
    fn add_struct_type(&mut self, name: &str, fields: &[FieldType]) -> u32 {
        let idx = self.next_idx;
        self.type_map.insert(name.to_string(), idx);
        self.types.ty().subtype(&SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Struct(StructType {
                    fields: fields.to_vec().into_boxed_slice(),
                }),
                shared: false,
            },
        });
        self.next_idx += 1;
        idx
    }

    /// Register a GC array type and return its index.
    fn add_array_type(&mut self, name: &str, element: FieldType) -> u32 {
        let idx = self.next_idx;
        self.type_map.insert(name.to_string(), idx);
        self.types.ty().subtype(&SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Array(ArrayType(element)),
                shared: false,
            },
        });
        self.next_idx += 1;
        idx
    }
}

/// Make a mutable field of the given storage type.
fn mutable_field(st: StorageType) -> FieldType {
    FieldType {
        element_type: st,
        mutable: true,
    }
}

/// Make an immutable field of the given storage type.
fn immutable_field(st: StorageType) -> FieldType {
    FieldType {
        element_type: st,
        mutable: false,
    }
}

/// Context for T3 GC emission.
struct T3EmitCtx {
    types: TypeAlloc,
    /// String literal data: (offset_in_data, bytes).
    string_literals: Vec<(u32, Vec<u8>)>,
    data_offset: u32,
    /// User function names → function index.
    fn_indices: HashMap<String, u32>,
    /// Next function index (after imports + builtins).
    next_fn_idx: u32,
    /// WASI fd_write function type index.
    fd_write_type_idx: u32,
    /// i32_to_string helper type index.
    i32_to_str_type_idx: u32,
    /// print_i32_ln helper type index.
    print_i32_ln_type_idx: u32,
    /// print_str_ln helper type index.
    print_str_ln_type_idx: u32,
    /// GC type indices for built-in types.
    /// $bytes_array: (array (mut i8))
    bytes_array_type_idx: u32,
    /// $string: (struct (field $bytes (ref $bytes_array)))
    string_type_idx: u32,
    /// Struct type indices: struct_name → type_idx.
    struct_type_indices: HashMap<String, u32>,
    /// Struct field info: struct_name → [(field_name, field_type_string)].
    struct_layouts: HashMap<String, Vec<(String, String)>>,
    /// Enum type indices: enum_name → type_idx.
    enum_type_indices: HashMap<String, u32>,
    /// Enum payload types: enum_name → [(variant_name, [type_name])].
    enum_payload_types: HashMap<String, Vec<(String, Vec<String>)>>,
    /// Function return types.
    fn_return_types: HashMap<String, ark_typecheck::types::Type>,
    /// Known string locals.
    string_locals: HashSet<u32>,
    /// Known f64 locals.
    f64_locals: HashSet<u32>,
    /// Known i64 locals.
    i64_locals: HashSet<u32>,
    /// Known bool locals.
    bool_locals: HashSet<u32>,
    /// Loop depth stack for break/continue.
    loop_depths: Vec<u32>,
}

impl T3EmitCtx {
    fn type_to_valtype(ty: &ark_typecheck::types::Type) -> ValType {
        match ty {
            ark_typecheck::types::Type::F64 => ValType::F64,
            ark_typecheck::types::Type::F32 => ValType::F32,
            ark_typecheck::types::Type::I64 => ValType::I64,
            // GC references for heap types would use ValType::Ref,
            // but for now we keep i32 as a simple approach
            _ => ValType::I32,
        }
    }
}

/// Emit a Wasm GC module from MIR.
///
/// This is a minimal T3 emitter that generates Wasm with GC type definitions
/// in the type section, but still uses linear memory for actual data storage.
/// This is a stepping stone toward a full GC backend.
pub fn emit(mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
    // For now, delegate to T1 emitter since T3 requires significant
    // additional work (GC runtime integration, new allocation strategy).
    // The T3 type definitions and structure are ready for when
    // the GC emitter is fully implemented.
    //
    // This function exists to validate the target dispatch works
    // and to serve as the entry point for incremental T3 development.
    super::t1_wasm32_p1::emit(mir, _sink)
}
