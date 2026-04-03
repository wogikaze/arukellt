# Extension: playground surface は repo で証明できる endpoint だけを指す

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 469
**Depends on**: 466, 468
**Track**: extension
**Blocks v1 exit**: no
**Priority**: 5

## Summary

VS Code extension の playground command / config / README が、current repo で route / build / publish を証明できる endpoint だけを expose するようにする。repo 外 URL を sole proof にする構成を禁止する。

## Visibility

user-visible

## Why this is a separate issue

extension exposure は product wiring より後段である。ここを実装・deploy と混ぜると、リンクだけ先に出して false-done になる。

## Primary paths

- `extensions/arukellt-all-in-one/package.json`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/README.md`
- route / publish proof from issues 466 and 468

## Allowed adjacent paths

- repo docs that state the canonical playground route

## Non-goals

- playground product code の実装
- docs shell route wiring
- deploy workflow の追加

## Acceptance criteria

- [x] extension command / config が指す playground endpoint は、issues 466 と 468 の repo proof から辿れる path だけである。
- [x] README の description は actual repo-proved endpoint behavior と一致する。
- [x] repo proof がない endpoint を default value や user-visible command で expose しない。

## Required verification

- extension command / setting value を grep する
- value を route / build / publish proof と突き合わせる
- README text を current behavior と比較する

## Close gate

- repo 内の現物ファイルが列挙されている
- user-visible command / setting が repo proof と一致している
- repo 外 URL を sole basis とした close を禁止する
- extension exposure だけで product availability を claim しない

## Evidence to cite when closing

- `extensions/arukellt-all-in-one/package.json`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/README.md`
- prerequisite route / build / publish proof files

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No**

## False-done risk if merged incorrectly

high — stale external URL を開くだけで feature shipped に見えてしまう。
