---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 678
Track: tooling-contract
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: P2 verification / false-done prevention checklist audit 2026-06-17
---

# 678 — Verification gates: stale docs, release checklist, and close-gate coverage

## Summary

False-done infrastructure (`check-false-done-close-gates.py`, FD-01–FD-10) covers
many done issues but audit checklist gaps remain: no verify-quick gate for stale
target-matrix / stdlib-capability / component-support-table docs; no release
checklist items for component interop smoke or P2 native wasmtime proof; no close
gate enforcing user-reachable host capability claims (#675 dependency).

## Acceptance

- [ ] `verify quick` gate: `docs/target-contract.md` vs `current-state.md` P2/component
      claims cannot drift silently
- [ ] `verify quick` gate: stdlib capability docs vs `std/manifest.toml` availability
- [ ] `verify quick` gate: component support tier table vs `export_unsupported_*` manifest
- [ ] Close gate checker registered for user-reachable host capabilities (blocks #675
      close until green)
- [ ] Issue audit rules encoded in `scripts/check/` (not only prose in
      `false-done-prevention.md`):
  - [ ] done issues must cite runnable fixture evidence
  - [ ] compile-only proof cannot close runtime issues
  - [ ] guard-only fixtures cannot close callable-import issues
  - [ ] docs-only slices cannot close implementation issues
- [ ] `docs/release-checklist.md` items for:
  - [ ] component interop smoke (`tests/component-interop/`)
  - [ ] P2 native wasmtime run (`gate_074` / `wasi_p2_native/*`)
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `docs/process/false-done-prevention.md`
- `scripts/check/check-false-done-close-gates.py`
- `scripts/check/check-docs-consistency.py`
- `docs/release-checklist.md`
