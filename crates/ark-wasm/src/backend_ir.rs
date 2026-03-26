//! Target-neutral backend IR descriptors.
//!
//! These types describe language-level constructs in a target-neutral way.
//! Per-target emitters consult these descriptors to decide concrete layouts.

/// How a type is represented in the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendType {
    /// Primitive scalar: i32, i64, f32, f64, bool, char.
    Scalar(ScalarKind),
    /// Heap-allocated string (length-prefixed in T1, GC struct in T3).
    String,
    /// Growable vector: `Vec<T>`.
    Vec(Box<BackendType>),
    /// User-defined struct with named fields.
    Struct(String),
    /// User-defined enum with tagged variants.
    Enum(String),
    /// Optional value: `Option<T>`.
    Option(Box<BackendType>),
    /// Result value: `Result<T, E>`.
    Result(Box<BackendType>, Box<BackendType>),
    /// Closure (code + captured environment).
    Closure,
    /// Unit / void.
    Unit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarKind {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Char,
}

/// Describes how a heap object is managed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeapObjectKind {
    /// Arena/bump allocation (T1).
    Arena,
    /// GC-managed struct/array (T3).
    GcManaged,
    /// Stack-allocated (future optimization).
    Stack,
}

/// ABI surface descriptor for exports/imports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbiSurface {
    /// Raw Wasm exports (Layer 2A) — same-language, same-ABI.
    RawWasm,
    /// WIT/Component Model exports (Layer 2B) — cross-language.
    ComponentWit,
}

/// Entry point shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryPointShape {
    /// `fn main()` — no arguments.
    NoArg,
    /// `fn main(caps: Capabilities) -> Result<(), IOError>` — capability-based.
    WithCapabilities,
}

/// Import kind for WASI/host functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportKind {
    /// WASI Preview 1 import.
    WasiP1 { module: String, name: String },
    /// WASI Preview 2 import (canonical ABI).
    WasiP2 { interface: String, name: String },
    /// Raw host function import.
    RawHost { module: String, name: String },
}

/// Export shape for module exports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportShape {
    pub name: String,
    pub params: Vec<BackendType>,
    pub result: Option<BackendType>,
    pub surface: AbiSurface,
}

/// Target-neutral backend IR — metadata that per-target emitters consume
/// alongside the MIR to make layout and ABI decisions.
#[derive(Clone, Debug)]
pub struct BackendIr {
    /// How heap objects are managed for this target.
    pub heap_kind: HeapObjectKind,
    /// ABI surface for this target.
    pub abi_surface: AbiSurface,
    /// Entry point shape detected from the source program.
    pub entry_point: EntryPointShape,
    /// Imports required by the program.
    pub imports: Vec<ImportKind>,
    /// Exports from the program.
    pub exports: Vec<ExportShape>,
}
