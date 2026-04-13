# Playground docs: feature claim は current implementation と一致させる

**Status**: done
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 470
**Depends on**: 465
**Track**: docs-audit
**Blocks v1 exit**: no
**Priority**: 6

## Summary

playground の docs claim を current repo implementation に揃える。特に `type checking available`、`browser で使える` などの user-visible claim は、callable surface と entrypoint proof がない限り current-state として書かせない。

## Visibility

internal-only

## Why this is a separate issue

feature-claim correction は route wiring や deploy correction と別 issue でないと、docs の先行が再発する。

## Primary paths

- `docs/playground/README.md`
- `docs/language/README.md`
- `crates/ark-playground-wasm/src/lib.rs`
- `playground/src/**`

## Allowed adjacent paths

- extension README if it repeats the same unsupported claims
- related ADR pages that restate current playground capabilities

## Non-goals

- missing feature の実装
- deploy / workflow の追加
- extension endpoint の修正

## Acceptance criteria

- [x] docs に残る playground capability claim はすべて current repo の callable / mountable surface に紐づいている。
- [x] `type checking available` のような claim は、concrete invocation surface と verification command がない限り current-state から除かれている。
- [x] browser availability claim は issue 466 が満たされるまでは current-state としては書かれていない。

## Required verification

- docs 内の playground capability claim を grep する
- source exports / invocation surfaces と突き合わせる
- docs を触る場合は `python3 scripts/check/check-docs-consistency.py` を実行する

## Close gate

- repo 内の現物ファイルが列挙されている
- surviving claim ごとに source proof がある
- docs claim だけで feature available を作らない
- 「将来やる」を acceptance に使わない

## Evidence to cite when closing

- `docs/playground/README.md`
- `docs/language/README.md`
- source proof files under `crates/ark-playground-wasm/` and `playground/src/`
- docs consistency check result

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No — internal-only docs audit issue**

## False-done risk if merged incorrectly

high — docs が再び product reality を先行すると false-done が即座に再発する。
