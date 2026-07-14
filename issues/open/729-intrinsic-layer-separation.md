---
Status: open
Created: 2026-07-10
Updated: 2026-07-15
ID: 729
Track: compiler-internal
Depends on: "724, 727"
Related: "718, 709, 798, 816, 817, ADR-040, ADR-036, ADR-042"
Orchestration class: blocked
Orchestration upstream: ADR-042 acceptance
Blocks v{N}: none
Priority: 1
Source: ADR-042 — intrinsic layer separation design
---

# 729 — Intrinsic layer separation (blocked epic)

## Summary

This issue is the **blocked epic** for the 5-layer intrinsic separation defined
in ADR-042. It does **not** contain detailed implementation checklists; those
live in the child issues and in [`docs/plans/intrinsic-layer-separation.md`](../../docs/plans/intrinsic-layer-separation.md).

The epic is blocked until ADR-042 is accepted. The first implementation child
is **#798**, which establishes the core-ops registry and `SignatureEntry` schema.

## Scope

- Intrinsic layer separation across the compiler and stdlib.
- Migration from callee-string dispatch to `FunctionId` + `SignatureRegistry` lookup.
- Moving host/runtime operations to runtime ABI / WIT import lowering.
- Moving pure and allocation-dependent operations to Ark stdlib.
- Introducing a limited stdlib-only inliner.

## Out of scope (separate RFC / child issues)

- **Prelude compilation restoration** — requires a separate RFC; tracked as **#816**.
- **Sealed raw API module name and surface** — requires a separate RFC; tracked as **#817**.

## Architecture

ADR-042 defines five layers:

1. Language primitives (MIR/backend)
2. Runtime ABI (runtime/host)
3. Semantic stdlib (Ark, with compiler-known meaning)
4. Normal stdlib (Ark)
5. Target intrinsics (`std::wasm`, `std::simd`, etc.)

## Child issues

- **#798** — ADR-042 semantic operation registry migration (first implementation child)
- **#816** — Prelude compilation restoration (RFC dependency)
- **#817** — Sealed raw API module for Vec/String internal representation (RFC dependency)

Additional implementation children (host ABI separation, stdlib inliner,
pure stdlib migration, allocation-dependent stdlib migration) will be filed
under this epic as the registry work in #798 completes.

## Acceptance

- [ ] ADR-042 is accepted
- [ ] Callee-string dispatch is removed from `call_*.ark` (all dispatch via `FunctionId` + `SignatureRegistry`)
- [ ] No host intrinsics remain in `src/compiler/wasm/intrinsic_*.ark`
- [ ] `SignatureEntry` carries `semantic_id`, effect, `const` semantics, `inline_policy`, `lowering_kind`, target, and fallback
- [ ] `data/core-ops.toml` is the SSOT for semantic types / `SemanticId` / effect / lowering / fallback
- [ ] `std/manifest.toml` remains the SSOT for public path / docs / stability / deprecation, referencing `core-ops.toml` via `semantic_id` / `type_id`
- [ ] stdlib-only inliner is operational
- [ ] ≥ 60 general intrinsics are migrated to Ark stdlib
- [ ] GC/LM dual intrinsic files are eliminated
- [ ] Differential tests pass for all migrated operations
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Close gate

A check script under `scripts/check/` that:

1. Asserts no `eq(clone(callee), ...)` in `call_*.ark` (FunctionId-based dispatch only)
2. Asserts no host intrinsic files in `src/compiler/wasm/intrinsic_*.ark`
3. Asserts `data/core-ops.toml` exists and is the SSOT referenced by `std/manifest.toml`
4. Asserts `SignatureEntry` includes the ADR-042 fields
5. Runs differential tests for migrated operations
6. Runs `python3 scripts/manager.py verify quick`

## Notes

- The detailed phase order is canonicalized in [`docs/plans/intrinsic-layer-separation.md`](../../docs/plans/intrinsic-layer-separation.md).
- This issue is intentionally **not** a checklist of phases; phase-level
checklists belong in child issues and the plan document.

## Dependency Notes

- Depends on **#724** (ADR-040 Phase 5-7 remaining work) — the semantic
  spine must be complete before extending it with effect/inline/lowering
  policy.
- Depends on **#727** (host bridge retirement + wasm-heap-grow-patcher
  retirement) — `verify quick` must pass before intrinsic dispatch refactoring
  begins. Host intrinsic removal is shared scope with the child migration
  issues.
- Related: **#718** (free-function → method migration) — the Ark stdlib
  migration target is the same; ADR-042 adds the semantic/inliner layer.
- Related: **#709** (trait-first API policy) — trait-based generics replace
  monomorphic intrinsics.
- Related: **ADR-040** (semantic type spine) — this issue extends ADR-040
  from type info to call semantics, effect, and intrinsic lowering.
- Related: **ADR-036** (trait-based stdlib redesign) — generic/trait
  implementations replace monomorphic intrinsic groups.
- Related: **ADR-042** (intrinsic layer separation) — the design ADR for
  this issue.

## References

- `docs/adr/ADR-042-intrinsic-layer-separation.md` — design ADR
- `docs/plans/intrinsic-layer-separation.md` — canonical migration order
- `docs/adr/ADR-040-typed-mir-signature-registry.md` — semantic type spine
- `docs/adr/ADR-036-trait-stdlib-redesign.md` — trait-based stdlib
- `data/core-ops.toml` — semantic registry SSOT
- `std/manifest.toml` — public API / docs / stability / deprecation SSOT
- `src/compiler/wasm/call_dispatch.ark` — current string-based dispatch
- `src/compiler/wasm/intrinsic_*.ark` — intrinsic files
- `src/compiler/corehir/signature_entry.ark` — SignatureEntry
- `src/compiler/mir/inst_record.ark` — `func_id_raw` field
- `std/prelude.ark` — prelude stub bodies
- `src/compiler/driver/pipeline_backend.ark` —
  `combine_loaded_and_main_decls_skip_prelude`
- Swift `@_semantics` and HighLevelSILOptimizations
- GHC `primops.txt.pp` + `genprimopcode`
- Rust `core::intrinsics` and lang items
- LLVM intrinsic design (meaning + type + memory effects)
