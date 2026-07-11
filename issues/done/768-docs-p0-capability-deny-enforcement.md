---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 768
Track: docs-audit
Depends on: 765
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs re-audit 2026-07-11 (P0-3)
Blocks: 770
---

# 768 — Docs P0: capability deny enforcement single story

## Summary

`process/policy.md` lists `--deny-clock` / `--deny-random` as `future enforcement`
while `current-state.md` and platform docs assert compile-time MIR scan enforcement.

## Acceptance

- [x] `process/policy.md` capability table uses Default / Deny mechanism / Enforcement phase / Transitive
- [x] clock/random match current-state (compile-time MIR, transitive, `run` only)
- [x] No `future enforcement` for implemented deny flags
- [x] Docs-related verify gates pass

## References

- `docs/current-state.md` Known Limitations
- `docs/platform/target-runtime-and-surfaces.md` capability deny section


## Completion

Completed 2026-07-11 as docs re-audit Phase 1.
