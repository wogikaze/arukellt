# 実行時性能: 数値型の Narrowing — i32 優先使用

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 105
**Depends on**: —
**Track**: runtime-perf
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/105-runtime-number-type-narrowing.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Arukellt のデフォルト整数型は `i64` だが、値範囲が 32-bit に収まる場合は `i32` を使うことで
Wasm の i32 命令 (より高速なケースがある) を活用できる。
値範囲解析で `i64` ローカルが実際には `i32` 範囲しか使わないことを検出して narrowing する。

## 受け入れ条件

1. `passes/type_narrowing.rs`: 値範囲解析で i64 → i32 に narrowing できる変数を検出
2. T3 emitter で narrowing 後の変数を `i32` として emit
3. narrowing によるバイナリサイズ削減と実行速度改善をベンチマークで確認
4. i32/i64 境界での変換命令 (`i64.extend_i32_s`) を自動挿入

## 参照

- roadmap-v4.md §2 (fib(35) 1.5x 目標)
