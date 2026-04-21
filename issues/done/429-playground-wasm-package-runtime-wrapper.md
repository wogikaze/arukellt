
## Closed

- **Date**: 2026-04-22
- **Commit**: TBD (to be filled after commit)
- **Branch**: feat/429-wasm-package-wrapper
- **Close note**: JS/TS wrapper verified to correctly load `docs/playground/wasm/ark_playground_wasm_bg.wasm`
  (568 KB, committed in #379). The wrapper in `playground/src/worker.ts` derives the JS glue URL from
  the wasm URL and calls `mod.default(wasmUrl)` to initialise the module, then exposes `parse`,
  `format`, `tokenize`, `typecheck`, and `version` APIs. Main-thread path in `playground/src/playground.ts`
  does the same. The compiled TypeScript output (`docs/playground/dist/`) was missing — `npm run build:app`
  built and copied it (57 files). All 171 JS unit tests pass. The `index.html` correctly imports from
  `./dist/playground-app.js` and loads wasm from `./wasm/ark_playground_wasm_bg.wasm`.

# Playground: parser / formatter / diagnostics の Wasm package と JS wrapper を作る

> **Status:** done
> **Track:** playground
> **Type:** Implementation

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

**Created**: 2026-03-31
**Updated**: 2026-04-22
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
