---
Status: done
Created: 2026-07-13
Updated: 2026-07-13
ID: 786
Track: tooling-contract
Depends on: "781, 782"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-07
---

# 786 — CQ-07: manager.py lint entrypoint

## Summary

Implement CQ-07 from the quality closed-loop plan.

## Acceptance

- [x] Deliverables for CQ-07 land and `python3 scripts/manager.py verify quick` passes
- [x] Primary artifact: `scripts/manager.py`

## Completion evidence

- `scripts/manager.py lint [--fix] [paths...]` is the canonical lint entrypoint.
- Full lint completed for 1,981 Ark files with zero command failures; changed code denies `prefer-else-if`.
- `python3 scripts/manager.py verify quick`: 177/177 passed (2026-07-13).
- Re-audit 2026-07-14: top-level `manager.py lint` now includes correctness lint and lint-contract smoke; 1,981 files completed with zero unbaselined command failures, 24 exact-hash parser skips owned by #791, and smoke passed.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
