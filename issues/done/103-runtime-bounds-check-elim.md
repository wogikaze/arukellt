# 実行時性能: 配列境界チェック除去 (Bounds Check Elimination)

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 103
**Depends on**: 080
**Track**: runtime-perf
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/103-runtime-bounds-check-elim.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

配列アクセス `a[i]` に対して毎回 `i < a.len()` の境界チェックを行っているが、
ループ内アクセスでインデックスが 0..len の範囲にあることが静的に証明できる場合は
チェックを除去 (または1回だけにホイスト) する。
LICM パス (#080) と組み合わせることで効果を最大化する。

## 受け入れ条件

1. `passes/bounds_check_elim.rs`: ループ不変の境界チェックをループ前にホイスト
2. `for i in 0..a.len()` パターンの境界チェックを完全除去
3. `vec_push_pop.ark` ベンチマークで境界チェック有り比 20% 以上改善
4. 境界チェック除去後も out-of-bounds アクセスが runtime trap することを確認

## 参照

- roadmap-v4.md §2 (vec-ops 2.0x 目標)
