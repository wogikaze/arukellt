---
Status: done
Created: 2026-03-28
Updated: 2026-04-10
ID: 101
Track: compile-speed
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: True
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-10)."
デフォルト: "`--opt-level 1` (安全な最適化のみ)。"
# CLI: --opt-level 0/1/2 フラグ + Session 統合
---
# CLI: --opt-level 0/1/2 フラグ + Session 統合

---

## Reopened by audit — 2026-04-10


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/101-compile-opt-level-flag.md` — incorrect directory for an open issue.


## Summary

`arukellt compile --opt-level 0/1/2` フラグを実装し、
roadmap-v4.md §5.1 に定義された最適化レベル定義を Session に統合する。
デフォルト: `--opt-level 1` (安全な最適化のみ)。

## 最適化レベル定義

| レベル | 有効パス |
|--------|---------|
| `0` | なし (デバッグ用) |
| `1` | const_fold, branch_fold, dce, copy_prop, dead_local_elim |
| `2` | 全パス (licm, escape_analysis, inline, cse, strength_reduction, gc_hint) |

## 受け入れ条件

1. `crates/arukellt/src/main.rs` に `--opt-level` フラグ追加
2. `ark-driver/src/session.rs` の `Session` が `OptLevel` を受け取り各パスに伝播
3. `--opt-level 0` でのコンパイル時間が `--opt-level 2` より 30% 以上短い
4. 全 fixture が `--opt-level 0/1/2` 全レベルで同じ実行結果を出す
5. `--no-pass=<name>` で個別パスを無効化できる

## 参照

- roadmap-v4.md §5.1 および §6 item 6