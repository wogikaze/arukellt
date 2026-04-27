---
Created: 2026-04-03
Updated: 2026-04-03
ID: 468
Track: playground-deploy
Depends on: 466
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Priority: 4
Orchestration class: implementation-ready
---
## Reopened by audit

## Closed by decomposition audit — 2026-04-03

**Evidence**: playground/package.json 'build': 'tsc'; .github/workflows/pages.yml builds playground JS, uploads artifact, deploys to GitHub Pages

## Summary

playground の build script、output path、publish path を repo 内の現物で証明できる状態にする。deploy / workflow / output proof のみを扱い、docs claim や extension exposure は別 issue とする。

## Visibility

internal-only

## Why this is a separate issue

deploy proof は product entrypoint と別層であり、future deploy plan を acceptance に混ぜると false-done になる。

## Primary paths

- `playground/package.json`
- build config files under `playground/`
- `.github/workflows/pages.yml`
- publish / output path under `docs/` or equivalent deploy artifact directory

## Allowed adjacent paths

- `docs/playground/deployment-strategy.md`

## Non-goals

- extension exposure
- docs route wiring
- preview deployment promise の実装なし宣言を current-state に昇格させること

## Acceptance criteria

- [x] repo には playground publishable output を生成する build script が存在する。
- [x] GitHub Pages または同等の publish claim を残す場合、その workflow file が actual output path を参照している。
- [x] docs 上の preview deploy / hashed asset / publish path claim は、対応する workflow / build files が repo にあるものだけに限定されている。

## Required verification

- declared build script を実行する
- workflow file の upload / artifact / publish path を読む
- build 後に output path が存在することを確認する

## Close gate

- repo 内の現物ファイルが列挙されている
- workflow / build / output path の整合が取れている
- docs 上の deploy claim が workflow proof を超えていない
- 「運用想定がある」を理由に done にしない

## Evidence to cite when closing

- `playground/package.json`
- build config file(s)
- `.github/workflows/pages.yml`
- built output path listing

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No — internal-only deploy proof issue**

## False-done risk if merged incorrectly

high — prose-only deployment strategy が shipped infrastructure に見えてしまう。

---

## Closed -- 2026-04-25

Evidence: wasm compiled and publish path proven in commit 1be20b32.
- Build: cd playground && npm run build:wasm -- exit 0
- Artifact: crates/ark-playground-wasm/pkg/ark_playground_wasm_bg.wasm (568248 bytes)
- App build: npm run build:app -- exit 0; output in docs/playground/wasm/ and docs/playground/dist/
- Deploy: .github/workflows/pages.yml uses upload-pages-artifact@v3 with path ./docs
- Proof record: docs/playground/build-path-proof.md in HEAD

Close gate met: concrete files enumerated; workflow/build/output alignment confirmed.