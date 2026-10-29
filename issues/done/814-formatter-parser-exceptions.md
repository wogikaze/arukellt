---
Status: done
Created: 2026-07-14
Updated: 2026-10-28
ID: 814
Track: code-quality
Depends on: "791"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved exceptions need open owner
---

# 814 — Formatter/parser exceptions (23 files)

## Summary

23 Ark files are exempted from formatter and parser checks via
content-addressed SHA256 exceptions. These files cannot be parsed by the
canonical formatter, preventing format enforcement.

## Exact failure scope

23 files in `src/compiler/` are listed in the formatter/parser exception
list. Each file has a SHA256 hash recorded in the exception baseline.

## Machine-readable baseline

The exception list is in `docs/data/ark-code-quality-baseline.toml` under
the formatter/parser exception section. Each entry has a file path and
SHA256 hash.

## Owner

compiler-tooling team

## Removal condition

Each file is removed from the exception list when the canonical formatter
can parse it without error. The SHA256 hash is removed and `fmt --check`
passes for that file.

## Validation command

```bash
python3 scripts/manager.py fmt --check
```

## Current count

23 exempted files

## New-failure ratchet

No new files may be added to the exception list. The count must only
decrease. Any new addition is a regression and blocks merge.

## Inventory — 2026-10-28 (exception mechanism no longer exists; close candidate)

The SHA256 formatter/parser exception mechanism described in this issue has
already been removed, and the current count is zero:

- `docs/data/ark-formatter-baseline.toml` does not exist on this checkout.
- `scripts/quality/checks.py` `_formatter_baseline()` returns `{}` when the file
  is absent, so no file is skipped as a baseline exception.
- `scripts/quality/structure.py` (CQ-STRUCT-009) explicitly treats a missing
  baseline as the desired end state of #791 and only enforces metadata
  (owner/issue/removal condition/recheck date) if the file is reintroduced.
- Issue #791 (dependency of this issue) is `done`; commit `3377a3712`
  (2026-09-25) removed all 11 remaining exceptions and deleted the baseline
  file. For the record, at this issue's creation (commit `d4e96fe03`,
  2026-07-15) the baseline listed exactly 23 entries (`grep -c 'path ='`),
  matching the issue title; that count is now 0.

Evidence:

- Invoked `python3 scripts/manager.py fmt --check`; exit 0,
  `fmt: checked=2001 failed=0 baseline-skipped=0` (2026-10-28; other lanes were
  editing `.ark` files in parallel and the run still had zero failures).
- `grep -rn 'formatter-baseline|parser exception' docs/ scripts/ docs/data/`
  finds only policy/description text:
  - `docs/data/code-quality-rules.toml` CQ-LINT-001 `exception_policy`
    ("tracked by issue 791" — wording is now historical since #791 is done)
  - `docs/current-state.md` and `docs/process/tooling-inventory.md`, both of
    which already state that #791 removed the last exceptions and deleted the
    file.
  - `scripts/quality/structure.py` + `scripts/quality/checks.py` (guard code for
    a future reintroduction) and `scripts/tests/test_quality_structure.py`
    (test fixture for that guard).
- No new-file ratchet is needed: there is no list to add to, and `quality
  structure` fails if a reintroduced baseline lacks metadata or has stale
  hashes.

Proposal: close as obsolete (superseded by #791). Residual docs nit for the
docs owner: CQ-LINT-001's `exception_policy` still says the exceptions are
"tracked by issue 791"; reword it or leave it as history since 791 is done.
No action taken here beyond this issue body.

## Resolution (2026-10-28)

Closed as obsolete (superseded by #791, done). The docs nit above was fixed in the
same change: `docs/data/code-quality-rules.toml` CQ-LINT-001 `exception_policy` now
says the exact-hash canonical parser exception baseline was retired with #791 and
must not be reintroduced. `python3 scripts/manager.py fmt --check` reports
`checked=2001 failed=0 baseline-skipped=0`.
