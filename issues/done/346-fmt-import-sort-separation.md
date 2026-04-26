# Formatter: import sort を formatter から分離する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-03
**ID**: 346
**Depends on**: 344
**Track**: formatter
**Blocks v1 exit**: no
**Priority**: 16

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: sort_imports() exported at fmt.rs:121, standalone test at 1490

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/346-fmt-import-sort-separation.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`format_module()` 内に埋め込まれた import sort ロジックを分離し、独立した `sort_imports()` 関数にする。これにより LSP の `source.organizeImports` が formatter 全体を呼ばずに import 操作だけを実行できるようになる。

## Current state

- `crates/ark-parser/src/fmt.rs`: `format_module()` が import を stdlib / project / alias の順にソートする副作用を持つ
- import sort が whole-file format と同じ entry point に混在
- `source.organizeImports` (LSP) が `format_source()` を呼ぶため、import 整理だけを意図しても全文が再整形される

## Acceptance

- [x] `sort_imports()` が独立関数として export される
- [x] `format_module()` が `sort_imports()` を呼ぶ (既存動作は維持)
- [x] LSP の `source.organizeImports` が `sort_imports()` のみを呼ぶ
- [x] テストで sort_imports の単独動作を検証する

## References

- `crates/ark-parser/src/fmt.rs` — import sort 埋め込み
- `crates/ark-lsp/src/server.rs` — `source.organizeImports`
