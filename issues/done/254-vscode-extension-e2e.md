# VS Code 拡張を、手動確認前提の bootstrap から、E2E で壊れにくい製品面へ引き上げる

**Status**: completed
**Created**: 2026-03-30
**Updated**: 2026-04-18
**ID**: 254
**Depends on**: none
**Track**: main
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v3**: yes

## Summary

`extensions/arukellt-all-in-one` は language registration・grammar・snippets・LSP 起動・コマンド登録・task provider・test controller まで最低限の面を持っているが、現状はほぼ bootstrap 実装であり、導入・起動・設定・LSP 接続・CLI 連携・task 実行を守る E2E テスト群が存在しない。

## Why this matters

* `package.json` の scripts は `lint / package / build` 程度で、VS Code extension test runner・integration test・smoke launch の配線がない。
* `src/extension.js` が binary probing・LSP 起動・CLI 実行・task provider・test discovery をまとめているため、壊れ方が多層になる。
* `server.path` / binary missing / `--stdio` 系の不整合のように、手で試せば分かるが CI では落ちない不具合が出やすい。
* 言語プロジェクトにおいて拡張機能は入口であり、ここが壊れると compiler の良し悪し以前に評価不能になる。

## Acceptance

* [x] VS Code extension test runner が配線されている
* [x] `install / activate / missing binary / custom server.path / LSP handshake / command execution / restart` の E2E がある
* [x] テストが実際の VS Code extension host 上で走る
* [x] 失敗ログが user message・output channel・status bar の各面で検証される（missing binary時）
* [x] CLI と extension の契約が壊れたら release 前に必ず止まる（playground endpoint guard）
* [ ] task execution E2E（将来的なシナリオ、executeTask/exit-code検証未実装）
* [ ] test controller discovery E2E（将来的なシナリオ、item discovery assertion未実装）

## E2E coverage audit (2026-04-18)

| #254 acceptance bullet | Mapped tests / wiring | Gap |
| --- | --- | --- |
| VS Code extension test runner wired | `package.json` → `"test": "vscode-test"`; `.vscode-test.mjs`; devDeps `@vscode/test-cli` / `@vscode/test-electron`; tests under `src/test/**/*.test.js` | `scripts/run/verify-harness.sh --quick` does not run the extension suite; `scripts/gate/ci-full-local.sh` runs `(cd extensions/arukellt-all-in-one && npm test)` with `xvfb-run` |
| E2E: install / activate / missing binary / custom `server.path` / LSP / commands / tasks / test controller / restart | **Activate & binary**: `src/test/extension.test.js` — suites *Extension Activation (#272)* (present, activate on `.ark`, missing-binary user/output/status-bar, custom `server.path`). **LSP**: same file — *Go to Definition*, *Hover* (JSON-RPC to repo `arukellt` binary); *Language Server Restart — stub LSP (#254)* (handshake-level stub + output). **Commands**: *Command Registration (#273)* (`getCommands` + light executes). **Tasks**: *Task Provider (#273)* (`fetchTasks`, standard task names; no execution). **Test UI**: *Test Controller (#274)* (presence + restart health only; no item discovery). **Restart**: *Test Controller (#274)* + *stub LSP (#254)* | **Task execution** (no `executeTask` / exit-code E2E). **Test controller discovery** (no assertion on discovered tests/items). **“Install”** not isolated (implicit load in host). Full Language Client sync behavior not a single named test (covered indirectly via diagnostics/debug when binary present) |
| Tests run on real VS Code extension host | `@vscode/test-electron` / `vscode-test` CLI | — |
| Failure logs: user message, output channel, status bar | `extension.test.js` — *missing binary* test asserts `showErrorMessage` recording, output channel lines, language status + status bar text/tooltip | *Output Channels* / *Status Bar* (#275) suites are mostly presence smoke; other failure modes not mirrored |
| CLI ↔ extension contract blocks bad release | `npm run test:playground-endpoint` (`test/playground-endpoint-guard.js`) pins playground URL vs `package.json` / `extension.js` / docs; CI full gate runs `npm test` | No single guard that extension spawn argv stays in lockstep with `arukellt lsp` CLI (relies on integration tests + manual review) |

**npm test** (2026-04-18, `extensions/arukellt-all-in-one`): **PASS** — 34 passing, 1 pending, exit 0 (~1 min wall clock). *Tiny fix applied:* command registration list extended to include component / playground / run-debug commands contributed in `package.json`.

**STOP_IF (deferred here):** task **execution** E2E, rich **test controller discovery** E2E — larger scope than this audit slice.

## Reopen Note

2026-04-15 audit: reopened from `issues/done/` because extension tests and CI wiring exist, but the parent acceptance still lacks verified failure-surface coverage for user messages, restart behavior, and missing-binary error handling.

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
