---
Status: done
Created: 2026-07-13
Updated: 2026-07-13
ID: 788
Track: tooling-contract
Depends on: "782"
Orchestration class: completed
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-09
---

# 788 — CQ-09: comment policy three kinds

## Summary

Implement CQ-09 from the quality closed-loop plan.

## Acceptance

- [x] Deliverables for CQ-09 land and `python3 scripts/manager.py verify quick` passes
- [x] Primary artifact: `docs/process/coding-conventions.md`

## Completion evidence

- `docs/process/coding-conventions.md` separates API docs, implementation rationale, and structured temporary debt.
- `scripts/check/check-comment-policy.py` enforces TODO/FIXME ownership, removal condition, and recheck date.
- `python3 scripts/manager.py verify quick`: 177/177 passed (2026-07-13).
- Re-audit 2026-07-14: canonical `quality quick` executed `check-comment-policy.py` and reported PASS; advisory public-API coverage remained non-blocking as registered.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
