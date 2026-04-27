---
Status: done
Created: 2026-04-03
Updated: 2026-04-03
ID: 467
Track: playground
Depends on: 466
Orchestration class: implementation-ready
---
# Playground: docs route が real entrypoint に配線される
**Blocks v1 exit**: no
**Priority**: 3

## Closed by decomposition audit — 2026-04-03

**Evidence**: docs/_sidebar.md: '▶ Try Playground' links to playground/index.html (repo-produced path); no external URL

## Summary

repo 内 docs / site navigation から actual playground entrypoint へ到達できるようにする。entrypoint 自体の存在とは別 issue とし、link / route wiring だけを扱う。

## Visibility

user-visible

## Why this is a separate issue

route/link wiring を entrypoint existence や deploy と混ぜると、リンクだけ先に生えて false-done になる。

## Primary paths

- `docs/index.html`
- `docs/playground/README.md`
- docs navigation / sidebar source files
- entrypoint path produced by issue 466

## Allowed adjacent paths

- repo-controlled built output path for playground page

## Non-goals

- browser entrypoint の実装
- Pages workflow の変更
- extension exposure
- unsupported feature claim の修正全般

## Acceptance criteria

- [x] repo-visible docs route または navigation link が、issue 466 で作られた actual playground entrypoint path を指している。
- [x] link target は repo-produced site 内の path であり、repo 外 URL ではない。
- [x] route / link text は current repo evidence を超える capability claim を新たに追加していない。

## Required verification

- docs navigation / route source を読む
- target path が repo 内に存在することを確認する
- docs を触る場合は `python3 scripts/check/check-docs-consistency.py` を実行する

## Close gate

- repo 内の現物ファイルが列挙されている
- user-visible route / link が repo で確認できる
- link target が actual entrypoint path と一致している
- docs / route だけで product completion を claim しない

## Evidence to cite when closing

- route / navigation file
- target entrypoint file or output path
- docs consistency check result

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No**

## False-done risk if merged incorrectly

medium-high — link to nowhere or external site でも一見完成に見えるため。