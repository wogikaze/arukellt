---
Status: done
Created: 2026-03-30
Updated: 2026-06-13
ID: 216
Track: parallel
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
---
# Formatter surface

## Summary

Shared selfhost formatter (`src/compiler/fmt/`) now backs CLI `arukellt fmt`,
LSP document/range formatting, VS Code default formatter settings, and the
playground Format action.

## Acceptance

- [x] `arukellt fmt` または LSP `textDocument/formatting` で安定した整形が動作する
- [x] VS Code で format on save / format selection が使える
- [x] formatter と compiler の surface 整合性（canonical 表現）が文書化されている

## Close evidence — 2026-06-13

- Shared API: `src/compiler/fmt/mod.ark` (`format_source`, `sort_imports`, `format_range`)
- CLI: `src/compiler/main/fmt.ark` with `--check`
- LSP: `textDocument/formatting` + `rangeFormatting` in `src/compiler/lsp/formatting.ark`
- Organize imports uses `fmt::sorted_import_block` only (#346)
- VS Code: `configurationDefaults["[arukellt]"].editor.defaultFormatter`
- Playground: `formatWithCompilerWasmSync` delegates to compiler `fmt`
- Goldens: `tests/fixtures/fmt/*.expected` + `python3 scripts/manager.py selfhost fmt-parity`
- LSP smoke: `tests/fixtures/selfhost/lsp_formatting.lsp-script`
- Docs: `docs/language/formatter.md`, updated `docs/tooling-feature-matrix.md`

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/done/343-fmt-comment-trivia-preservation.md` … `347-fmt-range-formatting.md`
