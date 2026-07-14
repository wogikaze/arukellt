---
Status: done
Created: 2026-07-14
Updated: 2026-07-14
ID: 794
Track: code-quality
Depends on: "793"
Orchestration class: completed
Orchestration upstream: None
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

- [x] CQ-13 baseline regression is repaired and re-closed
- [x] Hand-written production compiler lines of 200 or more characters are zero
- [x] Wrapper classification distinguishes semantic and boundary responsibilities
- [x] Unjustified pure forwarders are zero
- [x] Wrapper-only single-function production files are zero
- [x] Legitimate facade, adapter, accessor, validation, clone, and conversion boundaries remain
- [x] Relevant compiler checks, fixtures, formatter, lint, and quality gates pass
- [x] Before/after inventory is recorded below

## Validation commands

- `python3 scripts/check/check-ark-code-quality.py`
- `python3 scripts/check/check-ark-code-quality.py --report`
- `python3 scripts/manager.py fmt --check`
- `python3 scripts/manager.py lint`
- `python3 scripts/manager.py quality quick`
- `python3 scripts/manager.py verify quick`

## Completion evidence

CQ-13 was re-closed by `1ac840dd` after the shared `tomllib` baseline model and
section-preservation regressions passed 97 unit tests and full quality/docs checks.

| Inventory | Before | After |
|---|---:|---:|
| compiler Ark files | 1,900 | 1,899 |
| functions | 10,201 | 10,191 |
| lines >= 200 | 437 | 0 |
| legacy thin wrappers | 1,733 | 1,726 |
| single-function files | 530 | 529 |
| unjustified pure forwarders | 1 | 0 |
| wrapper-only single-function files | 1 | 0 |

`scripts/quality/debt.py` classifies logical statements rather than raw lines.
The final advisory classes are: pure forwarder 608, semantic wrapper 2,034,
boundary facade 852, record accessor/constructor 1,435, ambiguous 5,262. The
pure total is not a deletion target: imported facades and module-visibility
bridges remain. Only the unreferenced `lsp/diag_json.ark` forwarder was removed.

The 437 long lines were split without changing embedded lifecycle/template
bytes; concatenated outputs were characterized before and after. Selfhost S2
build/validation and fmt parity pass. Full fixture parity remains the known
repository blocker `PASS=804 FAIL=367 SKIP=417`; the isolated pre-CQ-14 commit
produces the exact same counts, so it is not CQ-14 regression evidence and is
not claimed as a passing release gate.

Relevant commits: `db4bfd0f`, `ae7b304e`, `2c688b6b`, `83193e95`, `904600b3`.

## Primary artifacts

- `scripts/quality/metrics.py`
- `scripts/check/check-ark-code-quality.py`
- `src/compiler/**/*.ark`
- `docs/data/ark-code-quality-baseline.toml`

## Remaining risks

- Ark cross-module visibility can make an apparent forwarder semantically necessary.
- Embedded resource strings may require ownership-aware extraction.
- The repository-wide fixture parity incident remains owned by issue #287; CQ-14
  established unchanged counts but did not resolve that incident.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
