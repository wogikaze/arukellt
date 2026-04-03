# Playground: browser entrypoint が repo 内に存在する

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 466
**Depends on**: 465
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 2

## Summary

repo 内でユーザーがどこから playground を開くのかを示す browser entrypoint を作る。`playground/src/**` の部品があること自体はこの issue の完了条件ではなく、repo-visible な mount 済み entrypoint があることのみを扱う。

## Visibility

user-visible

## Why this is a separate issue

user-visible feature の最小証拠は entrypoint であり、parts implementation / docs route / deploy / extension exposure と混ぜると false-done が起きる。

## Primary paths

- `playground/src/playground-app.ts`
- `playground/package.json`
- `docs/index.html`
- entrypoint / route file to be added under repo-controlled web output path

## Allowed adjacent paths

- `playground/src/index.ts`
- `playground/src/worker-client.ts`
- `crates/ark-playground-wasm/**`

## Non-goals

- docs navigation の追加
- Pages deploy workflow の追加
- extension からの公開導線
- type checking claim の実装

## Acceptance criteria

- [ ] repo-visible な browser entrypoint file が存在し、`createPlaygroundApp(...)` または同等の mounted application surface を実際に呼び出している。
- [ ] issue 本文に「ユーザーが開く path / route」が明記され、その path が repo files から確認できる。
- [ ] entrypoint を生成または build する script が `playground/package.json` または同等の repo build config に存在する。
- [ ] entrypoint は placeholder text ではなく、現行 playground implementation surface を mount している。

## Required verification

- entrypoint を生成する repo script を実行する
- entrypoint path / output path が存在することを確認する
- `createPlaygroundApp\(` もしくは同等の mount call を grep する

## Close gate

- repo 内の現物ファイルが列挙されている
- user-visible entrypoint / route が repo で確認できる
- build script が repo に存在する
- 「未配線だが部品はある」を理由に done にしない

## Evidence to cite when closing

- entrypoint source file
- build script 定義
- mount call がある file / line
- output path または route file

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No**

## False-done risk if merged incorrectly

high — placeholder page や mount していない shell だけで done になりうる。
