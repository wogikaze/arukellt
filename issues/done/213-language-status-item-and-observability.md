# Language status item + observability surface

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 213
**Depends on**: 190
**Track**: parallel
**Blocks v1 exit**: no

## Summary

language status item の実装、indexing / compiling / testing 中の状態表示、output channel の分類、verbose log toggle、server trace level 切り替えを含む。

現状は単一 output channel にすべてのログが混在し、LSP サーバーの状態が UI に見えない。

## Acceptance

- [ ] language status item が LSP サーバーの現在状態（indexing / ready / error 等）を表示する
- [ ] output channel が用途別に分類されている（LSP / compiler / test / task）
- [ ] verbose log toggle と server trace level 切り替えが設定またはコマンドから行える

## Scope

### Language status item

- LSP LanguageStatusItem の登録
- `indexing…` / `ready` / `error` などの状態遷移
- エラー時のクリックで output channel を開く導線
- compiling / testing 中の busy indicator

### Output channel taxonomy

- `Arukellt Language Server` — LSP server log
- `Arukellt Compiler` — compile / check output
- `Arukellt Tests` — test runner output
- `Arukellt Tasks` — task 実行 output

### Observability controls

- `Arukellt: Toggle Verbose Logging` コマンド
- server trace level 設定（`off` / `messages` / `verbose`）
- crash / restart の可視化と再起動コマンド
- telemetry 方針の明記（README / package.json contributes）

## References

- `issues/open/190-vscode-commands-tasks-and-status-surfaces.md`
- `issues/open/184-vscode-extension-foundation.md`
- `crates/ark-lsp/src/lib.rs`
- `extensions/arukellt-all-in-one/src/`
