---
Status: open
Created: 2026-07-10
Updated: 2026-07-10
ID: 729
Track: compiler-internal
Depends on: "724"
Related: "718, 709, 727, ADR-040, ADR-036, ADR-042"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: ADR-042 — intrinsic layer separation design
---

# 729 — Intrinsic layer separation: semantic stdlib + runtime ABI + Ark migration

## Summary

The compiler has ~164 general intrinsics and ~256 SIMD intrinsics, with
~20,000 lines of intrinsic/dispatch code in the Wasm emitter.  The current
structure conflates three independent axes — semantic knowledge,
implementation ownership, and code generation — into a single binary:
"compiler intrinsic or not."

ADR-042 defines a 5-layer architecture that separates these axes:

1. **Language primitives** (MIR/backend) — GC ops, raw array access, trap
2. **Runtime ABI** (runtime/host) — allocator, panic, WASI/WIT imports
3. **Semantic stdlib** (Ark) — `Vec.len`, bounds check (meaning known,
   implementation in Ark, optimized via SemanticId)
4. **Normal stdlib** (Ark) — `split`, `trim`, `sort`, `map`, `parse`
5. **Target intrinsics** (target module) — Wasm SIMD, atomics

This issue tracks the phased migration from the current monolithic
intrinsic structure to the 5-layer architecture.

## Problem

### Current state

- **135 intrinsic files** (15,811 lines) in `src/compiler/wasm/intrinsic_*.ark`
- **28 dispatch files** (3,302 lines) in `src/compiler/wasm/call_*.ark`
- **418 callee string comparisons** (`eq(clone(callee), "starts_with")`)
  despite `func_id_raw` already existing in MirInst
- **GC/LM dual implementations** for most string/vec intrinsics
  (e.g. `intrinsic_string_split.ark` + `intrinsic_string_split_gc.ark`)
- **prelude.ark is a stub** — bodies are never compiled
  (`combine_loaded_and_main_decls_skip_prelude` excludes them)
- **SignatureEntry lacks effect/semantic info** — only has types + ABI kind
- **LICM purity check** is opcode enumeration, not effect-based
- **ADR-040 spine** stops at type info; intrinsic dispatch still uses strings

### Root cause

The boundary is drawn as "compiler knows it or not" (binary).  It should
be three independent axes:

1. Does the compiler know the **meaning**? (semantic knowledge)
2. **Who owns the implementation**? (Ark vs emitter vs runtime)
3. **Who generates the final code**? (MIR op vs emitter expansion vs call)

## Goals

1. **Freeze new intrinsic additions** (except target-specific SIMD)
2. **Migrate callee string dispatch to `func_id_raw` + `SemanticId`**
3. **Move host intrinsics** (HTTP, fs, sockets, clock, random, process,
   stdio) to runtime ABI / WIT import lowering
4. **Extend SignatureEntry** with semantic ID, effect, may-trap,
   const-evaluable, inline policy, lowering policy, fallback body
5. **Create `core-ops.toml`** as the single source of truth for primitive
   operations, generating resolver/typechecker/MIR/docs/runtime tables
6. **Build a limited stdlib-only inliner** (small functions, no recursion,
   cost-threshold-gated)
7. **Migrate pure operations** to Ark stdlib (starts_with, contains, trim,
   sort, map, filter, fold, reverse, etc.)
8. **Restore prelude compilation** — remove
   `combine_loaded_and_main_decls_skip_prelude`, make prelude bodies real
9. **Add sealed raw API** (`core::raw` / `core::intrinsics`) for Vec/String
   internal representation access, accessible only to stdlib
10. **Add differential tests** — Ark fallback vs optimized lowering must
    produce identical results across opt-levels, GC/LM, and targets

## Non-goals

- Full general-purpose inliner (limited stdlib-only inliner is sufficient
  for initial migration)
- `dyn Trait` / vtable support (ADR-036 scope)
- T4 native / LLVM backend changes (target intrinsics are isolated)
- Removing all SIMD intrinsics (target-specific SIMD stays as intrinsics)

## Migration phases

### Phase 1: Intrinsic freeze + FunctionId dispatch

