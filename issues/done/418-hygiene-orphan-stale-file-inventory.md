# Repo Hygiene: orphan / stale file inventory を作るスクリプトを追加する

**Status**: done
**Created**: 2026-03-31
**Closed**: 2026-07-28
**ID**: 418

## Completed

- [x] orphan/stale inventory スクリプトが追加される — `scripts/check-orphan-inventory.sh`
- [x] 少なくとも docs / tests / benchmarks / artifacts を走査する — large files, orphan fixtures, orphan .expected, broken doc refs, orphan bench assets の5カテゴリ
- [x] レポートに候補ファイルと参照状況が出る — 217 candidates detected (12 orphan fixtures, 2 orphan .expected, 188 broken doc refs, 14 orphan bench files, 1 large file)
- [x] CI か定期手順から呼び出せる — `bash scripts/check-orphan-inventory.sh` (advisory, exit 0)
