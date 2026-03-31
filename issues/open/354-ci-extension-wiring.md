# Tooling Contract: VS Code extension テストを CI に接続する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 354
**Depends on**: 353
**Track**: tooling-contract
**Blocks v1 exit**: no
**Priority**: 22

## Summary

`extensions/arukellt-all-in-one` のテスト (35 件) を CI pipeline に接続し、extension の回帰を merge-blocking で検出する。現在 `.vscode-test.mjs` と `extension.test.js` は存在するが CI job がない。

## Current state

- `extensions/arukellt-all-in-one/.vscode-test.mjs`: テスト設定あり
- `extensions/arukellt-all-in-one/src/test/extension.test.js`: 35 テスト
- `.github/workflows/ci.yml`: extension test job なし
- `docs/test-strategy.md`: editor-tooling = "0 automated tests / not started" (outdated)

## Acceptance

- [ ] CI に extension test job が追加される
- [ ] 35 件の既存テストが CI で pass する
- [ ] extension test の失敗が merge-blocking になる
- [ ] `docs/test-strategy.md` の editor-tooling status が更新される

## References

- `extensions/arukellt-all-in-one/.vscode-test.mjs` — テスト設定
- `extensions/arukellt-all-in-one/src/test/extension.test.js` — 35 テスト
- `.github/workflows/ci.yml` — CI 定義
- `docs/test-strategy.md` — テスト戦略
