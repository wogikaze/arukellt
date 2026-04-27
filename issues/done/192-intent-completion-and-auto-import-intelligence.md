---
Status: done
Created: 2026-03-29
Updated: 2026-04-03
ID: 192
Track: parallel
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: LSP completion with context implemented in server.rs, tracking issues defined
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---
# Intent completion + auto import intelligence

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/192-intent-completion-and-auto-import-intelligence.md` — incorrect directory for an open issue.


## Summary

prefix 一致中心の補完を超え、期待型・周辺 AST・未 import symbol を用いた intent-aware completion と auto import intelligence を整備する。authoring surface の知能面を担当する child issue。

## Acceptance

- [x] expected-type や周辺文脈を使う補完責務が追跡できる
- [x] 未 import symbol 候補と auto import 導線が定義されている
- [x] completion ranking / relevance の責務が issue queue 上で追跡できる

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/ark-lsp/src/server.rs`