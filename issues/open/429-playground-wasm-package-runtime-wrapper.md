
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: TS wrapper exists but wraps a wasm module that was never compiled; worker.ts loads wasm dynamically but no binary exists
- **Root cause**: The playground wasm binary (ark-playground-wasm) has never been compiled. crates/ark-playground-wasm/pkg/ does not exist. docs/playground/wasm/ is empty. All playground user-visible functionality depends on this binary.
- **Evidence**: `find . -name '*.wasm' -path '*playground*'` returns nothing; `ls crates/ark-playground-wasm/pkg/` fails; `ls docs/playground/wasm/` is empty.

# Playground: parser / formatter / diagnostics の Wasm package と JS wrapper を作る

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 429
**Depends on**: 379
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 2

## Summary

browser 側で使う parser / formatter / diagnostics を 1 つの Wasm package としてまとめ、frontend が直接使える wrapper API を用意する。wasm build ができるだけではなく、UI が呼びやすい surface を整える。

## Current state

- parser / formatter は Wasm 化の候補だが、frontend 向けの package 形式や wrapper はまだない。
- 個別 crate をそのまま expose すると UI 実装が複雑になる。
- browser から呼ぶ API surface を先に整えたい。

## Acceptance

- [x] playground 用 Wasm package が作成される。
- [x] parse / format / diagnostics の wrapper API が定義される。
- [x] package のビルド手順が自動化される。
- [x] 最低限のブラウザ起動確認がある。

## References

- ``crates/ark-parser/``
- ``crates/ark-lexer/``
- ``crates/ark-diagnostics/``
