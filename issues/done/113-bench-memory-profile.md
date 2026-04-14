# 計測: コンパイラ RSS + 実行時 GC ヒープ計測統合

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 113
**Depends on**: 100
**Track**: benchmark
**Blocks v4 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/113-bench-memory-profile.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

roadmap-v4.md §2 のメモリ使用量目標を達成・検証するための計測インフラを実装する。

| 目標 | 計測方法 |
|------|---------|
| コンパイラ RSS ≤ 100MB (1000行) | `/proc/self/status` VmRSS |
| GC ヒープ ≤ 入力サイズ比 3x | wasmtime --fuel + GC stats |

## 受け入れ条件

1. `arukellt compile --profile-memory` でコンパイラ自身の RSS ピーク値を stderr 出力
2. `arukellt run --wasm-stats output.wasm` で実行時 GC ヒープ最大値を表示
3. `scripts/run/verify-harness.sh` の memory gate: RSS > 100MB で failure
4. wasmtime GC stats が利用不可の場合は `VmRSS` のみで計測

## 参照

- roadmap-v4.md §2 (メモリ使用量目標)
