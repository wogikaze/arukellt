# ADR-004 P4: メソッド構文 / trait 再評価

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 157
**Depends on**: —
**Track**: language-design
**Blocks v1 exit**: no

**Status note**: roadmap-v4 では v4 後半で評価するが、最適化パス安定化までは着手しない。

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/157-adr004-method-syntax-evaluation.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/process/roadmap-v4.md` §6 item 9 および §12 item 1 は、ADR-004 P4 として
メソッド構文 / trait 解禁の是非を v4 後半で再評価することを求めている。
現状の issue queue には、この判断自体を追跡する専用 issue がない。
採用を前提に実装を急ぐのではなく、開始条件・評価対象・出口条件を固定して判断のブレを防ぐ。

## 受け入れ条件

1. 開始条件として「v4 最適化パスが stable であること」を明文化する
2. 評価対象を `.map()`, `.filter()`, `.len()`, `.push()` などの最小セットに限定して比較できる
3. parser / resolve / typecheck / docs / migration への影響範囲を洗い出す
4. 結論が「採用」「延期」「不採用」のいずれでも ADR-004 補遺または同等の記録に残せる

## 実装タスク

1. `docs/adr/ADR-004-trait-strategy.md` と既存 syntax/current-state 記述を棚卸しする
2. trait なしの関数呼び出し中心設計と、最小メソッド構文導入案の差分を整理する
3. `.method()` 解禁で必要になる構文・名前解決・型推論・stdlib surface の変更点を列挙する
4. 最終判断を記録する文書位置（ADR 補遺 / roadmap 更新 / migration guide）を固定する

## 参照

- `docs/process/roadmap-v4.md` §6 item 9
- `docs/process/roadmap-v4.md` §12 item 1
- `docs/adr/ADR-004-trait-strategy.md`
- `issues/done/001-v0-syntax-canonical-surface.md`
