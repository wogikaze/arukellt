# VS Code 拡張を、手動確認前提の bootstrap から、E2E で壊れにくい製品面へ引き上げる

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 254
**Depends on**: none
**Track**: main
**Blocks v1 exit**: yes

## Summary

`extensions/arukellt-all-in-one` は language registration・grammar・snippets・LSP 起動・コマンド登録・task provider・test controller まで最低限の面を持っているが、現状はほぼ bootstrap 実装であり、導入・起動・設定・LSP 接続・CLI 連携・task 実行を守る E2E テスト群が存在しない。

## Why this matters

* `package.json` の scripts は `lint / package / build` 程度で、VS Code extension test runner・integration test・smoke launch の配線がない。
* `src/extension.js` が binary probing・LSP 起動・CLI 実行・task provider・test discovery をまとめているため、壊れ方が多層になる。
* `server.path` / binary missing / `--stdio` 系の不整合のように、手で試せば分かるが CI では落ちない不具合が出やすい。
* 言語プロジェクトにおいて拡張機能は入口であり、ここが壊れると compiler の良し悪し以前に評価不能になる。

## Acceptance

* [ ] VS Code extension test runner が配線されている
* [ ] `install / activate / missing binary / custom server.path / LSP handshake / command execution / task execution / test controller discovery / restart` の E2E がある
* [ ] テストが実際の VS Code extension host 上で走る
* [ ] 失敗ログが user message・output channel・status bar の各面で検証される
* [ ] CLI と extension の契約が壊れたら release 前に必ず止まる

## Scope

### extension test runner の配線（→ 271）

* `@vscode/test-electron` または `@vscode/test-cli` の導入と CI 配線

### install/activate/binary discovery E2E（→ 272）

* 起動成功・binary missing 時のエラー表示・`server.path` カスタム設定の E2E

### LSP/command/task E2E（→ 273）

* LSP handshake・コマンド実行・task provider 動作の E2E

### test controller / restart E2E（→ 274）

* test controller discovery・restart コマンドの E2E

### 失敗ログ検証面の確立（→ 275）

* output channel・status bar・user message notification の各面での失敗検証

## References

* `extensions/arukellt-all-in-one/`
* `extensions/arukellt-all-in-one/package.json`
* `extensions/arukellt-all-in-one/src/extension.js`
* `issues/open/271-vscode-test-runner-wiring.md`
* `issues/open/272-extension-install-activate-binary-e2e.md`
* `issues/open/273-extension-lsp-command-task-e2e.md`
* `issues/open/274-extension-test-controller-restart-e2e.md`
* `issues/open/275-extension-failure-log-verification.md`
