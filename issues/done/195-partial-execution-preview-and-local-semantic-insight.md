# Partial execution preview + local semantic insight

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-13
**Closed**: 2026-04-18
**ID**: 195
**Depends on**: none
**Track**: parallel
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: No partial execution/sandbox preview implementation found.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

関数 / 式単位の partial execution preview、hover / code lens 上での入力例・推論結果・sandbox 実行結果提示など、ローカル理解を助ける semantic insight surface を追う。

## Acceptance

- [x] 関数 / 式単位 preview の責務が追跡できる
- [x] hover / code lens での local semantic insight 導線が定義されている
- [x] 推論と sandbox 実行の境界を issue queue 上で追跡できる

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `crates/ark-lsp/src/lib.rs`
- `docs/current-state.md`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for partial execution preview and local semantic insight features.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking responsibilities, defining entry points, tracking inference/sandbox boundaries)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for partial execution preview and local semantic insight
- References to related LSP issues provide context

**Acceptance mapping:**
- ✓ Function/expression unit preview responsibilities tracked
- ✓ Hover/code lens local semantic insight entry points defined
- ✓ Inference and sandbox execution boundary tracking established

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of partial execution preview/sandbox execution in LSP/extension is tracked in separate implementation issues
- The audit reopened this because no implementation was found, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future implementation work
