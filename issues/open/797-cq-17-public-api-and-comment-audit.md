---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 797
Track: code-quality
Depends on: "796"
Orchestration class: blocked
Orchestration upstream: 796
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-17
---

# 797 — CQ-17: public API and comment audit

## Summary

Classify Ark public visibility into external API, stable subsystem boundary,
and internal cross-module visibility, then apply documentation and comment policy
without blanket comments or compatibility breaks.

## Scope

- Derive A/B/C API classification from existing manifests, facades, imports,
  exports, CLI surface, and ownership.
- Remove only proven unnecessary public aliases/re-exports/visibility.
- Document external contracts and non-obvious stable boundaries.
- Remove boilerplate headers, issue-only markers, stale/commented-out code, and
  malformed temporary debt.
- Extend comment-policy findings and tests with high-confidence hard errors.

## Non-goals

- No blanket doc requirement for all compiler `pub fn`.
- No giant hand-maintained symbol allowlist or user-facing API change.
- No natural-language style guess promoted to a hard gate.

## Acceptance

- [ ] API is classified into A/B/C and counts are recorded
- [ ] External API documentation coverage is 100%
- [ ] Required stable-boundary contracts are documented
- [ ] Internal cross-module visibility is not blanket-failed
- [ ] Proven unnecessary public surface is removed without boundary bypass
- [ ] Boilerplate headers, issue-only comments, and commented-out production code are zero
- [ ] Remaining TODO/FIXME entries are structured
- [ ] Comment-policy checker and tests cover classification and attachment contracts
- [ ] Before/after visibility, documentation, and comment inventories are recorded below

## Validation commands

- `python3 scripts/check/check-comment-policy.py`
- `python3 scripts/manager.py quality quick`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py verify quick`
- Targeted API/export fixtures named in completion evidence

## Completion evidence

Pending implementation and verification.

## Primary artifacts

- `scripts/check/check-comment-policy.py`
- `src/compiler/**/*.ark`
- `std/**/*.ark`
- `docs/data/code-quality-rules.toml`

## Remaining risks

- Ark `pub` can mean module visibility rather than external contract.
- Export reachability and dynamic CLI dispatch require characterization evidence.

## References

- `docs/adr/ADR-044-trait-method-syntax-adopted.md`
- `docs/adr/ADR-046-free-function-eradication.md`
- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
