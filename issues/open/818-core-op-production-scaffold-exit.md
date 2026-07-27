---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 818
Parent: 729
Track: compiler-internal
Depends on: "798, 816, 817, 819, 820, 821, 822"
Related: "727, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "816, 817, 819, 820, 821, 822"
Blocks v{N}: none
Priority: 2
Source: CoreOpRegistry production scaffold exit
---

# 818 — CoreOpRegistry production scaffold exit

## Summary

Act as the final production-exit gate promised by ADR-042 after the bounded
implementation children complete. Integrate their verified outputs, remove the
now-unused migration bridge, and change `data/core-ops.toml` from `migration`
to `production` only after every production gate is satisfied.

## Scope

- Verify that #819–#822 replaced all `[operations.lowering.legacy]` entries
  with runtime/WIT, Ark fallback, MIR, or target-intrinsic production lowerings.
- Remove `legacy_bindings` after #816/#817 and the migration children no longer
  require compatibility aliases.
- Complete compiler-aware fallback resolution, signature, cycle, effect, and
  target-handler validation.
- Aggregate differential-test receipts from #819, #821, and #822.
- Remove the migration-only `legacy_emitter` lowering kind and generated alias bridge.
- Remove migration-only `ABI_KIND_CORE_OP_ALIAS` synthetic `SignatureEntry`
  records. Every remaining `SignatureEntry` must carry the real function
  signature required by ADR-042 D5/D6.
- Set `status = "production"` only after the structural and compiler-aware gates pass.
- Add registry/compiler/fixture-manifest hashes to the CoreOp shadow receipt and
  make `verify quick` reject a stale receipt.

## Non-goals

- Do not reopen the FunctionId → SignatureEntry → CoreOpId dispatch cutover owned by #798.
- Do not implement the full #816 or #817 scope here; consume their accepted outputs.
- Do not implement runtime ABI lowering, the stdlib-only inliner, or stdlib
  operation bodies here; those belong to #819–#822.
- Do not hide regressions by changing T3 baselines.

## Acceptance

- [ ] No `legacy_emitter` lowerings or `legacy_bindings` remain
- [ ] No synthetic core-op alias `SignatureEntry` remains; every registry entry
      carries its resolved function signature
- [ ] Every CoreOp has a production lowering and, where required, a resolvable Ark fallback
- [ ] #819–#822 are done and their differential receipts cover every migrated operation
- [ ] No host operation is implemented by `call_host_*` or `intrinsic_*` emitter helpers
- [ ] Compiler-aware fallback, signature, cycle, effect, and target-handler checks pass
- [ ] Differential tests pass for all migrated operations
- [ ] `data/core-ops.toml` has `status = "production"`
- [ ] CoreOp shadow receipt contains registry/compiler/fixture-manifest hashes,
      and its `verify quick` freshness gate passes
- [ ] `python3 scripts/check/check-core-ops.py --production-structural-readiness` passes
- [ ] `python3 scripts/manager.py verify full` passes

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/798-adr-042-semantic-operation-registry-migration.md`
- `issues/done/816-prelude-compilation-restoration.md`
- `issues/done/817-sealed-raw-api-module.md`
- `data/core-ops.toml`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
