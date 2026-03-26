//! LLVM IR backend for the Arukellt compiler.
//!
//! This crate implements the T4 `native` target backend by lowering MIR
//! to LLVM IR via the `inkwell` safe wrapper around the LLVM C API.
//!
//! The native backend follows Wasm semantics — it does not introduce
//! LLVM-specific type features or native-only language constructs.
//! Its purpose is to provide fast local execution and debugging.
//!
//! ## Supported features (Phase 1)
//!
//! - i32, i64, f32, f64, bool constants and arithmetic
//! - Local variables and function parameters
//! - Function calls and returns
//! - If/else and while control flow
//! - Basic println via libc printf bridge
//!
//! ## Not yet supported
//!
//! - Heap types (String, Vec, struct, enum) — requires GC or malloc runtime
//! - Closures and indirect calls
//! - WASI-equivalent I/O (uses libc printf as bridge)

mod emit;

pub use emit::emit_llvm_ir;
pub use emit::emit_object;
