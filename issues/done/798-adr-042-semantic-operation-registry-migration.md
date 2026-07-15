---
Status: done
Created: 2026-07-14
Updated: 2026-07-15
ID: 798
Parent: 729
Track: architecture
Depends on: none
Related: "724, 727, 808, 816, 817, 818, ADR-040, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: ADR-042 semantic registry migration
---

# 798 — ADR-042 semantic operation registry migration

## Summary

First implementation child of **#729**. ADR-042 is accepted and this issue is
ready to start. Establish the compiler-consumed migration CoreOpRegistry and
migrate emitter dispatch from callee-string matching to `FunctionId` + `SignatureRegistry` lookup.
Production lowering and final scaffold exit are owned by #818.

## Scope

- Define the CoreOpRegistry schema and file location (`data/core-ops.toml`).
- Extend `SignatureEntry` with an optional `core_op_id` and function signature.
  `CoreOpId` metadata (effect, lowering, fallback, inline policy, semantics)
  stays in `data/core-ops.toml`; `SignatureEntry` does not duplicate it.
- Replace `exposure` with three orthogonal fields:
  - `visibility` (`public` / `internal`)
  - `classification` (`layer` = `primitive` / `runtime` / `semantic_stdlib` / `target_raw`)
  - `binding` (`policy` = `required` / `optional` / `forbidden`)
- Define `TypeExpr` grammar for `signature.inputs` / `outputs` with `kind`
  discriminator (`ref`, `primitive`, `var`, `tuple`, `function`).
- Define manifest type-string to `TypeExpr` conversion and generic arity validation.
- Define `runtime_call` lowering as a discriminated union:
  - `internal` (`symbol` + `abi_version`)
  - `wit` (`package` + `interface` + `function` + `version`)
  - `native` (`backend` + `symbol` + `abi_version`)
- Define `target_id` as a backend-owned handler key.
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
- Remove callee-string dispatch (`eq(clone(callee), ...)` comparisons) from
  backend helpers. Freeze remaining compatibility aliases at the registry-build boundary.

## Non-goals

- Do not implement ADR-042 while it is PROPOSED.
- Do not claim `data/core-ops.toml` is a current compiler SSOT while it is a scaffold.
- Do not maintain two authoritative registries during migration.
- Do not combine callee-string dispatch removal with the initial schema cutover.
- Do not migrate host ABI separation, stdlib inliner, or stdlib operations in
  this issue; those are separate child issues of #729.
- Do not implement prelude restoration or sealed raw API; those are sibling
  children #816 and #817 under #729.
- Do not implement production Ark fallbacks or change the registry to
  `status = "production"`; that is #818.
- Do not fix #727; the runtime ABI / host bridge migration is downstream or
  parallel work, not a prerequisite for the registry schema.
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

- [x] ADR-042 is ACCEPTED and #729 is unblocked before implementation begins
- [x] `data/core-ops.toml` is the canonical CoreOpRegistry file with schema_version = 4
- [x] `std/manifest.toml` is the SSOT for public path / docs / stability /
      deprecation and references `core-ops.toml` via `core_op_id` / `type_id`
- [x] `SignatureEntry` carries only `core_op_id` and function signature;
      `CoreOpId` metadata is not duplicated into `SignatureEntry`
- [x] Every builtin / intrinsic / runtime-ABI operation has a `CoreOpId` and
      `LoweringKind` mapping
- [x] Generator / checker rejects:
      - unreferenced `required` public bindings (only when `visibility = "public"` and `binding.policy = "required"`)
      - public binding collisions
      - signature / effect / lowering / fallback / specialization inconsistencies
      - invalid `visibility` + `binding` combinations
      - specialization ambiguity
- [x] Shadow dispatch mode infrastructure compares legacy vs registry at
      `EffectiveLoweringDecision` level (`core_op_shadow.ark`)
- [x] Emitter dispatch uses `FunctionId` + `SignatureRegistry` lookup in routers
      (`inst_dispatch.ark`, `core_op_dispatch.ark`)
