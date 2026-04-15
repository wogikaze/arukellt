//! WasmGC `i31ref` scalar optimization helpers for the T3 emitter.
//!
//! WasmGC's `i31ref` type stores 31-bit signed integers as unboxed tagged
//! immediates on the Wasm stack, avoiding GC heap allocation entirely.
//! This is the canonical way to represent small scalar values (booleans,
//! characters, small enum discriminants) in a GC-typed program without
//! paying heap-object overhead.
//!
//! ## Instructions
//!
//! | Wasm text          | `wasm-encoder` variant        | Description                    |
//! |--------------------|-------------------------------|--------------------------------|
//! | `ref.i31`          | `Instruction::RefI31`         | i32 → (ref null i31)           |
//! | `i31.get_s`        | `Instruction::I31GetS`        | (ref null i31) → i32 (signed)  |
//! | `i31.get_u`        | `Instruction::I31GetU`        | (ref null i31) → i32 (unsigned)|
//!
//! ## Usage in T3 emitter
//!
//! ### Boxing (i32 → i31ref)
//!
//! When a small integer (bool, char, small enum tag) must be stored in an
//! `anyref`/`eqref`-typed slot (e.g., a generic function parameter typed as
//! `T` = `Any`), the emitter inserts a `ref.i31` instruction to convert the
//! raw i32 into an unboxed i31ref reference.
//!
//! **Example** — passing `true` to `fn identity<T>(x: T) -> T`:
//! ```wasm
//! i32.const 1    ;; push bool(true) as i32
//! ref.i31        ;; box as unboxed i31ref — no heap allocation
//! call $identity
//! ```
//!
//! ### Unboxing (i31ref → i32)
//!
//! When an `anyref`/`eqref` value must be coerced back to a concrete i32
//! (because the destination local has type `bool`/`char`/`i32`), the emitter
//! inserts `ref.cast (ref null i31)` followed by `i31.get_s`.
//!
//! **Example** — receiving a bool back from `fn identity<T>`:
//! ```wasm
//! ref.cast (ref null i31)   ;; narrow anyref → i31ref
//! i31.get_s                 ;; unbox to i32
//! local.set $a              ;; store bool 0/1
//! ```
//!
//! ### Direct anyref compatibility
//!
//! An i31ref value is a subtype of `anyref`/`eqref`, so it can be stored
//! in those slots directly without an additional cast.
//!
//! ## Optimisation level
//!
//! * **O1 (default)**: i31ref is used for boxing small integers into `anyref`
//!   slots (generic parameters).  Local variable types remain `i32`.
//!
//! * **O2**: (future) bool locals are declared as `(ref null i31)` directly,
//!   eliminating the boxing step at generic call sites entirely.  Condition
//!   checks emit `i31.get_s` to obtain the i32 value needed for `if`/`br_if`.

use wasm_encoder::{AbstractHeapType, HeapType, Instruction, RefType, ValType};

use super::peephole::PeepholeWriter;

// ── Public type helpers ───────────────────────────────────────────────────────

/// The nullable `(ref null i31)` value type.
///
/// Use this when declaring a local or function-return as an i31ref slot.
pub(super) fn i31ref_valtype() -> ValType {
    ValType::Ref(RefType::I31REF)
}

/// Returns `true` iff `vt` is an i31ref type (nullable or non-nullable).
#[allow(dead_code)]
pub(super) fn is_i31ref(vt: &ValType) -> bool {
    matches!(
        vt,
        ValType::Ref(RefType {
            heap_type: HeapType::Abstract {
                ty: AbstractHeapType::I31,
                ..
            },
            ..
        })
    )
}

// ── Emission helpers ──────────────────────────────────────────────────────────

/// Emit `ref.i31`: convert an i32 on the Wasm stack into an i31ref.
///
/// Precondition: an `i32` value is on top of the stack.
/// Postcondition: an `(ref null i31)` value is on top of the stack.
pub(super) fn emit_box(f: &mut PeepholeWriter<'_>) {
    f.instruction(&Instruction::RefI31);
}

/// Emit `i31.get_s`: signed-unbox an i31ref into an i32.
///
/// Precondition: an `(ref null i31)` value is on top of the stack.
/// Postcondition: an `i32` value is on top of the stack (sign-extended from 31 bits).
pub(super) fn emit_unbox_signed(f: &mut PeepholeWriter<'_>) {
    f.instruction(&Instruction::I31GetS);
}

/// Emit `i31.get_u`: unsigned-unbox an i31ref into an i32.
///
/// Precondition: an `(ref null i31)` value is on top of the stack.
/// Postcondition: an `i32` value is on top of the stack (zero-extended from 31 bits).
#[allow(dead_code)]
pub(super) fn emit_unbox_unsigned(f: &mut PeepholeWriter<'_>) {
    f.instruction(&Instruction::I31GetU);
}

/// Emit the instruction sequence to box an i32 bool (0 or 1) as i31ref
/// when it needs to flow into an `anyref`/`eqref`-typed slot.
///
/// This is the hot path for `bool` values passed to generic (`T = Any`) functions.
/// The emitter first produces the i32 via the normal `ConstBool` / `local.get` path,
/// then calls this function before the `call` instruction.
pub(super) fn emit_bool_to_anyref(f: &mut PeepholeWriter<'_>) {
    // i32 (0/1) → i31ref: zero-cost unboxed tagging, no heap allocation.
    emit_box(f);
}
