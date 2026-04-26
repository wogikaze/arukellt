# Playground false-done 監査表と status rollback

**Status**: done
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 465
**Depends on**: none
**Track**: playground-audit
**Blocks v1 exit**: no
**Priority**: 1

## Summary

playground 周辺の done issue と現行 repo の現物を突き合わせ、A/B/C/D/E/F で分類した監査表を固定する。user-visible claim を repo 内証拠なしに done にできないよう、過大な done claim は open queue に戻し、以後は narrower issue 群で追跡する。

## Visibility

internal-only

## Why this is a separate issue

これは実装 issue ではなく、issue decomposition と status governance の issue である。product wiring / docs 修正 / deploy とは分離しないと、監査そのものが prose-only で done になりうる。

## Audit summary table

| Surface | Current repo evidence | Classification | Why |
|---|---|---|---|
| Playground TS/UI parts | `playground/src/playground-app.ts`, `editor.ts`, `diagnostics.ts`, `share.ts`, `examples.ts` | B | 部品はあるが repo-visible browser entrypoint がない |
| Playground Wasm engine | `crates/ark-playground-wasm/src/lib.rs` exports `parse`, `format`, `tokenize`, `version` | A | internal engine としては repo 証拠がある |
| Browser entrypoint / route | `docs/index.html` is docsify shell only | C / F | mount 済み page / route が repo で確認できない |
| Pages deploy path | `.github/workflows/pages.yml` uploads `./docs` only | E / F | repo は docs を publish しているが playground publish proof はない |
| Playground docs capability claims | rollback landed in generated docs and now marks missing entrypoint / type-checking proof | D mitigated | docs no longer present browser reachability or browser type checking as current repo proof |
| Language docs playground reference | rollback landed in generated docs and now points to implementation work + remaining gaps | D mitigated | user docs no longer present the generated docs site as a live playground route |
| Extension playground exposure | `arukellt.playgroundUrl` now defaults to empty and command errors until configured | E mitigated | repo no longer ships an unproved public playground endpoint by default |
| Docs command/workflow claims | draft docs now mark `npm run dev`, `npm run build:full`, and preview deploys as target-state only | D / F mitigated | current-state docs no longer present missing commands/workflows as repo proof |

## Observed mismatches

- `issues/done/380-playground-editor-ui.md` は browser editor usability を claim しているが、repo-visible entrypoint を要求していない。
- `issues/done/431-playground-editor-shell-highlighting.md` は browser editor 追加を claim しているが、mount 済み surface を要求していない。
- historical broad done issues (`380`, `431`) は historical implementation-parts only の注記なしでは current product proof と誤読されうるため、status note で rollback 済み。
- generated docs source は以前 browser availability / type-checking / live route を current-state のように出力していたため、current repo proof と target-state を分離する wording に rollback 済み。
- extension は以前 unproved external playground URL を default で expose していたため、current repo-backed default ではない設定に rollback 済み。
- deployment / performance draft docs は以前 missing local scripts / preview deploy を current-state のように読める形だったため、target-state only であることを明記済み。

## Primary paths

- `issues/done/380-playground-editor-ui.md`
- `issues/done/431-playground-editor-shell-highlighting.md`
- `issues/done/379-playground-wasm-build.md`
- `issues/done/428-playground-v1-contract-adr.md`
- `issues/done/435-playground-unsupported-capability-ux.md`
- `issues/done/438-playground-privacy-telemetry-error-reporting.md`
- `issues/open/`
- `issues/open/index.md`
- `issues/open/dependency-graph.md`

## Allowed adjacent paths

- `playground/**`
- `crates/ark-playground-wasm/**`
- `docs/playground/**`
- `docs/language/README.md`
- `docs/index.html`
- `extensions/arukellt-all-in-one/**`
- `.github/workflows/pages.yml`

## Non-goals

- playground product code の実装
- actual browser entrypoint / route / deploy proof の実装
- deploy infrastructure の追加
- type checker の実装

## Acceptance criteria

- [x] A/B/C/D/E/F 監査表が本 issue 内に固定され、各行に repo file evidence が書かれている。
- [x] false-done risk が高い playground surface について、1 issue = 1 product claim の narrower open issue 群が `issues/open/` に作成されている。
- [x] `issues/done/380` と `issues/done/431` に historical implementation-parts only の注記が入り、broad user-visible claim をそのまま future proof に使わず、replacement open issues からのみ追跡できる状態になっている。
- [x] `bash scripts/gen/generate-issue-index.sh` 実行後、`issues/open/index.md` と `issues/open/dependency-graph.md` に本監査 initiative が反映されている。

## Required verification

- `bash scripts/gen/generate-issue-index.sh`
- 生成後の `issues/open/index.md` を読む
- 生成後の `issues/open/dependency-graph.md` を読む
- 新規 open issue 各ファイルの metadata (`Status`, `ID`, `Depends on`, `Track`) を確認する

## Close gate

- repo 内の現物ファイルが issue 本文に列挙されている
- audited surface ごとに classification が明記されている
- replacement open issues が `issues/open/` に存在する
- generated index / graph が更新されている
- 「部品はある」を理由に user-visible surface を done 扱いしない

## Evidence to cite when closing

- `issues/open/465-playground-false-done-audit-and-status-rollback.md`
- `issues/done/380-playground-editor-ui.md`
- `issues/done/431-playground-editor-shell-highlighting.md`
- `docs/playground/README.md`
- `docs/language/README.md`
- `docs/README.md`
- `docs/playground/deployment-strategy.md`
- `docs/playground/diagnostics-worker-performance-budget.md`
- `extensions/arukellt-all-in-one/package.json`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/README.md`
- `issues/open/index.md`
- `issues/open/dependency-graph.md`
- `issues/open/466-playground-browser-entrypoint-exists.md`
- `issues/open/467-playground-docs-route-wired-to-real-entrypoint.md`
- `issues/open/468-playground-build-and-publish-path-proof.md`
- `issues/open/469-extension-playground-surface-points-to-repo-proved-endpoint.md`
- `issues/open/470-playground-feature-claims-match-implementation.md`
- `issues/open/471-playground-docs-command-workflow-reality-audit.md`
- `issues/open/472-playground-type-checker-product-claim.md`

## False-done prevention checks

- Can this be closed with only parts existing? **No**
- Can docs get ahead and still allow close? **No**
- Can extension expose a link and still allow close without route proof? **No**
- Can deploy be claimed without workflow/output proof? **No**
- Does this rely on a repo-external URL as proof? **No**
- Can it be closed without concrete evidence files? **No**
- Does this contain a user-visible claim without entrypoint acceptance? **No**

## False-done risk if merged incorrectly

high — 監査 issue 自体が曖昧だと、その後の narrow issue 群も根拠なく done になりうる。