- [x] No `eq(clone(callee), ...)` comparisons remain in helper `call_*.ark` emitters
- [x] `normalize_callee_name` and `__intrinsic_` prefix stripping are removed
- [x] Targeted migration differential tests pass
- [x] T3 validation failure count does not increase beyond #808 baseline
- [x] `python3 scripts/manager.py docs regenerate` and `python3 scripts/manager.py docs check` pass
- [x] `python3 scripts/manager.py quality structure` passes
- [x] `data/core-ops.toml` uses explicit `status = "migration"`; every temporary
      `legacy_emitter` names its handler and tracking issue #818

## Validation commands

- `python3 scripts/check/check-core-ops.py`
- `python3 scripts/check/check-core-ops.py --strict`
- `python3 scripts/check/check-core-ops.py --production-structural-readiness`
- `python3 scripts/manager.py docs regenerate`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py quality structure`
- `python3 scripts/manager.py verify quick` (informational; global green is blocked by #808)
- Targeted registry signature, reference, drift, shadow, and differential tests added
  by the migration

## Completion evidence

Design scaffold and structural checker implemented:

- `data/core-ops.toml` schema_version = 4 with `visibility` / `classification` / `binding` axes
- `TypeExpr` grammar, manifest type-string parser with `generic_params` support, `core_op_id` / `type_id` references
- Binding contract enforcement (`public`/`internal`/`optional`/`forbidden`/`required`), public fallback path check
- Lowering variant closure, ADR-037-aligned capability/when values, specialization ambiguity detection
- `[validation]` split into `python` / `compiler` ownership
- `scripts/check/check-core-ops.py` (structural + manifest-semantic layer) PASS
- `scripts/tests/test_core_ops_checker.py` regression tests (48 cases) PASS

Compiler consumption, FunctionId router cutover, shadow infrastructure with
unresolved accounting, and handler-branch semantic mapping are implemented.
The backend dispatch owner is consolidated in `inst_dispatch.ark`; leaf helpers
receive generated integer handler IDs and contain no callee-string semantic
comparisons.

Close evidence (2026-07-15):

- ADR-042 D5/D6 audit: normal calls follow
  `FunctionId → SignatureEntry → CoreOpId → CoreOpRegistry`; metadata remains in
  `data/core-ops.toml`, and backend names are diagnostic/fallback lookup only.
- `docs/data/798-core-op-shadow-receipt.json`: 162/162 fixtures compiled,
  9,274/9,274 candidates matched, mismatched=0, unresolved=0.
- T3 ratchet: 210 pass, 213 validate-fail, 0 compile-fail, 23 skip; the 213
  failure identities are identical to the #808 baseline (new=0, removed=0).
- Core-op checker, strict checker, compiler validator, generated registry and
  binding freshness, frozen-alias inventory, and call-router string-dispatch
  ratchet all pass. Unit regressions: 56/56 pass.
- `python3 scripts/manager.py fmt --check`, `python3 scripts/manager.py lint`,
  `python3 scripts/manager.py docs regenerate`, `python3 scripts/manager.py docs check`,
  `python3 scripts/manager.py quality structure`, and
  `python3 scripts/manager.py verify quick` pass (171/171 quick checks).
- Production readiness intentionally rejects `status = "migration"`.
  Production lowerings, alias removal, and replacement of synthetic empty-signature
  alias entries with real function signatures are explicitly owned by open #818.

## Primary artifacts

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `data/core-ops.toml`
- `std/manifest.toml`
- `scripts/check/check-core-ops.py`
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

## Dependency Notes

- #798 used the `SignatureRegistry` spine already completed within #724.
  The type-inference removal and host-adapter work still open in #724 are not
  prerequisites for the bounded dispatch-spine migration or its close evidence.
- #724 remains related because its remaining semantic-spine cleanup can affect
  later production work, but it is not a hard dependency of this done issue.

## References

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/plans/intrinsic-layer-separation.md`
- `docs/adr/ADR-040-typed-mir-signature-registry.md`
- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
- `issues/open/729-intrinsic-layer-separation.md`
- `issues/open/808-t3-wasm-validation-failures.md`
- `issues/done/796-cq-16-duplicated-knowledge-and-ssot-consolidation.md`
