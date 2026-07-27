---
Status: done
Created: 2026-07-14
Updated: 2026-07-21
ID: 811
Track: selfhost
Depends on: none
Orchestration class: done
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 811 — Selfhost CLI parity drift

## Summary

`verify full` reported 2 selfhost CLI parity failures. The selfhost
compiler's CLI output differed from the reference for 2 commands.

## Exact failure scope

2 CLI commands produced different output between the selfhost compiler and
the reference. Fixed 2026-07-20; closed 2026-07-21 after re-verification.

## Validation command

```bash
python3 scripts/manager.py selfhost parity --mode --cli
```

## Close evidence (2026-07-21)

Re-ran on worktree with current `arukellt-s2.wasm`:

```text
cli-parity: PASS=19 FAIL=0
✓ all 19 CLI parity cases pass
```

Including previously failing cases:

- `--help` (matches golden)
- `compose --validate` (exit 0, graph printed)

### Root causes and fixes (landed 2026-07-20)

- `--help` drift: golden `tests/snapshots/selfhost/cli-help.txt` updated for
  lint global options (`--list`, `--local`, `--allow`, `--deny`).
- `compose --validate`: harness now preopens the temporary directory first and
  uses it as cwd (`scripts/selfhost/checks.py`).

Commits: `fe66b9c5`, merge `31247a19`.

## Acceptance

- [x] Selfhost CLI parity reports `FAIL=0`
- [x] `python3 scripts/manager.py selfhost parity --mode --cli` exits 0
- [x] No new CLI parity mismatches introduced (ratchet: count only decreases)

## New-failure ratchet

No new CLI parity mismatches may be added. The count must only decrease.
