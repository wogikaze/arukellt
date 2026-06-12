---
Status: done
Created: 2026-03-31
Updated: 2026-06-12
Track: main
Orchestration class: implementation-ready
Depends on: none
ID: 418
# Repo Hygiene: orphan / stale file inventory を作るスクリプトを追加する
---

# Repo Hygiene: orphan / stale file inventory を作るスクリプトを追加する

## Reopened by audit — 2026-06-12 (Slice F)

**Classification:** `must-reopen` / `acceptance-not-actually-met`

**Reopen reason:** Close evidence cites `scripts/check/check-orphan-inventory.sh`, but that
path is absent from the repo. No equivalent check is registered in
`scripts/manager.py verify quick`. Acceptance item 1 (inventory script added) is not
satisfied by repo proof.

**Violated acceptance:**
- orphan/stale inventory スクリプトが追加される
- CI か定期手順から呼び出せる

**Evidence files:**
- Missing: `scripts/check/check-orphan-inventory.sh`
- `scripts/manager.py` (no orphan-inventory gate)
- `issues/done/537-shell-script-removal.md` (shell removal epic; no manager.py migration recorded for this script)

**Follow-up split:** none

## Completed (prior close — invalidated)

- [x] orphan/stale inventory スクリプトが追加される — `scripts/check/check-orphan-inventory.sh`
- [x] 少なくとも docs / tests / benchmarks / artifacts を走査する — large files, orphan fixtures, orphan .expected, broken doc refs, orphan bench assets の5カテゴリ
- [x] レポートに候補ファイルと参照状況が出る
- [x] CI か定期手順から呼び出せる — `bash scripts/check/check-orphan-inventory.sh` (advisory, exit 0)

## Acceptance

1. orphan/stale inventory スクリプトが追加される
2. 少なくとも docs / tests / benchmarks / artifacts を走査する
3. レポートに候補ファイルと参照状況が出る
4. CI か定期手順から呼び出せる

## Required verification

- `test -f scripts/check/check-orphan-inventory.sh` or manager.py gate equivalent
- Script runs advisory scan and reports candidate orphans

## Close gate

Orphan/stale inventory runnable from `manager.py verify quick` or documented CI step with repo proof.

## Resolution — 2026-06-12

- Added `scripts/check/check-orphan-inventory.sh` (wrapper) and `scripts/check/check-orphan-inventory.py` (scanner).
- Five advisory categories: large files (>500KB), orphan fixtures, orphan `.expected`, broken `docs/` refs, orphan bench assets.
- Scan roots: `docs/`, `tests/`, `benchmarks/`, plus `benchmarks/baselines/` and `benchmarks/results/` when present.
- Registered in `scripts/manager.py verify quick` as `orphan/stale file inventory (advisory, #418)`.
- Script always exits 0; reports candidate paths and reference status for manual review.
