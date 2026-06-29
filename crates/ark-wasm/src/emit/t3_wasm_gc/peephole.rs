//! Peephole optimization for the T3 Wasm GC emitter.
//!
//! Provides `PeepholeWriter`, a thin wrapper around `wasm_encoder::Function`
//! that intercepts instruction emission to apply two peephole optimizations:
//!
//! ## Pass 1 — local.set / local.get → local.tee
//!
//!   `local.set X` immediately followed by `local.get X`  →  `local.tee X`
//!
//! This avoids the redundant store-then-load by keeping the value on the stack
//! while also writing it to the local.
//!
//! **GC ref exemption**: This optimization is intentionally skipped for locals
//! that hold GC references (structs, strings, enums, vecs, any). Wasmtime's
//! deferred reference-counting (DRC) GC does not re-register the stack copy
//! produced by `local.tee` in its `VMGcRefActivationsTable`, which causes a
//! panic when subsequent instructions (e.g. `struct.get`) try to read through
//! that untracked GC ref.  For GC-ref locals we emit the canonical
//! `local.set` / `local.get` pair so wasmtime always sees a tracked ref.
//!
//! ## Pass 2 — struct.get / struct.set RMW no-op elimination (opt_level >= 1)
//!
//! Detects the four-instruction read-modify-write no-op pattern:
//!
//! ```wasm
//! local.get $x       ; push struct ref for struct.set
//! local.get $x       ; push struct ref for struct.get (same local)
//! struct.get $T $F   ; read field F of struct type T
//! struct.set $T $F   ; write same value back to field F of same struct
//! ```
//!
//! When both `local.get` instructions load the **same** local index `$x`, and
//! both struct operations address the **same** type `$T` and field `$F`, the
//! four instructions are a semantic no-op (field value unchanged).  The entire
//! sequence is eliminated, saving four instructions per occurrence.
//!
//! This pattern arises from MIR statements of the form `obj.field = obj.field`
//! (identity field assignment), which can appear as compiler-generated code or
//! source-level identity assignments.
//!
//! **Conditions for firing:**
//! - `opt_level >= 1`
//! - Both `local.get` loads use the same local index
//! - Both `struct.get` and `struct.set` address the same `struct_type_index`
//!   and `field_index`
//! - The four instructions are consecutive (no intervening instructions)
//!
//! **Safety:** No `local.tee` is used, so there are no wasmtime DRC tracking
//! concerns for GC refs.  The stack effect of the eliminated sequence is
//! net zero (`[] → []`).

use std::collections::HashSet;

use wasm_encoder::{Function, Instruction};

// ── RMW state machine ────────────────────────────────────────────────────────

/// Internal state for the RMW (read-modify-write) no-op peephole.
///
/// Tracks a sliding window of up to three buffered instructions that may form
/// the prefix of the four-instruction no-op pattern:
///   `local.get $x ; local.get $x ; struct.get $T $F ; struct.set $T $F`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RmwState {
    /// No RMW candidate buffered.
    Idle,
    /// Buffered one `local.get $x` (step 1/4).  Not yet emitted.
    Get1(u32),
    /// Buffered `local.get $x ; local.get $x` (steps 1–2/4).  Not yet emitted.
    Get2(u32),
    /// Buffered `local.get $x ; local.get $x ; struct.get $T $F` (steps 1–3/4).
    /// Waiting to see `struct.set $T $F` to confirm the no-op.
    StructGet {
        local_idx: u32,
        struct_type: u32,
        field_idx: u32,
    },
}

// ── PeepholeWriter ───────────────────────────────────────────────────────────

