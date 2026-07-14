---
Status: done
Created: 2026-07-13
Updated: 2026-07-13
ID: 782
Track: tooling-contract
Depends on: "780, 781"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-03
---

# 782 — CQ-03: code-quality-rules.toml registry

## Summary

Implement CQ-03 from the quality closed-loop plan.

## Acceptance

- [x] Deliverables for CQ-03 land and `python3 scripts/manager.py verify quick` passes
- [x] Primary artifact: `docs/data/code-quality-rules.toml`

## Completion evidence

- `docs/data/code-quality-rules.toml` records scope, enforcement, severity, exception policy, owner, and ADR.
- `scripts/check/check-code-quality-contract.py` rejects incomplete, duplicate, or unowned rules.
- `python3 scripts/manager.py verify quick`: 177/177 passed (2026-07-13).
- Re-audit 2026-07-14: contract reports 19 complete rules; unit fixtures reject duplicate IDs and unknown command/CI references; `quality structure --json` returned zero findings.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
