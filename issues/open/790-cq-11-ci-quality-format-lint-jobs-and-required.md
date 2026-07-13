---
Status: open
Created: 2026-07-13
Updated: 2026-07-13
ID: 790
Track: tooling-contract
Depends on: "784, 786, 787"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-11
---

# 790 — CQ-11: CI quality-format/lint jobs and required-checks docs

## Summary

Implement CQ-11 from the quality closed-loop plan.

## Acceptance

- [ ] Deliverables for CQ-11 land and `python3 scripts/manager.py verify quick` passes
- [ ] Primary artifact: `.github/workflows/ci.yml`

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