- [ ] No new callee-string intrinsics (enforce via check script)
- [ ] All intrinsic dispatch uses `func_id_raw` + `SemanticId`,
      not `eq(clone(callee), ...)`
- [ ] `call_dispatch.ark` and all `call_*.ark` refactored to registry lookup
- [ ] `normalize_callee_name` removed (no more `__intrinsic_` prefix stripping)

### Phase 2: Host intrinsic → runtime ABI separation

- [ ] HTTP intrinsics removed from emitter → WIT import lowering
- [ ] fs intrinsics removed from emitter → WIT import lowering
- [ ] sockets intrinsics removed from emitter → WIT import lowering
- [ ] clock/random intrinsics removed from emitter → WIT import lowering
- [ ] process exit/abort → runtime PanicHandler FunctionId
- [ ] stdio print/println/eprintln → runtime ABI
- [ ] env var intrinsics → WIT import lowering
- [ ] Compiler no longer knows individual `fs_read_file`, `http_get` etc.
      — only generic WIT/import lowering

### Phase 3: Semantic registry

- [ ] `SignatureEntry` extended with: semantic_id, effect, may_trap,
      const_evaluable, inline_policy, lowering_policy, fallback_body_fid
- [ ] `core-ops.toml` created as single source of truth
- [ ] Generator script produces: resolver builtin table, typechecker
      builtin table, MIR semantic table, docs reference, runtime ABI table
- [ ] `std/manifest.toml` inconsistencies resolved (becomes generated output)
- [ ] Effect categories: pure, readnone, read-storage, write-storage,
      allocate, may-trap, IO, noreturn

### Phase 4: Limited stdlib inliner

- [ ] Inliner targets only compiler-shipped core/std functions
- [ ] No recursion
- [ ] MIR instruction count threshold (e.g. ≤ 20 instructions)
- [ ] Single basic block or simple CFG
- [ ] `@inline(always)` annotation support
- [ ] Cost-threshold-based inlining for small functions
- [ ] Target-specific code size budget enforcement

### Phase 5: Pure operations → Ark stdlib

- [ ] `gcd` → Ark implementation
- [ ] range operations (new, contains, len) → Ark implementation
- [ ] `starts_with`, `ends_with` → Ark (using char_at + len)
- [ ] `contains`, `index_of` → Ark (using char_at + len)
- [ ] `to_upper`, `to_lower` → Ark (using char_at + string_new)
- [ ] `trim_start`, `trim_end` → already partially Ark; complete migration
- [ ] `reverse` → Ark (using get + set)
- [ ] `any`, `find`, `fold` → Ark (using get + loop)
- [ ] `sort` → Ark (using get + set + loop)
- [ ] Monomorphic `map_i32_i32` etc. → generic/trait implementation
- [ ] Each migration includes differential test (Ark vs old intrinsic)

### Phase 6: Allocation-dependent operations → Ark stdlib

- [ ] `split`, `join` → Ark (using string_slice + push)
- [ ] `replace` → Ark (using index_of + concat + string_slice)
- [ ] `repeat` → already partially Ark; complete migration
- [ ] `pad_left`, `pad_right` → Ark (using len + string_new + concat)
- [ ] `lines` → Ark (using split)
- [ ] HashMap/HashSet → Ark implementation
- [ ] `parse_i32`, `parse_i64`, `parse_f64` → Ark or runtime function
      (not emitter intrinsic)
- [ ] `i32_to_string`, `i64_to_string`, `f64_to_string` → Ark or runtime
      function (not emitter intrinsic)

### Phase 7: Prelude restoration

- [ ] `combine_loaded_and_main_decls_skip_prelude` removed
- [ ] Prelude bodies are real compilable Ark code
- [ ] Fake stub bodies in `std/prelude.ark` removed
- [ ] All prelude functions have real implementations in std/ modules
- [ ] `python3 scripts/manager.py verify quick` passes with prelude compiled

### Phase 8: Sealed raw API

- [ ] `core::raw` or `core::intrinsics` module created
- [ ] `raw_array_new<T>`, `raw_array_len<T>`, `raw_array_get_unchecked<T>`,
      `raw_array_set_unchecked<T>`, `raw_array_grow<T>` defined
