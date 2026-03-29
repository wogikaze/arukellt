# VS Code: `arukellt-all-in-one` 拡張機能 epic

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 183
**Depends on**: 184, 185, 186, 187, 188
**Track**: parallel
**Blocks v1 exit**: no

**Status note**: IDE / DX track. 言語側未実装の機能も含めて「全部入り」拡張を定義し、足りない前提機能は child issue で追跡する。

## Summary

`arukellt-all-in-one` は、Arukellt 向けの VS Code 体験を 1 つの拡張に集約する。
最低限の LSP 連携に留めず、syntax / semantic highlight、hover、completion、definition / references、workspace navigation、format、tasks、test explorer、debug、`ark.toml` scripts、component / target-aware commands まで含む batteries-included な導線を目標にする。

現状でも `arukellt lsp` と `ark-lsp` は存在するが、rename / code actions / workspace symbols / formatting / test runner / debug / `ark.toml` scripts は未整備または未定義である。これらを本 issue 配下で分解し、拡張だけ先行実装して機能が空振りする状態を避ける。

## 受け入れ条件

1. #184, #185, #186, #187, #188 が完了している
2. `.ark` を開いた直後に、language configuration・syntax highlight・semantic tokens・diagnostics・hover・completion・definition / references が 1 つの拡張で利用できる
3. VS Code 上から `check` / `compile` / `run` / `test` / `debug` / `script run` の主要導線が辿れる
4. 未導入の外部依存 (`wasm-tools` など) や未対応 target / feature は、拡張が黙って失敗せず setup guidance を出す
5. 拡張 README / docs が、Arukellt の current behavior とズレずにセットアップ手順・制約・対応 target を説明している

## 実装タスク

1. child issue ごとに拡張側 / 言語側の責務を分離する
2. VS Code 拡張の最小土台を #184 で整える
3. 編集体験の不足分を #185 で補う
4. テスト体験を #186 で、デバッグ体験を #187 で、プロジェクト / scripts 体験を #188 で整える
5. docs とセットアップ手順を user-facing に一本化する

## 参照

- `crates/ark-lsp/src/lib.rs`
- `crates/arukellt/src/main.rs`
- `crates/arukellt/src/commands.rs`
- `docs/current-state.md`
- `issues/open/124-wit-component-import-syntax.md`
