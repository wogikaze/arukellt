---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 335
Track: lsp-navigation
Depends on: 333
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 4
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: LSP server has cross-file goto_definition via symbol_index, lsp_e2e.rs tests present
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

# LSP: cross-file go to definition を実装する
- `crates/ark-lsp/src/server.rs: "3037-3080` — `goto_type_definition()` 同一ファイル探索"
- `find_definition_span()` (506-546): top-level item → impl block method → let binding → param の順で同一 module 内を探索
- qualified name (`module: ":item`) の解決なし"
- [x] qualified name (`module: ":fn()`) の定義元に飛べる"
# LSP: cross-file go to definition を実装する

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/335-lsp-cross-file-goto-definition.md` — incorrect directory for an open issue.


## Summary

`goto_definition` を single-file AST walk から project-wide symbol index 検索に切り替える。現在 `find_definition_span()` は current file の top-level item / let / param のみを探し、`Location.uri` も常に現在ファイルに固定されている。

## Current state

- `crates/ark-lsp/src/server.rs:2232-2269`: `goto_definition()` が current file のみ走査
- `find_definition_span()` (506-546): top-level item → impl block method → let binding → param の順で同一 module 内を探索
- 他ファイルで定義された関数・型に対して definition が返らない
- qualified name (`module::item`) の解決なし

## Acceptance

- [x] 別ファイルで定義された関数に go to definition で飛べる
- [x] `use` 文で import した名前から定義元ファイルに飛べる
- [x] qualified name (`module::fn()`) の定義元に飛べる
- [x] `goto_type_definition` も cross-file で動作する

## References

- `crates/ark-lsp/src/server.rs:2232-2269` — `goto_definition()` 実装
- `crates/ark-lsp/src/server.rs:506-546` — `find_definition_span()` 同一ファイル探索
- `crates/ark-lsp/src/server.rs:3037-3080` — `goto_type_definition()` 同一ファイル探索