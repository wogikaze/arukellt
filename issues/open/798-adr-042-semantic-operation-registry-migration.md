---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 798
Track: architecture
Depends on: none
Orchestration class: blocked
Orchestration upstream: ADR-042 acceptance
Blocks v{N}: none
Priority: 2
Source: CQ-16 scope correction
---

# 798 — ADR-042 semantic operation registry migration

## Summary

After ADR-042 is accepted, migrate semantic operation ownership from manual
resolver/typechecker/MIR/emitter registrations to a checked registry without a
period of unowned or dual-authoritative data.

## Scope

- Define the conditions that remove `status = "scaffold"` from
  `docs/data/core-ops.toml`.
- Add `semantic_id` / `type_id` references to `std/manifest.toml`.
- Migrate resolver, typechecker, MIR, and documentation generation in that order.
- Check signature compatibility, unreferenced semantic operations, and duplicate
  public bindings.
- Add deterministic generation and source/view drift checks.
- Differential-test generated lowering against existing fallback bodies.
- Remove callee-string dispatch in a later explicitly gated stage.

## Non-goals

- Do not implement ADR-042 while it is PROPOSED.
- Do not claim `core-ops.toml` is a current compiler SSOT while it is a scaffold.
- Do not maintain two authoritative registries during migration.
- Do not combine callee-string dispatch removal with the initial ownership cutover.

## Acceptance

- [ ] ADR-042 is ACCEPTED before implementation begins
- [ ] Scaffold exit criteria are recorded and satisfied
- [ ] Manifest semantic/type references have schema and compatibility checks
- [ ] Resolver, typechecker, MIR, and docs consume the selected owner in the stated order
- [ ] Unreferenced operations and duplicate public bindings are rejected
- [ ] Fallback bodies pass differential tests
- [ ] Each cutover has a rollback boundary and no dual-authoritative interval
- [ ] Callee-string dispatch removal is separately staged and verified

## Validation commands

- `python3 scripts/manager.py docs regenerate`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py quality structure`
- `python3 scripts/manager.py verify quick`
- Targeted registry signature, reference, drift, and differential tests added by the migration

## Completion evidence

Not started. ADR-042 is PROPOSED, so implementation is blocked by design.

## Primary artifacts

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/data/core-ops.toml`
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

## References

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
- `issues/open/796-cq-16-duplicated-knowledge-and-ssot-consolidation.md`
