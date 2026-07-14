---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 795
Track: code-quality
Depends on: "794"
Orchestration class: blocked
Orchestration upstream: 794
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-15
---

# 795 — CQ-15: naming and control-flow cleanup

## Summary

Audit current hotspots and improve misleading names and unnecessarily nested
control flow without changing behavior or optimizing metric values for their own sake.

## Scope

- Audit the top 50 hotspots, complexity 25+, high-complexity/high-nesting, and
  parameter-count 10+ functions.
- Rename high-confidence ambiguous internal symbols with characterization coverage.
- Flatten invalid/error/non-applicable paths and clarify exclusive dispatch.

## Non-goals

- No public compatibility break, speculative abstraction, or table-driven rewrite
  solely to reduce complexity.
- No combined large rename and control-flow commit.

## Acceptance

- [ ] Top 50 hotspots and every complexity 25+ function have a recorded disposition
- [ ] High-complexity/high-nesting and parameter-count 10+ functions are audited
- [ ] Changed predicates and names match their observable contracts
- [ ] Normal paths are not unnecessarily nested
- [ ] No new unjustified wrapper or one-function file is introduced
- [ ] Renames are synchronized across code, tests, docs, and generated views
- [ ] Before/after hotspot report and behavior evidence are recorded below

## Validation commands

- `python3 scripts/manager.py quality report`
- `python3 scripts/manager.py fmt --check`
- `python3 scripts/manager.py lint`
- `python3 scripts/manager.py verify quick`
- Targeted self-checks and fixtures named in completion evidence

## Completion evidence

Pending implementation and verification.

## Primary artifacts

- `src/compiler/wasm/`
- `src/compiler/mir/lower/`
- `scripts/quality/metrics.py`

## Remaining risks

- Renames may cross dynamic string dispatch or generated documentation boundaries.
- Branch-dense compiler logic can be irreducible under the current contract.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