/// A wrapper around `wasm_encoder::Function` that applies peephole
/// optimizations on the fly during instruction emission.
///
/// When `enabled` (opt_level >= 1):
/// - Buffers a pending `local.set X` and converts it to `local.tee X` when the
///   very next instruction is `local.get X` for the same index — **except** for
///   GC-ref locals (see module doc for the wasmtime DRC exemption).
/// - Detects the RMW no-op pattern and eliminates the entire four-instruction
///   sequence (see module doc for details).
pub(super) struct PeepholeWriter<'a> {
    func: &'a mut Function,
    /// Pending `local.set X` for the local.tee pass.
    pending_set: Option<u32>,
    enabled: bool,
    tee_count: usize,
    /// Local indices that hold GC references; the tee optimization is
    /// suppressed for these to avoid wasmtime DRC activation-table issues.
    gc_ref_locals: HashSet<u32>,
    /// State for the RMW no-op pass (Pass 2).
    rmw_state: RmwState,
    /// Number of four-instruction RMW no-op sequences eliminated.
    rmw_count: usize,
}

impl<'a> PeepholeWriter<'a> {
    pub fn new(func: &'a mut Function, opt_level: u8) -> Self {
        Self {
            func,
            pending_set: None,
            enabled: opt_level >= 1,
            tee_count: 0,
            gc_ref_locals: HashSet::new(),
            rmw_state: RmwState::Idle,
            rmw_count: 0,
        }
    }

    /// Create a PeepholeWriter with a known set of GC-ref local indices.
    /// The `local.tee` optimization is suppressed for any local in this set.
    pub fn with_gc_ref_locals(
        func: &'a mut Function,
        opt_level: u8,
        gc_ref_locals: HashSet<u32>,
    ) -> Self {
        Self {
            func,
            pending_set: None,
            enabled: opt_level >= 1,
            tee_count: 0,
            gc_ref_locals,
            rmw_state: RmwState::Idle,
            rmw_count: 0,
        }
    }

