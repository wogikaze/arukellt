# Playground: type-checker product claim を独立 issue に分離する

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 472
**Depends on**: 466
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 8

## Summary

`type checking available` を parser / diagnostics wording に紛れ込ませず、独立した product claim として追跡する。callable checker surface と entrypoint-level proof がない限り、この claim は done にしない。

## Visibility

user-visible

## Why this is a separate issue

type-checker claim は parser / format / diagnostics とは別の product claim である。混ぜると parser diagnostics だけで false-done になる。

## Primary paths

- actual checker invocation surface to be determined
- browser entrypoint path from issue 466
- `docs/playground/README.md`

## Allowed adjacent paths

- `crates/ark-playground-wasm/**`
- `playground/src/**`
- checker implementation / invocation files in compiler crates if they become the actual source of proof

## Non-goals

- docs-only wording tweak で claim を成立させること
- deploy / extension exposure
- parser diagnostics を checker proof とみなすこと

## Acceptance criteria

- [ ] current repo に callable checker surface が存在し、その source path が issue 本文に明記されている。
- [ ] issue 466 の browser entrypoint から、その checker surface が実際に invoke されることを repo files で確認できる。
- [ ] checker result を機械的に検証する command / test / fixture が repo に存在する。

## Required verification

- checker surface の source path を grep する
- entrypoint から checker invocation を grep する
- command / test / fixture を実行して checker behavior を確認する

## Close gate

- repo 内の現物ファイルが列挙されている
- user-visible entrypoint から checker invocation が確認できる
- parser diagnostics だけを evidence にしない
- docs claim だけで close しない

## Evidence to cite when closing

- checker source file(s)
- entrypoint file(s)
- verification command / test / fixture
- any docs file updated after implementation proof exists

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No**

## False-done risk if merged incorrectly

very high — parse errors や lexer diagnostics を type checking と誤認すると即 false-done になる。
