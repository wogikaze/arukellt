---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 342
Track: lsp-semantic
Depends on: 338
Orchestration class: implementation-ready
---
# LSP: completion をコンテキスト対応にする
**Blocks v1 exit**: no
**Priority**: 8

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: server.rs:1057 handles use-context completions, test at line 6264 verifies

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/342-lsp-completion-context.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

completion を「位置無関係の全候補フラット表示」から、カーソル位置の構文コンテキストに応じたフィルタリング・ランキングに改善する。dot completion (method / field)、pattern context での enum variant 提案、type annotation context での型名提案を実装する。

## Current state

- `crates/ark-lsp/src/server.rs:244-484` (`get_completions()`): prefix 文字列一致で全 builtin + keyword + module を返す
- dot completion なし: `x.` の後でメソッド / field を提案しない
- pattern context での enum variant 提案なし
- 型注釈位置での型名優先表示なし

## Acceptance

- [x] `.` の後でレシーバ型に応じた method / field が提案される
- [x] `match` arm の pattern 位置で enum variant が提案される
- [x] 型注釈位置 (`:` の後) で型名が優先表示される
- [x] `use` 文の後で import 可能な module / symbol が提案される

## References

- `crates/ark-lsp/src/server.rs:244-484` — `get_completions()` hardcoded 一覧
- `crates/ark-lsp/src/server.rs:185-198` — `completion_prefix()` テキスト抽出