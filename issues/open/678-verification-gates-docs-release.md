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
checklist items for component interop smoke or P2 native wasmtime proof. The old
#675 host-capability request is superseded and closed by ADR-054; this issue now
covers the remaining generic audit and release-gate work, not restoration of the
deleted network APIs.

## Acceptance

- [x] `verify quick` gate: `docs/target-contract.md` vs `current-state.md` P2/component
      claims cannot drift silently
      — `check_docs_runtime_contract` in `scripts/check/check-docs-consistency.py`
      now resolves the live contract doc (`docs/platform/target-runtime-and-surfaces.md`,
      falling back to `docs/target-contract.md` if it ever exists) instead of
      silently skipping when the file is absent. It checks the emit-component
      surface row carries `contract_audit.component_emit_tier`, the doc keeps
      the WASI P2 / ADR-054 contract anchor for `p2_native_tier`, current-state
      keeps the wasmtime evidence anchor, and current-state may not upgrade a
      component surface claim past the contract tier.
- [x] `verify quick` gate: stdlib capability docs vs `std/manifest.toml` availability
      — new `check_capability_manifest_parity` in check-docs-consistency.py:
      `capabilities.toml` registered modules must appear in `std/manifest.toml`
      and have existing `path`s; every `std::host::*` module in the manifest
      must be documented in capabilities.toml.
- [x] `verify quick` gate: component support tier table vs `export_unsupported_*` manifest
      — gate-648 (`export_unsupported_*` fixture + `compile-error:` manifest
      contract) was already close-gate enforced; gate-673 (component-export
      tier2 matrix + E0401 rejection contract + retired encoder absence) is now
      also TRACKED in check-false-done-close-gates.py.
- [x] Close gate checker keeps current user-reachable capability claims aligned with
      the manifest (the superseded #675 network request no longer blocks this issue)
      — capability↔manifest parity check plus newly enforced done-issue gates
      (#668, #673 added to TRACKED; #668's gate was red while the issue sat in
      done/ — exactly the false-done class this issue targets).
- [x] Issue audit rules encoded in `scripts/check/` (not only prose in
      `false-done-prevention.md`):
  - [x] done issues must cite runnable fixture evidence
  - [x] compile-only proof cannot close runtime issues
  - [x] guard-only fixtures cannot close callable-import issues
  - [x] docs-only slices cannot close implementation issues
      — `scripts/check/check-issue-evidence-rules.py` encodes all four rules;
      pre-existing violations live in `docs/data/issue-evidence-rules-baseline.toml`
      (813 grandfathered); wired into `verify quick`.
- [x] `docs/release-checklist.md` items for:
  - [x] component interop smoke (`tests/component-interop/`)
      — pre-existing `check_component_interop_wasmtime` release-blocking check.
  - [x] P2 native wasmtime run (`gate_074` / `wasi_p2_native/*`)
      — new `check_p2_native_wasmtime` release-blocking check
      (`scripts/check/gate-668-p2-native-polish.py`).
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Update 2026-10-15

- Found while wiring: `gate-668-p2-native-polish.py` was RED on the current
  tree — it still expected the pre-`aaf4ba5b7` bare `blocking-write-and-flush`
  import name (emit now produces the canonical
  `[method]output-stream.blocking-write-and-flush`) and a 8192-byte component
  ceiling from the removed Rust host-linker path (official WASI P2 command
  world imports measure 10438). Expectations updated; gate now PASS and,
  more importantly, now runs at all — #668 was in done/ with a dead gate.
- `docs/target-contract.md` no longer exists; the contract-doc check in
  check-docs-consistency.py was passing vacuously. Resolved to the real doc.

## References

- `docs/process/false-done-prevention.md`
- `scripts/check/check-false-done-close-gates.py`
- `scripts/check/check-docs-consistency.py`
- `docs/release-checklist.md`
