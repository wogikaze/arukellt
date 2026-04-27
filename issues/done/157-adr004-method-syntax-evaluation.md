---
Status: done
Created: 2026-03-29
Updated: 2026-04-15
ID: 157
Track: language-design
Depends on: —
Orchestration class: design-ready
Orchestration upstream: —
---

# ADR-004 P4: メソッド構文 / trait 再評価
**Blocks v1 exit**: no

**Status note**: ADR-004-P4 evaluation decision formalized. Trigger conditions defined with measurable criteria. Evaluation deferred pending trigger (v4 opt passes not yet stable as of 2026-04-15).

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

1. [x] 開始条件として「v4 最適化パスが stable であること」を明文化する
2. [x] 評価対象を `.map()`, `.filter()`, `.len()`, `.push()` などの最小セットに限定して比較できる
3. [x] parser / resolve / typecheck / docs / migration への影響範囲を洗い出す
4. [x] 結論が「採用」「延期」「不採用」のいずれでも ADR-004 補遺または同等の記録に残せる

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

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
