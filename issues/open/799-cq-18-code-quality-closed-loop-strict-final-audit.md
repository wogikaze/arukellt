---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 799
Track: code-quality
Depends on: "797"
Orchestration class: in-progress
Orchestration upstream: 797
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop strict final audit
---

# 799 — CQ-18: code-quality closed-loop strict final audit

## Summary

Strictly audit CQ-01 through CQ-17 from repository evidence, canonical commands,
CI and GitHub ruleset state, generated views, exceptions, and a complete single
`verify full` run. This is a consistency audit, not release-readiness approval or
a new refactor wave.

## Scope

- Map every CQ issue to decision, implementation, command, CI, rule, tests,
  Acceptance, evidence, and remaining exception.
- Run every canonical quality/docs command and a single uninterrupted full gate.
- Compare failure identities, not only counts, with the recorded baseline.
- Inventory baselines, allowlists, exclusions, skips, and conditional CI paths.
- Manually sample wrapper, hotspot, API, target alias, generated view, comment,
  and SSOT classifications for checker blind spots.
- Read back the live GitHub ruleset and compare required status contexts.

## Non-goals

- No release-readiness claim.
- No CQ-19 refactor or broad cleanup hidden inside an audit.
- No baseline increase, unowned exception, or proposed-ADR implementation.

## CQ-01 through CQ-17 audit matrix

| CQ / issue | Decision artifact | Implementation artifact | Canonical command | CI job | Rule ID | Tests | Acceptance / evidence | Remaining exception |
|---|---|---|---|---|---|---|---|---|
| CQ-01 #780 | ADR-047/048 | accepted ADR headers | `quality quick` | `verify-quick` | CQ-REV-001 | ADR/docs gates | done; contracts exist | none |
| CQ-02 #781 | ADR-047 | `tooling-inventory.toml` | `quality structure` | `verify-quick` | CQ-STRUCT-009 | `test_quality_structure.py` | done; enforced entries have commands | deferred families are explicit |
| CQ-03 #782 | ADR-047 | `code-quality-rules.toml` | `quality structure` | `verify-quick` | CQ-STRUCT-009 | `test_quality_structure.py` | done; references validate | none |
| CQ-04 #783 | ADR-047 | `.editorconfig`, basics checker | `quality quick` | `verify-quick` | CQ-FMT-002 | quick gate fixtures | done; whitespace gate present | generated/binary exclusions |
| CQ-05 #784 | ADR-047 | `scripts/quality/checks.py`, manager dispatch | `fmt [--check]` | `quality-format` | CQ-FMT-001 | `test_manager.py`, `test_quality_metrics.py` | done; canonical selfhost formatter | #791 content-addressed exceptions |
| CQ-06 #785 | ADR-047 | formatted source plus ratchet | `fmt --check` | `quality-format` | CQ-FMT-001/002 | format pass/fail tests | done; format-only wave retained | #791 |
| CQ-07 #786 | ADR-047 | quality lint domain and smoke | `lint` | `quality-lint` | CQ-LINT-001..003 | manager/quality tests | done; exit contract verified | parser exceptions owned by #791 |
| CQ-08 #787 | ADR-047/048 | Ark quality baseline/checker | `quality quick` | `verify-quick` | CQ-STRUCT-001 | metric/baseline tests | done; lower-only ratchets | count baselines listed below |
| CQ-09 #788 | ADR-047/048 | comment policy/conventions | `quality full` | `verification` | CQ-DOC-001/002 | `test_comment_policy.py` | done; three comment kinds retained | natural language remains advisory |
| CQ-10 #789 | ADR-047/048 | AGENTS/review checklist | `docs check` | `docs` | CQ-REV-001 | docs gates | done; process surfaces synchronized | none |
| CQ-11 #790 | ADR-047 | CI jobs/required-check docs | workflow jobs | `quality-format`, `quality-lint`, `verify-quick`, `verify` | CQ-STRUCT-009 | `test_ci_category_summary.py` | live ruleset readback required below | emergency bypass requires record |
| CQ-12 #792 | ADR-047/048 | `scripts/quality/structure.py` | `quality structure` | `verify-quick` | CQ-STRUCT-002..009 | `test_quality_structure.py` | done; text/JSON same model | declared facade cycles only |
| CQ-13 #793 | ADR-047/048 | metrics/baseline shared model | `quality report` | `verification` | CQ-METRIC-001/002 | `test_quality_metrics.py` | done; advisory report, explicit write | churn unknown without Git |
| CQ-14 #794 | ADR-048 | debt classifier/source cleanup | `quality report` | `verification` | CQ-STRUCT-001 | wrapper classifier tests | done; long lines/pure debt zero | semantic/boundary wrappers retained |
| CQ-15 #795 | ADR-048 | hotspot audit/targeted cleanup | `quality report` | `verification` | CQ-METRIC-001 | characterization evidence in issue | done; 50 hotspots audited | branch-dense contracts retained |
| CQ-16 #796 | ADR-007/047/048 | target SSOT/views; phase/Vec dedupe | `quality structure` | `verify-quick` | CQ-STRUCT-007/009 | `test_target_contract.py` | done at `9f771087` | ADR-042 migration #798 |
| CQ-17 #797 | ADR-044/046/047/048 | comment checker/API docs | `quality full` | `verification` | CQ-API-001, CQ-DOC-003..006 | `test_comment_policy.py` | done at `7e08ab67` | natural-language advisories |

## Acceptance

- [ ] CQ-01 through CQ-17 contain no false-done state
- [ ] CQ-16 and CQ-17 are closed in dependency order
- [ ] Target aliases canonicalize once and old spellings remain only in allowed scopes
- [ ] Proposed ADR-042 and current core-ops ownership are described accurately
- [ ] Commands, CI, rule registry, required checks, CODEOWNERS, and live ruleset agree
- [ ] No baseline increase or newly unowned exception exists
- [ ] Formatter, linter, structure, comment, docs, unit, and quick gates pass
- [ ] `verify full` completes once and all failures are identity-classified
- [ ] Docs regenerate without drift and issue indexes are generated
- [ ] Manual samples and ambiguity/false-positive/false-negative results are recorded
- [ ] Worktree is clean at a committed final HEAD

## Validation commands

All commands listed in the CQ-18 work order, including the single-process
`python3 scripts/manager.py verify full`, plus live `gh api` ruleset readback.

## Completion evidence

In progress. Command receipts, exception inventory, manual samples, ruleset
readback, and full-gate identity comparison will be added before close review.

## Primary artifacts

- `issues/done/780-*` through `issues/done/797-*`
- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
- `docs/data/code-quality-rules.toml`
- `docs/data/tooling-inventory.toml`
- `.github/workflows/ci.yml`
- `.github/rulesets/master-quality.json`
- `.github/CODEOWNERS`

## Remaining risks

- Full verification has known release blockers; identity comparison is mandatory.
- Live GitHub ruleset readback depends on repository administration visibility.
- Historical skip mechanisms outside the code-quality loop need ownership
  classification, not automatic deletion.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
- `docs/process/false-done-prevention.md`
