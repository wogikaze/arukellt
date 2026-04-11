# WasmGC Post-MVP プレビュー: 将来拡張の設計調査

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 120
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/120-wasm-gc-post-mvp-preview.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/spec/spec-3.0.0/proposals/gc/Post-MVP.md` に記載された
WasmGC の将来拡張 (static fields, weak references, finalization, 型パラメータ等) を調査し、
Arukellt v5 以降での活用可能性を評価する設計ドキュメントを作成する。
v4 での実装は行わないが、v5 の設計判断に活用する。

## 受け入れ条件

1. `docs/adr/ADR-008-wasm-gc-post-mvp.md` を作成
2. Post-MVP 機能ごとに「Arukellt での活用可能性」と「実装コスト推定」を記載
3. 特に `static fields` と `weak references` については詳細設計案を記述

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/Post-MVP.md`
- `docs/spec/spec-3.0.0/OVERVIEW.md`
