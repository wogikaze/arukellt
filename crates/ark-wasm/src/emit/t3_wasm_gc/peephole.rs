//! Peephole optimization for the T3 Wasm GC emitter.
//!
//! Provides `PeepholeWriter`, a thin wrapper around `wasm_encoder::Function`
//! that intercepts instruction emission to apply local-variable peephole
//! optimizations:
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

use std::collections::HashSet;

use wasm_encoder::{Function, Instruction};

/// A wrapper around `wasm_encoder::Function` that applies peephole
/// optimizations on the fly during instruction emission.
///
/// When `enabled` (opt_level >= 1), it buffers a pending `local.set X` and,
/// if the very next instruction is `local.get X` for the same index, emits
/// `local.tee X` instead of the set+get pair — **except** for locals whose
/// index appears in `gc_ref_locals`, where the optimization is suppressed to
/// avoid a wasmtime DRC GC tracking issue.
pub(super) struct PeepholeWriter<'a> {
    func: &'a mut Function,
    pending_set: Option<u32>,
    enabled: bool,
    tee_count: usize,
    /// Local indices that hold GC references; the tee optimization is
    /// suppressed for these to avoid wasmtime DRC activation-table issues.
    gc_ref_locals: HashSet<u32>,
}

impl<'a> PeepholeWriter<'a> {
    pub fn new(func: &'a mut Function, opt_level: u8) -> Self {
        Self {
            func,
            pending_set: None,
            enabled: opt_level >= 1,
            tee_count: 0,
            gc_ref_locals: HashSet::new(),
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
        }
    }

    /// Emit an instruction, applying peephole optimizations when enabled.
    pub fn instruction(&mut self, inst: &Instruction<'_>) -> &mut Self {
        if !self.enabled {
            self.func.instruction(inst);
            return self;
        }

        match inst {
            Instruction::LocalSet(x) => {
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
            Instruction::LocalGet(x) if self.pending_set == Some(*x) => {
                // local.set X ; local.get X  →  local.tee X
                self.pending_set = None;
                self.func.instruction(&Instruction::LocalTee(*x));
                self.tee_count += 1;
            }
            _ => {
                // Any non-matching instruction flushes the pending set first.
                self.flush_pending();
                self.func.instruction(inst);
            }
        }
        self
    }

    /// Flush any buffered `local.set` that hasn't been paired with a `local.get`.
    fn flush_pending(&mut self) {
        if let Some(x) = self.pending_set.take() {
            self.func.instruction(&Instruction::LocalSet(x));
        }
    }

    /// Flush pending state. Must be called before the underlying `Function` is
    /// finalized (i.e., before emitting `End` or passing to `CodeSection`).
    pub fn flush(&mut self) {
        self.flush_pending();
    }

    /// Number of `local.set`/`local.get` pairs replaced with `local.tee`.
    pub fn tee_count(&self) -> usize {
        self.tee_count
    }
}
