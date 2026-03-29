# Partial execution preview + local semantic insight

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 195
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

関数 / 式単位の partial execution preview、hover / code lens 上での入力例・推論結果・sandbox 実行結果提示など、ローカル理解を助ける semantic insight surface を追う。

## Acceptance

- [ ] 関数 / 式単位 preview の責務が追跡できる
- [ ] hover / code lens での local semantic insight 導線が定義されている
- [ ] 推論と sandbox 実行の境界を issue queue 上で追跡できる

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `crates/ark-lsp/src/lib.rs`
- `docs/current-state.md`
