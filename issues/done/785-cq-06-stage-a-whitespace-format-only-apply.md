---
Status: done
Created: 2026-07-13
Updated: 2026-07-13
ID: 785
Track: tooling-contract
Depends on: "783, 784"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-06
---

# 785 — CQ-06: Stage A whitespace format-only apply

## Summary

Implement CQ-06 from the quality closed-loop plan.

## Acceptance

- [x] Deliverables for CQ-06 land and `python3 scripts/manager.py verify quick` passes
- [x] Primary artifact: `scripts/check/check-editorconfig-basics.py`

## Completion evidence

- `python3 scripts/manager.py fmt` completed for 1,981 Ark files; 26 exact-hash exceptions are tracked by #791.
- Formatter parity fixtures cover delimiters in strings and interleaved imports without declaration loss.
- `python3 scripts/manager.py verify quick`: 177/177 passed (2026-07-13).
- Re-audit 2026-07-14: full canonical `fmt --check` and formatter fixtures passed; the 26 parse gaps remain fail-closed by content hash and are not claimed complete under this issue.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
