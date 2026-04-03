# Extension: initializationOptions で全 5 設定を LSP サーバーに渡す

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 478
**Depends on**: 477
**Track**: extension
**Blocks v1 exit**: no

---

## Decomposed from 462

Issue 462 (`extension-settings-rationalization`) の **extension wiring layer** を担当する。
`package.json` の設定宣言が完成した (#477) 後に、`extension.js` が
全 5 設定を `initializationOptions` に含めて LSP サーバーへ渡す変更を行う。

Upstream: #477 (manifest宣言) — 完了後に着手  
Downstream: #479 (LSP server LspConfig) — この issue 完了後に着手

---

## Summary

`extension.js` の LSP クライアント初期化コードで、
新規 5 設定 (enableCodeLens, hoverDetailLevel, diagnostics.reportLevel,
useSelfHostBackend, check.onSave) を `initializationOptions` に追加する。
また `workspace/didChangeConfiguration` ハンドラで設定変更を LSP サーバーに通知する。

**LSP サーバー側の実装変更は含まない** (それは #479)。

## Why this is a separate issue

extension.js の変更は「VS Code から LSP サーバーへ値が届く」層だけを担当する。
LSP サーバー側の変更 (#479) と分離することで:
- extension.js が渡すだけで LSP サーバーが無視している状態を単独で確認できる
- LSP 実装前に extension wiring だけ先行して PR を出せる
- 「extension が設定を渡している」という主張を「LSP が設定を使って動作する」と混同しない

## Visibility

internal-only (LSP サーバーが値を受け取れるようになるが、動作変化はまだない)

## Primary paths

- `extensions/arukellt-all-in-one/src/extension.js` — languageClient 初期化部分

## Allowed adjacent paths

- なし

## Non-goals

- `package.json` への設定追加 (#477)
- LSP サーバー側の `LspConfig` 構造体 (#479)
- LSP ハンドラでの設定値反映 (#479)
- README 更新 (#480)
- `arukellt.check.onSave` の実際の on-save チェック挙動

## Acceptance

1. `extension.js` の `initializationOptions` 構築部分に以下の 5 キーが含まれている:
   `enableCodeLens`, `hoverDetailLevel`, `useSelfHostBackend`,
   `diagnosticsReportLevel` (または `diagnostics.reportLevel`), `checkOnSave`
2. `workspace/didChangeConfiguration` ハンドラが実装されており、
   設定変更時に LSP サーバーへ通知が送られる
3. `extension.js` への変更は #477 で追加した 5 設定の値だけを `initializationOptions` に追加し、
   既存の設定 (target, server.path, server.args 等) は変更しない

## Required verification

- `grep -E "enableCodeLens|hoverDetailLevel|useSelfHost|diagnosticsReport|checkOnSave" extensions/arukellt-all-in-one/src/extension.js` が 5 件以上ヒットする
- `grep "didChangeConfiguration" extensions/arukellt-all-in-one/src/extension.js` が 1 件以上ヒットする
- `npm test` (extension unit tests) が pass する

## Close gate

- `extension.js` の initializationOptions 構築箇所に 5 設定が含まれている (grep で確認)
- `didChangeConfiguration` ハンドラが存在している (grep で確認)
- LSP サーバーが設定を無視していても、この issue は close できる
  (LSP の挙動変化は #479 が担当)
- **LSP サーバー側のコード変更がないこと** (`git diff` が `extension.js` のみ)

## Evidence to cite when closing

- `extensions/arukellt-all-in-one/src/extension.js` の initializationOptions 構築行 (行番号)
- `didChangeConfiguration` ハンドラの行番号

## False-done risk if merged incorrectly

- 「extension が設定を渡している」だけで「設定が動作に反映される」と誤解される
  → Visibility を internal-only にし、LSP 実装は #479 と明記することで防止
- LSP サーバーのコードを変更してこの issue を close する
  → Close gate に「LSP サーバー変更なし」を明記