    /// Emit an instruction, applying peephole optimizations when enabled.
    pub fn instruction(&mut self, inst: &Instruction<'_>) -> &mut Self {
        if !self.enabled {
            self.func.instruction(inst);
            return self;
        }

        match inst {
            // ── Pass 1: local.set/local.get → local.tee ──────────────────
            Instruction::LocalSet(x) => {
                // Any incoming LocalSet disrupts the RMW candidate window.
                self.flush_rmw();
                // Flush any previous pending set before buffering the new one.
                self.flush_pending();
                // Don't buffer GC-ref locals — emit immediately to avoid the
                // tee optimization that confuses wasmtime DRC tracking.
                if self.gc_ref_locals.contains(x) {
                    self.func.instruction(inst);
                } else {
                    self.pending_set = Some(*x);
                }
            }

            // ── Interaction: local.get matching a pending local.set ───────
            Instruction::LocalGet(x) if self.pending_set == Some(*x) => {
                // local.set X ; local.get X  →  local.tee X
                // This fires before the RMW state machine; reset RMW state.
                self.rmw_state = RmwState::Idle;
                self.pending_set = None;
                self.func.instruction(&Instruction::LocalTee(*x));
                self.tee_count += 1;
            }

            // ── Pass 2: RMW no-op — local.get advance ────────────────────
            Instruction::LocalGet(x) => {
                // No matching pending_set; enter / advance the RMW window.
                self.flush_pending();
                self.advance_rmw_get(*x);
            }

            // ── Pass 2: RMW no-op — struct.get ───────────────────────────
            Instruction::StructGet {
                struct_type_index,
                field_index,
            } => {
                if let RmwState::Get2(local_idx) = self.rmw_state {
                    // Steps 1–3 of the RMW pattern are now buffered.
                    self.rmw_state = RmwState::StructGet {
                        local_idx,
                        struct_type: *struct_type_index,
                        field_idx: *field_index,
                    };
                    // Don't emit yet — wait to see struct.set.
                } else {
                    // Pattern broken; flush buffer and emit normally.
                    self.flush_pending();
                    self.flush_rmw();
                    self.func.instruction(inst);
                }
            }

            // ── Pass 2: RMW no-op — struct.set (confirmation / elimination)
            Instruction::StructSet {
                struct_type_index,
                field_index,
            } => {
                if let RmwState::StructGet {
                    struct_type,
                    field_idx,
                    ..
                } = self.rmw_state
                    && struct_type == *struct_type_index
                    && field_idx == *field_index
                {
                    // All four instructions form the RMW no-op.  Eliminate the
                    // entire sequence — nothing is emitted to `func`.
                    self.rmw_state = RmwState::Idle;
                    self.rmw_count += 1;
                } else {
                    // Pattern broken; flush buffer and emit normally.
                    self.flush_pending();
                    self.flush_rmw();
                    self.func.instruction(inst);
                }
            }

            // ── Catch-all: flush both buffers before emitting ─────────────
            _ => {
                self.flush_pending();
                self.flush_rmw();
                self.func.instruction(inst);
            }
        }
        self
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Flush any buffered `local.set` that hasn't been paired with a `local.get`.
    fn flush_pending(&mut self) {
        if let Some(x) = self.pending_set.take() {
            self.func.instruction(&Instruction::LocalSet(x));
        }
    }

    /// Advance the RMW state machine for an incoming `local.get $x`.
    ///
    /// `flush_pending()` must have been called before this.
    fn advance_rmw_get(&mut self, x: u32) {
        match self.rmw_state {
            RmwState::Idle => {
                // Buffer the local.get — it might be step 1 of the pattern.
                self.rmw_state = RmwState::Get1(x);
            }
            RmwState::Get1(prev) if prev == x => {
                // Two consecutive local.get for the same local — step 2 of
                // the RMW pattern.
                self.rmw_state = RmwState::Get2(x);
            }
            RmwState::Get1(prev) => {
                // Different local — flush the buffered Get1 and start over.
                self.func.instruction(&Instruction::LocalGet(prev));
                self.rmw_state = RmwState::Get1(x);
            }
            _ => {
                // Any other state: flush the RMW buffer and start a new Get1.
                self.flush_rmw();
                self.rmw_state = RmwState::Get1(x);
            }
        }
    }

    /// Emit all buffered RMW candidate instructions to `func` and reset to Idle.
    fn flush_rmw(&mut self) {
        match self.rmw_state {
            RmwState::Idle => {}
            RmwState::Get1(x) => {
                self.func.instruction(&Instruction::LocalGet(x));
            }
            RmwState::Get2(x) => {
                self.func.instruction(&Instruction::LocalGet(x));
                self.func.instruction(&Instruction::LocalGet(x));
            }
            RmwState::StructGet {
                local_idx,
                struct_type,
                field_idx,
            } => {
                self.func.instruction(&Instruction::LocalGet(local_idx));
                self.func.instruction(&Instruction::LocalGet(local_idx));
                self.func.instruction(&Instruction::StructGet {
                    struct_type_index: struct_type,
                    field_index: field_idx,
                });
            }
        }
        self.rmw_state = RmwState::Idle;
    }

    // ── Public accessors ─────────────────────────────────────────────────────

    /// Flush pending state. Must be called before the underlying `Function` is
    /// finalized (i.e., before emitting `End` or passing to `CodeSection`).
    pub fn flush(&mut self) {
        self.flush_rmw();
        self.flush_pending();
    }

    /// Byte length of the encoded function body so far, after flushing peephole
    /// buffers. Used for `metadata.code.branch_hint` offsets (byte offset to the
    /// hinted `if` instruction).
    pub fn encoded_body_byte_len(&mut self) -> usize {
        self.flush();
        self.func.byte_len()
    }

    /// Number of `local.set`/`local.get` pairs replaced with `local.tee`.
    pub fn tee_count(&self) -> usize {
        self.tee_count
    }

    /// Number of four-instruction RMW no-op sequences eliminated.
    ///
    /// Each eliminated sequence corresponds to a `struct.get $T $F` immediately
    /// followed by `struct.set $T $F` on the same struct (same local) and same
    /// field — a pattern that reads and writes back the same value unchanged.
    pub fn rmw_count(&self) -> usize {
        self.rmw_count
    }
}
