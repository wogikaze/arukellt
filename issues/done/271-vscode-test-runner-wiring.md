---
Status: done
Created: 2026-03-30
Updated: 2026-04-03
ID: 271
Track: main
Depends on: 254
Orchestration class: implementation-ready
---

# VS Code extension test runner を配線する

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/271-vscode-test-runner-wiring.md` — incorrect directory for an open issue.


## Summary

`extensions/arukellt-all-in-one/package.json` の scripts は `lint / package / build` のみで、VS Code extension test runner が配線されていない。E2E テストを実行できるインフラを先に確立する。

## Acceptance

- [x] `@vscode/test-cli` または `@vscode/test-electron` が devDependency に追加されている
- [x] `package.json` の `test` スクリプトで `vscode-test` が実行される
- [x] CI ジョブ（headless Linux、Xvfb 等）で extension テストが実行される
- [x] テスト実行結果が CI ログに出力される（合否・件数）

## Scope

- `@vscode/test-cli` の導入と設定ファイルの作成（`.vscode-test.mjs` 等）
- `package.json` の `scripts.test` を更新
- CI ジョブに `cd extensions/arukellt-all-in-one && npm test` を追加
- headless 実行のための Xvfb 設定

## References

- `extensions/arukellt-all-in-one/package.json`
- `issues/open/254-vscode-extension-e2e.md`