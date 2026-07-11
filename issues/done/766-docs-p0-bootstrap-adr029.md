---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 766
Track: docs-audit
Depends on: 765
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs re-audit 2026-07-11 (P0-1)
Blocks: 770
---

# 766 — Docs P0: bootstrap trust model → ADR-029 pinned wasm

## Summary

Accepted ADR-029 and `state/compiler.md` use pinned `bootstrap/arukellt-selfhost.wasm`
as Stage 0. Current detail docs (`compiler/bootstrap.md`,
`process/bootstrap-verification.md`, parts of testing docs) still describe a
Rust Stage 0 / `verify-bootstrap.sh` model.

## Acceptance

- [x] `compiler/bootstrap.md` rewritten to ADR-029 / `manager.py selfhost fixpoint`
- [x] `process/bootstrap-verification.md` aligned or reduced to pointer + generated facts
- [x] Rust-era bootstrap narrative moved to `docs/history/reports/`
- [x] `state/compiler.md` does not treat retired `verify-bootstrap.sh` as completion criteria
- [x] Gate detects `Stage 0 (Rust)` / `trusted base (Rust compiler)` in current bootstrap docs
- [x] Docs-related verify gates pass

## References

- `docs/adr/ADR-029-selfhost-native-verification-contract.md`
- `scripts/run/arukellt-selfhost.sh`
- `python3 scripts/manager.py selfhost fixpoint`


## Completion

Completed 2026-07-11 as docs re-audit Phase 1.
