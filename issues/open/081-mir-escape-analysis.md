# MIR: エスケープ解析 + Scalar Replacement パス

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 081
**Depends on**: —
**Track**: mir-opt
**Blocks v4 exit**: yes


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/081-mir-escape-analysis.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`crates/ark-mir/src/opt/escape_analysis.rs` を実装し、
関数外にエスケープしない struct を scalar (個々のローカル変数) に分解する。
`binary_tree` ベンチマークの 1.83x 問題 (ADR-002 記録) をこのパスで改善することが
roadmap-v4.md §10 item 7 で明示的な目標になっている。

## 対象パターン

- ローカル関数内のみで使用され、`return`・`call` の引数・`store` への書き込みがない struct
- 小さい (フィールド数 ≤ 4) 短命な struct (ノードオブジェクト、座標、タプル相当)

## 受け入れ条件

1. `passes/escape_analysis.rs`: エスケープしない struct を `LocalId` 群に展開
2. `OptimizationPass::EscapeAnalysis` を追加
3. `binary_tree.ark` (depth=15) ベンチマークで C 比 1.5x 以内を目標
4. エスケープ判定の保守的な実装 (疑わしい場合は非展開)
5. `--opt-level 2` でのみ有効

## 参照

- `docs/process/roadmap-v4.md` §5.2 item 6 および §10 item 7
