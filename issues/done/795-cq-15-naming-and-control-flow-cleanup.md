---
Status: done
Created: 2026-07-14
Updated: 2026-07-14
ID: 795
Track: code-quality
Depends on: "794"
Orchestration class: completed
Orchestration upstream: None
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

- [x] Top 50 hotspots and every complexity 25+ function have a recorded disposition
- [x] High-complexity/high-nesting and parameter-count 10+ functions are audited
- [x] Changed predicates and names match their observable contracts
- [x] Normal paths are not unnecessarily nested
- [x] No new unjustified wrapper or one-function file is introduced
- [x] Renames are synchronized across code, tests, docs, and generated views
- [x] Before/after hotspot report and behavior evidence are recorded below

## Validation commands

- `python3 scripts/manager.py quality report`
- `python3 scripts/manager.py fmt --check`
- `python3 scripts/manager.py lint`
- `python3 scripts/manager.py verify quick`
- Targeted self-checks and fixtures named in completion evidence

## Completion evidence

The post-CQ-14 report identified 50 hotspots, 35 functions with complexity >=25,
and 59 functions with at least 10 parameters. Every item was reviewed. The
parameter group consists of record constructors whose positional ABI is already
consumed and Wasm emit helpers that carry instruction state together; moving
them into an unbounded context record would only hide the contract.

Top-50 dispositions (rank is the report rank before final comment-only churn):

- simplified/flattened: 7 `should_gc_ref_cast_to_dest` (complexity 22 -> 20,
  open-enum narrowing normal path moved left)
- renamed: 4 `ctx_is_void_builtin` -> `is_void_returning_builtin`; 13
  `type_name_is_ref_type` -> `is_ref_type_name`
- preserved, boundary/adapter: 1-3, 5-6, 8-9, 16-17, 21-23, 25-28, 30,
  34-35, 38-40, 43-45, 48, 50
- preserved, branch-dense by contract: 10-12, 14-15, 18-20, 24, 29,
  31-33, 36-37, 41-42, 46-47, 49

The 35 complexity>=25 functions were also checked outside the top 50. Their
remaining density is dominated by explicit builtin/operator spelling sets,
SIMD dispatch, parser/type-shape classification, or result/enum layout. None
had two independent responsibilities that justified remote one-use helpers.
High nesting plus high complexity items received the same boundary or
branch-dense disposition. `trait_hir_self_check` also replaced an issue-number
name without changing its deterministic output contract.

Before/after distribution maxima remain complexity 55 and nesting 14; these are
advisory, not scores. Functions decreased 10,201 -> 10,191 through CQ-14/16
duplicate removal. The final hotspot order changed with git churn, as expected;
the changed cast predicate remains complexity 20. Targeted fmt/lint, S2
validation, selfhost fmt parity, comment-policy tests, and verify quick provide
behavior evidence. Relevant commits: `4e5b79f7`, `3b4c6956`, `f0ccd9ed`.

## Primary artifacts

- `src/compiler/wasm/`
- `src/compiler/mir/lower/`
- `scripts/quality/metrics.py`

## Remaining risks

- Renames may cross dynamic string dispatch or generated documentation boundaries.
- Branch-dense compiler logic can be irreducible under the current contract.
- Metric order is churn-sensitive; preserved entries must be reconsidered when
  their contract changes, not merely when rank changes.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
