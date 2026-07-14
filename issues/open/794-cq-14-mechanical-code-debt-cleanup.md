---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 794
Track: code-quality
Depends on: "793"
Orchestration class: blocked
Orchestration upstream: 793
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-14
---

# 794 — CQ-14: mechanical code debt cleanup

## Summary

Classify wrapper debt and remove only mechanically proven production compiler
debt while reducing hand-written 200-character lines to zero.

## Scope

- Classify wrappers as pure forwarder, semantic wrapper, boundary facade,
  record accessor/constructor, or ambiguous.
- Remove high-confidence unjustified forwarders and wrapper-only files.
- Split hand-written compiler lines of 200 or more characters by meaning.
- Remove mechanically provable dummy, unused private alias, and identity debt.

## Non-goals

- No language, ABI, target, or user-visible behavior change.
- No blanket deletion of wrappers or single-function files.
- No complexity-driven function splitting or baseline increase.

## Acceptance

- [ ] CQ-13 baseline regression is repaired and re-closed
- [ ] Hand-written production compiler lines of 200 or more characters are zero
- [ ] Wrapper classification distinguishes semantic and boundary responsibilities
- [ ] Unjustified pure forwarders are zero
- [ ] Wrapper-only single-function production files are zero
- [ ] Legitimate facade, adapter, accessor, validation, clone, and conversion boundaries remain
- [ ] Relevant compiler checks, fixtures, formatter, lint, and quality gates pass
- [ ] Before/after inventory is recorded below

## Validation commands

- `python3 scripts/check/check-ark-code-quality.py`
- `python3 scripts/check/check-ark-code-quality.py --report`
- `python3 scripts/manager.py fmt --check`
- `python3 scripts/manager.py lint`
- `python3 scripts/manager.py quality quick`
- `python3 scripts/manager.py verify quick`

## Completion evidence

Pending implementation and verification.

## Primary artifacts

- `scripts/quality/metrics.py`
- `scripts/check/check-ark-code-quality.py`
- `src/compiler/**/*.ark`
- `docs/data/ark-code-quality-baseline.toml`

## Remaining risks

- Ark cross-module visibility can make an apparent forwarder semantically necessary.
- Embedded resource strings may require ownership-aware extraction.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
