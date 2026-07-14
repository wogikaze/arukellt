---
Status: open
Created: 2026-07-14
Updated: 2026-07-15
ID: 798
Track: architecture
Depends on: "724, 727"
Related: "729, 808, 816, 817, ADR-040, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: ADR-042 acceptance
Blocks v{N}: none
Priority: 2
Source: ADR-042 semantic registry migration
---

# 798 — ADR-042 semantic operation registry migration

## Summary

First implementation child of **#729**. After ADR-042 is accepted, establish
the production `data/core-ops.toml` CoreOpRegistry and migrate intrinsic dispatch
from callee-string matching to `FunctionId` + `SignatureRegistry` lookup.

## Scope

- Define the CoreOpRegistry schema, file location (`data/core-ops.toml`), and
  scaffold-exit criteria.
- Extend `SignatureEntry` with an optional `core_op_id` and function signature.
  `CoreOpId` metadata (effect, lowering, fallback, inline policy, semantics)
  stays in `data/core-ops.toml`; `SignatureEntry` does not duplicate it.
- Replace `exposure` with three orthogonal fields:
  - `visibility` (`public` / `internal`)
  - `classification` (`layer` = `primitive` / `runtime` / `semantic_stdlib` / `normal_stdlib` / `target_raw`)
  - `binding` (`policy` = `required` / `optional` / `forbidden`)
- Define `TypeExpr` grammar for `signature.inputs` / `outputs` with `kind`
  discriminator (`ref`, `primitive`, `var`, `tuple`, `function`).
- Assign `CoreOpId` and `LoweringKind` to every builtin / intrinsic /
  runtime-ABI operation.
- Add `core_op_id` / `type_id` references to `std/manifest.toml`.
- Add generator / checker for cross-reference consistency, signature
  compatibility, effect/lowering consistency, unreferenced required bindings,
  public binding collisions, specialization ambiguity, and resolvable fallbacks.
- Implement a shadow dispatch mode that compares the old callee-string dispatch
  result with the new `SignatureRegistry` result, gating dispatch cutover on
  100% agreement of `EffectiveLoweringDecision` (after capability resolution).
- Switch emitter dispatch to `FunctionId`-based `SignatureRegistry` lookup.
- Remove callee-string dispatch (`eq(clone(callee), ...)` comparisons) and
  callee alias handling.

## Non-goals

- Do not implement ADR-042 while it is PROPOSED.
- Do not claim `data/core-ops.toml` is a current compiler SSOT while it is a scaffold.
- Do not maintain two authoritative registries during migration.
- Do not combine callee-string dispatch removal with the initial schema cutover.
- Do not migrate host ABI separation, stdlib inliner, or stdlib operations in
  this issue; those are separate child issues of #729.
- Do not implement prelude restoration or sealed raw API; those are #816 and #817.
- Do not fix #808; the global `verify quick` green gate is a pre-existing
  compiler problem and remains in #808. This issue may ratchet the T3 failure
  count but does not require it to be zero.

## Phase order

1. **CoreOpRegistry schema and `SignatureEntry` extension** — define
   `data/core-ops.toml` schema (schema_version = 4), extend `SignatureEntry`,
   keep existing string dispatch.
2. **CoreOpId / LoweringKind assignment** — assign `CoreOpId` / `LoweringKind`
   to all existing builtins / intrinsics / runtime ABI operations.
3. **Shadow validation and consistency checks** — generator / checker validates
   `core-ops.toml` and `std/manifest.toml` references; shadow dispatch reaches
   100% agreement.
4. **FunctionId dispatch switch** — `call_*.ark` resolves `FunctionId` via
   `SignatureRegistry` and dispatches by `CoreOpId` / `LoweringKind`.
5. **String dispatch removal** — delete `eq(clone(callee), ...)` comparisons and
   `normalize_callee_name`.

See [`docs/plans/intrinsic-layer-separation.md`](../../docs/plans/intrinsic-layer-separation.md)
for the canonical order and downstream work.

## Acceptance

- [ ] ADR-042 is ACCEPTED and #729 is unblocked before implementation begins
- [ ] `data/core-ops.toml` is the canonical CoreOpRegistry file with schema_version = 4
- [ ] `std/manifest.toml` is the SSOT for public path / docs / stability /
      deprecation and references `core-ops.toml` via `core_op_id` / `type_id`
- [ ] `SignatureEntry` carries only `core_op_id` and function signature;
      `CoreOpId` metadata is not duplicated into `SignatureEntry`
- [ ] Every builtin / intrinsic / runtime-ABI operation has a `CoreOpId` and
      `LoweringKind` mapping
- [ ] Generator / checker rejects:
      - unreferenced `required` public bindings (only when `visibility = "public"` and `binding.policy = "required"`)
      - public binding collisions
      - signature / effect / lowering / fallback / specialization inconsistencies
      - specialization ambiguity
- [ ] Shadow dispatch mode shows 100% agreement between old string dispatch and
      new `SignatureRegistry` dispatch at `EffectiveLoweringDecision` level
- [ ] Emitter dispatch uses `FunctionId` + `SignatureRegistry` lookup
- [ ] No `eq(clone(callee), ...)` comparisons remain in `call_*.ark`
- [ ] `normalize_callee_name` and `__intrinsic_` prefix stripping are removed
- [ ] Targeted migration differential tests pass
- [ ] T3 validation failure count does not increase beyond #808 baseline
- [ ] `python3 scripts/manager.py docs regenerate` and `python3 scripts/manager.py docs check` pass
- [ ] `python3 scripts/manager.py quality structure` passes

## Validation commands

- `python3 scripts/manager.py docs regenerate`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py quality structure`
- `python3 scripts/manager.py verify quick` (informational; global green is blocked by #808)
- Targeted registry signature, reference, drift, shadow, and differential tests added
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
- `issues/open/808-t3-wasm-validation-failures.md`
- `issues/done/796-cq-16-duplicated-knowledge-and-ssot-consolidation.md`
