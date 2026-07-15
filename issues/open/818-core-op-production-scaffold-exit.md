---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 818
Parent: 729
Track: compiler-internal
Depends on: "798, 816, 817"
Related: "727, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "798, 816, 817"
Blocks v{N}: none
Priority: 2
Source: CoreOpRegistry production scaffold exit
---

# 818 — CoreOpRegistry production scaffold exit

## Summary

Complete the production implementation promised by ADR-042 after the #798
dispatch-spine migration. Replace migration-only emitter lowerings and frozen
compatibility aliases with Ark fallback bodies, runtime/WIT calls, MIR ops, or
target intrinsics, then change `data/core-ops.toml` from `migration` to
`production` only after every production gate is satisfied.

## Scope

- Replace all `[operations.lowering.legacy]` entries with production lowerings.
- Remove `legacy_bindings` after #816 restores compiled prelude bodies and the
  required representation subset of #817 provides stable Vec/String raw operations.
- Complete compiler-aware fallback resolution, signature, cycle, effect, and
  target-handler validation.
- Add differential tests for every migrated operation.
- Remove the migration-only `legacy_emitter` lowering kind and generated alias bridge.
- Remove migration-only `ABI_KIND_CORE_OP_ALIAS` synthetic `SignatureEntry`
  records. Every remaining `SignatureEntry` must carry the real function
  signature required by ADR-042 D5/D6.
- Set `status = "production"` only after the structural and compiler-aware gates pass.

## Non-goals

- Do not reopen the FunctionId → SignatureEntry → CoreOpId dispatch cutover owned by #798.
- Do not implement the full #816 or #817 scope here; consume their accepted outputs.
- Do not hide regressions by changing T3 baselines.

## Acceptance

- [ ] No `legacy_emitter` lowerings or `legacy_bindings` remain
- [ ] No synthetic core-op alias `SignatureEntry` remains; every registry entry
      carries its resolved function signature
- [ ] Every CoreOp has a production lowering and, where required, a resolvable Ark fallback
- [ ] Compiler-aware fallback, signature, cycle, effect, and target-handler checks pass
- [ ] Differential tests pass for all migrated operations
- [ ] `data/core-ops.toml` has `status = "production"`
- [ ] `python3 scripts/check/check-core-ops.py --production-structural-readiness` passes
- [ ] `python3 scripts/manager.py verify full` passes

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/798-adr-042-semantic-operation-registry-migration.md`
- `issues/open/816-prelude-compilation-restoration.md`
- `issues/open/817-sealed-raw-api-module.md`
- `data/core-ops.toml`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