- [ ] Vec/String implementations use only sealed raw API
- [ ] Sealed module not accessible from user code (visibility enforcement)
- [ ] GC/LM representation differences isolated to raw API layer

### Phase 9: Cleanup

- [ ] `intrinsic_*_gc.ark` / `intrinsic_*_lm.ark` dual files removed
      (representation differences absorbed by raw API)
- [ ] `call_*.ark` dispatch files simplified (most branches removed)
- [ ] Intrinsic file count reduced from 135 to target (~20-40)
- [ ] `call_dispatch.ark` 7-layer if-else chain simplified
- [ ] `std/manifest.toml` regenerated from `core-ops.toml`
- [ ] docs/current-state.md updated
- [ ] docs/stdlib/reference.md regenerated

## Acceptance

- [ ] No callee-string dispatch in `call_*.ark` (all via `func_id_raw`)
- [ ] No host intrinsics in `src/compiler/wasm/intrinsic_*.ark`
      (HTTP, fs, sockets, clock, random, process, stdio, env)
- [ ] `SignatureEntry` has semantic_id, effect, inline_policy, lowering_policy
- [ ] `core-ops.toml` is the single source of truth; manifest.toml is generated
- [ ] stdlib-only inliner operational (small functions inlined at O1+)
- [ ] ≥ 60 general intrinsics migrated to Ark stdlib
- [ ] Prelude is compiled (not skipped)
- [ ] Sealed raw API exists and is used by Vec/String implementations
- [ ] GC/LM dual intrinsic files eliminated
- [ ] Differential tests pass for all migrated operations
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Close gate

A check script under `scripts/check/` that:

1. Asserts no `eq(clone(callee), ...)` in `call_*.ark` (FunctionId-based dispatch only)
2. Asserts no host intrinsic files in `src/compiler/wasm/intrinsic_*.ark`
3. Asserts `core-ops.toml` exists and `std/manifest.toml` is generated from it
4. Asserts prelude is included in compilation (not skipped)
5. Counts intrinsic files and asserts ≤ 50 (excluding SIMD)
6. Runs differential tests for migrated operations

## Dependency Notes

- Depends on **#724** (ADR-040 Phase 5-7 remaining work) — the semantic
  spine must be complete before extending it with effect/inline/lowering
  policy
- Related: **#718** (free-function → method migration) — the Ark stdlib
  migration target is the same; ADR-042 adds the semantic/inliner layer
- Related: **#709** (trait-first API policy) — trait-based generics replace
  monomorphic intrinsics
- Related: **#727** (host bridge retirement) — host intrinsic removal is
  shared scope with Phase 2
- Related: **ADR-040** (semantic type spine) — this issue extends ADR-040
  from type info to call semantics, effect, and intrinsic lowering
- Related: **ADR-036** (trait-based stdlib redesign) — generic/trait
  implementations replace monomorphic intrinsic groups
- Related: **ADR-042** (intrinsic layer separation) — the design ADR for
  this issue

## References

- `docs/adr/ADR-042-intrinsic-layer-separation.md` — design ADR
- `docs/adr/ADR-040-typed-mir-signature-registry.md` — semantic type spine
- `docs/adr/ADR-036-trait-stdlib-redesign.md` — trait-based stdlib
- `src/compiler/wasm/call_dispatch.ark` — current string-based dispatch
- `src/compiler/wasm/intrinsic_*.ark` — 135 intrinsic files (15,811 lines)
- `src/compiler/corehir/signature_entry.ark` — SignatureEntry (to extend)
- `src/compiler/mir/inst_record.ark` — `func_id_raw` field
- `std/prelude.ark` L519-524 — "function bodies are never compiled"
- `src/compiler/driver/pipeline_backend.ark` L35 —
  `combine_loaded_and_main_decls_skip_prelude`
- `std/manifest.toml` — current manifest (to become generated output)
- Swift `@_semantics` and HighLevelSILOptimizations
- GHC `primops.txt.pp` + `genprimopcode`
- Rust `core::intrinsics` and lang items
- LLVM intrinsic design (meaning + type + memory effects)
