---
Status: open
Created: 2026-07-10
Updated: 2026-07-15
ID: 729
Track: compiler-internal
Depends on: "724"
Related: "718, 709, 727, 798, 808, 816, 817, 818, ADR-040, ADR-036, ADR-042"
Orchestration class: unblocked
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: ADR-042 — intrinsic layer separation design
---

# 729 — Intrinsic layer separation (unblocked epic)

## Summary

This issue is the **unblocked epic** for the 5-layer intrinsic separation defined
in ADR-042. It does **not** contain detailed implementation checklists; those
live in the child issues and in [`docs/plans/intrinsic-layer-separation.md`](../../docs/plans/intrinsic-layer-separation.md).

ADR-042 is now accepted. The first implementation child is **#798**, which
establishes the core-ops registry and `SignatureEntry` schema.

## Scope

- Intrinsic layer separation across the compiler and stdlib.
- Migration from callee-string dispatch to `FunctionId` + `SignatureRegistry` lookup.
- Moving host/runtime operations to runtime ABI / WIT import lowering.
- Moving pure and allocation-dependent operations to Ark stdlib.
- Introducing a limited stdlib-only inliner.

## Separately scoped children

- **Prelude compilation restoration** — separate RFC and child **#816**.
- **Sealed raw API module name and surface** — separate RFC and child **#817**.
- **Production scaffold exit** — production lowerings and differential proof in **#818**.

## Architecture

ADR-042 defines five layers:

1. Language primitives (MIR/backend)
2. Runtime ABI (runtime/host)
3. Semantic stdlib (Ark, with compiler-known meaning)
4. Normal stdlib (Ark)
5. Target intrinsics (`std::wasm`, raw SIMD; portable `std::simd` is `normal_call` + specializations)

## Child issues

- **#798** — ADR-042 semantic operation registry migration (first implementation child)
- **#816** — Prelude compilation restoration (RFC-gated)
- **#817** — Sealed raw API for Vec/String representation (RFC-gated)
- **#818** — CoreOpRegistry production scaffold exit

Additional implementation children (host ABI separation, stdlib inliner,
pure stdlib migration, allocation-dependent stdlib migration) will be filed
under this epic as the registry work in #798 completes.

## Related issues

- **#816** — Prelude compilation restoration (outside #798 scope)
- **#817** — Sealed raw API module for Vec/String internal representation (outside #798 scope)
- **#818** — Production lowerings, fallback bodies, differential proof, and `status = "production"`
- **#808** — T3/Wasm validation failures (global `verify quick` gate blocker)

## Acceptance

- [ ] ADR-042 is accepted
- [ ] All child issues of this epic are closed
- [ ] Callee-string dispatch is removed from call lowering (semantic invariant, not just a file-name check)
- [ ] No host intrinsics remain in the emitter; host operations are lowered through runtime ABI / WIT import
- [ ] `SignatureEntry` carries only `core_op_id` and function signature; `CoreOpRegistry` metadata is not duplicated
- [ ] `data/core-ops.toml` is the SSOT for semantic types / `CoreOpId` / visibility / classification / binding / effect / lowering / fallback
- [ ] `std/manifest.toml` remains the SSOT for public path / docs / stability / deprecation, referencing `core-ops.toml` via `core_op_id` / `type_id`
- [ ] stdlib-only inliner is operational
- [ ] Migrated operations have Ark fallback bodies and pass differential tests
- [ ] `data/core-ops.toml` has `status = "production"` and no migration-only lowerings or aliases
- [ ] Global `python3 scripts/manager.py verify quick` exits 0 (blocked by #808 until fixed)

## Close gate

A check script under `scripts/check/` that verifies semantic invariants, not
specific file names or syntax:

1. Call lowering does not use callee strings for semantic dispatch (except for diagnostics and error messages).
2. Every CALL `FunctionId` either resolves to a `SignatureRegistry` entry or is treated as an ordinary function call.
3. Every non-`normal_call` lowering has a canonical `CoreOpId`.
4. Host operations are lowered through the runtime ABI layer, not through `intrinsic_*.ark` files in the emitter.
5. Public `std/manifest.toml` bindings are consistent with `data/core-ops.toml` `signature` and `effect`.
6. Differential tests pass between Ark fallback and optimized lowering for all migrated operations.
7. The #818 production scaffold-exit gates pass and `status = "production"`.
8. `python3 scripts/manager.py verify quick` passes (requires #808 closed).

## Notes

- The detailed phase order is canonicalized in [`docs/plans/intrinsic-layer-separation.md`](../../docs/plans/intrinsic-layer-separation.md).
- This issue is intentionally **not** a checklist of phases; phase-level
checklists belong in child issues and the plan document.
- #816 and #817 are children of this epic but remain outside #798's bounded
  dispatch-spine migration. #818 consumes their required outputs for final production exit.

## Dependency Notes

- Depends on **#724** (ADR-040 Phase 5-7 remaining work) — the semantic
  spine must be complete before extending it with effect/inline/lowering
  policy.
- Related: **#727** (host bridge retirement) — runtime ABI / host bridge
  migration is downstream of the registry work; host intrinsic removal is
  shared scope with the child migration issues.
- Related: **#798** — registry schema and `SignatureEntry` extension.
- Related: **#808** — T3/Wasm validation failures (global `verify quick` blocker).
- Related: **#816** — prelude compilation restoration (RFC dependency, out of scope).
- Related: **#817** — sealed raw API module (RFC dependency, out of scope).
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
- `src/compiler/wasm/inst_dispatch.ark` — FunctionId/CoreOp call dispatch owner
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
