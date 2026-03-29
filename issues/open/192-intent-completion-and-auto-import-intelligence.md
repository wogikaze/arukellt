# Intent completion + auto import intelligence

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 192
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

prefix 一致中心の補完を超え、期待型・周辺 AST・未 import symbol を用いた intent-aware completion と auto import intelligence を整備する。authoring surface の知能面を担当する child issue。

## Acceptance

- [ ] expected-type や周辺文脈を使う補完責務が追跡できる
- [ ] 未 import symbol 候補と auto import 導線が定義されている
- [ ] completion ranking / relevance の責務が issue queue 上で追跡できる

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/ark-lsp/src/server.rs`
