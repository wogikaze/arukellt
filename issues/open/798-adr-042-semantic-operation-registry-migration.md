---
Status: open
Created: 2026-07-14
Updated: 2026-07-15
ID: 798
Track: architecture
Depends on: "729"
Related: "724, 727, ADR-040, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "729 (ADR-042 acceptance)"
Blocks v{N}: none
Priority: 2
Source: ADR-042 semantic registry migration
---

# 798 — ADR-042 semantic operation registry migration

## Summary

First implementation child of **#729**. After ADR-042 is accepted and the
parent epic is unblocked, establish the production `core-ops.toml` registry and
migrate intrinsic dispatch from callee-string matching to `FunctionId` +
`SignatureRegistry` lookup.

## Scope

- Define the registry schema, file location (`data/core-ops.toml`), and
  scaffold-exit criteria.
- Extend `SignatureEntry` with `semantic_id`, effect (orthogonal attribute set),
  const semantics, `inline_policy`, `lowering_kind`, target, and fallback.
- Assign `SemanticId` and `LoweringKind` to every builtin / intrinsic /
  runtime-ABI operation.
- Add `semantic_id` / `type_id` references to `std/manifest.toml`.
- Add generator / checker for cross-reference consistency, signature
  compatibility, unreferenced semantic operations, duplicate public bindings,
  effect/lowering consistency, and resolvable fallbacks.
- Switch emitter dispatch to `FunctionId`-based `SignatureRegistry` lookup.
- Remove callee-string dispatch (`eq(clone(callee), ...)` comparisons) and
  callee alias handling.

## Non-goals

- Do not implement ADR-042 while it is PROPOSED.
- Do not claim `core-ops.toml` is a current compiler SSOT while it is a scaffold.
- Do not maintain two authoritative registries during migration.
- Do not combine callee-string dispatch removal with the initial schema cutover.
- Do not migrate host ABI separation, stdlib inliner, or stdlib operations in
  this issue; those are separate child issues of #729.

## Phase order

1. **Registry schema and `SignatureEntry` extension** — define `data/core-ops.toml`
   schema, extend `SignatureEntry`, keep existing string dispatch.
2. **Semantic mapping** — assign `SemanticId` / `LoweringKind` to all existing
   builtins / intrinsics / runtime ABI operations.
3. **Consistency checks** — generator / checker validates `core-ops.toml` and
   `std/manifest.toml` references.
4. **FunctionId dispatch switch** — `call_*.ark` resolves `FunctionId` via
   `SignatureRegistry` and dispatches by `LoweringKind` / `SemanticOpId`.
5. **String dispatch removal** — delete `eq(clone(callee), ...)` comparisons and
   `normalize_callee_name`.

See [`docs/plans/intrinsic-layer-separation.md`](../../docs/plans/intrinsic-layer-separation.md)
for the canonical order and downstream work.

## Acceptance

- [ ] ADR-042 is ACCEPTED and #729 is unblocked before implementation begins
- [ ] `data/core-ops.toml` is the canonical registry file with schema_version = 2
- [ ] `std/manifest.toml` is the SSOT for public path / docs / stability /
      deprecation and references `core-ops.toml` via `semantic_id` / `type_id`
- [ ] `SignatureEntry` carries `semantic_id`, effect, const semantics,
      `inline_policy`, `lowering_kind`, target, and fallback
- [ ] Every builtin / intrinsic / runtime-ABI operation has a `SemanticId` and
      `LoweringKind` mapping
- [ ] Generator / checker rejects unreferenced semantic operations, duplicate
      public bindings, and signature / effect / lowering inconsistencies
- [ ] Emitter dispatch uses `FunctionId` + `SignatureRegistry` lookup
- [ ] No `eq(clone(callee), ...)` comparisons remain in `call_*.ark`
- [ ] `normalize_callee_name` and `__intrinsic_` prefix stripping are removed
- [ ] Differential tests pass between old string dispatch and new registry dispatch
- [ ] `python3 scripts/manager.py docs regenerate` and `python3 scripts/manager.py docs check` pass
- [ ] `python3 scripts/manager.py verify quick` passes

## Validation commands

- `python3 scripts/manager.py docs regenerate`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py quality structure`
- `python3 scripts/manager.py verify quick`
- Targeted registry signature, reference, drift, and differential tests added
  by the migration

## Completion evidence

Not started. ADR-042 is PROPOSED and #729 is blocked, so implementation is
blocked by design.

## Primary artifacts

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `data/core-ops.toml`
- `std/manifest.toml`
- `src/compiler/resolver/`
- `src/compiler/typechecker/`
- `src/compiler/mir/`
- `src/compiler/wasm/`
- `scripts/gen/`

## Remaining risks

- A partial cutover can create dual truth or silently change lowering.
- Signature-compatible entries can still differ in effects or fallback behavior.
- Rollback must restore the previous owner as a unit, not field by field.
- `func_id_raw` is a compile-local physical `FunctionId` representation;
  using it as a semantic key would create a fragile implicit ABI.

## References

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/plans/intrinsic-layer-separation.md`
- `docs/adr/ADR-040-typed-mir-signature-registry.md`
- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/796-cq-16-duplicated-knowledge-and-ssot-consolidation.md`
