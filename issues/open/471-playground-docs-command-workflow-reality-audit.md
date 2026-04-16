# Playground docs: command / workflow / publish claim の現実整合監査

**Status**: done
**Created**: 2026-04-03
**Updated**: 2026-04-14
**ID**: 471
**Depends on**: 465
**Track**: docs-audit
**Blocks v1 exit**: no
**Priority**: 7

## Summary

playground docs に書かれている command / script / workflow / publish claim を current repo の現物に揃える。存在しない script や preview deploy を current-state として記述する状態を止める。

## Visibility

internal-only

## Why this is a separate issue

command / workflow reality は feature claim correction と別層であり、分けないと「機能 claim は直したが運用 claim は虚偽」の状態が残る。

## Primary paths

- `docs/playground/deployment-strategy.md`
- `docs/playground/diagnostics-worker-performance-budget.md`
- `playground/package.json`
- `.github/workflows/pages.yml`

## Allowed adjacent paths

- any docs page that mentions playground scripts, preview deploys, Pages paths, or build outputs

## Non-goals

- missing script / workflow の実装
- extension endpoint 修正
- feature claim correction 全般

## Acceptance criteria

- [ ] playground docs に書かれた command / script はすべて current repo に verbatim で存在する。
- [ ] workflow / preview deploy / publish path claim は、current workflow file と output path で証明できるものだけに限定されている。
- [ ] current repo に存在しない script / workflow / preview deploy は、docs から current-state として除かれるか、separate open issue としてのみ参照されている。

## Required verification

- docs から command / workflow 名を grep する
- `playground/package.json` scripts と `.github/workflows/*.yml` を読む
- docs を触る場合は `python3 scripts/check/check-docs-consistency.py` を実行する

## Close gate

- repo 内の現物ファイルが列挙されている
- nonexistent script / workflow が current-state docs に残っていない
- future plan は current acceptance と混同されていない
- docs だけで deploy proof を作らない

## Evidence to cite when closing

- `docs/playground/deployment-strategy.md`
- `docs/playground/diagnostics-worker-performance-budget.md`
- `playground/package.json`
- `.github/workflows/pages.yml`
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

medium-high — nonexistent command や preview deploy の記述は future implementation を既成事実化する。
