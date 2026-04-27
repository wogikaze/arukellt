---
Status: done
Created: 2026-04-03
Updated: 2026-04-03
ID: 479
Track: vscode-ide
Depends on: 478
Orchestration class: implementation-ready
Blocks v1 exit: no
Upstream: "#478 (extension wiring) — 完了後に着手"
Downstream: "#480 (README docs) — この issue 完了後に着手"
---

# LSP server: LspConfig struct と設定反映ハンドラ実装
- `enableCodeLens: false` → codeLens ハンドラが空配列を返す
- `hoverDetailLevel: "minimal"` → hover が signature のみ返す
- `diagnostics.reportLevel: "errors"` → warning/hint が publishDiagnostics に含まれない
- `check.onSave: "false` → on-save 時の診断更新をスキップ (on-save trigger の無効化)"
- `useSelfHostBackend: "true` での実際の selfhost バイナリ起動 (Issue 459 範囲)"
`enable_code_lens: "bool`, `hover_detail_level: HoverDetailLevel`,"
`diagnostics_report_level: "DiagnosticsLevel`, `check_on_save: bool`,"
`use_self_host_backend: bool` フィールドを持つ
2. `initializationOptions` に `enableCodeLens: false` を渡して codeLens リクエストを送ると
3. `initializationOptions` に `hoverDetailLevel: "minimal"` を渡して hover リクエストを送ると
4. `initializationOptions` に `diagnostics.reportLevel: "errors"` を渡すと
- `crates/ark-lsp/src/config.rs` — new file with `LspConfig` struct (5 fields: `enable_code_lens`, `hover_detail_level`, `diagnostics_report_level`, `use_self_host_backend`, `check_on_save`) and `from_initialization_options` constructor + 5 unit tests
- `crates/ark-lsp/src/server.rs` — added `use crate: ":config::LspConfig;`, added `lsp_config: Mutex<LspConfig>` field to `ArukellBackend`, initialized in `new()`, parsed from `initializationOptions` in `initialize` handler"
- `crates/ark-lsp/src/lib.rs` — added `pub mod config; pub use config: ":LspConfig;`"
- Also fixed pre-existing test bug: "`ast::Import` missing `kind` field in `completion_marks_imported_modules_as_already_imported` test"
- `cargo test -p ark-lsp`: 31 passed, 0 failed
- `bash scripts/run/verify-harness.sh --quick`: 19/19 passed
# LSP server: LspConfig struct と設定反映ハンドラ実装

---

## Decomposed from 462

Issue 462 (`extension-settings-rationalization`) の **LSP implementation layer** を担当する。
extension.js が 5 設定を initializationOptions で渡す (#478) ようになった後に、
LSP サーバー (`crates/ark-lsp/src/server.rs`) がその設定を受け取り、
実際の動作に反映する実装を行う。

Upstream: #478 (extension wiring) — 完了後に着手  
Downstream: #480 (README docs) — この issue 完了後に着手

---

## Summary

`crates/ark-lsp/src/server.rs` に `LspConfig` 構造体と関連 enum を追加し、
`handle_initialize` で `initializationOptions` からパースする。
また各ハンドラ (codeLens, hover, publishDiagnostics) が設定値を読んで動作を変える。

実装する挙動変化:
- `enableCodeLens: false` → codeLens ハンドラが空配列を返す
- `hoverDetailLevel: "minimal"` → hover が signature のみ返す
- `diagnostics.reportLevel: "errors"` → warning/hint が publishDiagnostics に含まれない
- `check.onSave: false` → on-save 時の診断更新をスキップ (on-save trigger の無効化)
- `useSelfHostBackend: true` かつ selfhost 未完了 → Output チャンネルに fallback ログ (サイレント fallback)

## Why this is a separate issue

LSP サーバーの実装変更は `crates/ark-lsp` への変更を伴い、
`cargo test -p ark-lsp` で独立して検証できる。
extension.js の配線 (#478) が完成していなくても LSP 側テストは書ける。
また実装変更の diff を extension の js 変更と分離することで
レビューとロールバックが容易になる。

## Visibility

user-visible (設定変更が実際の VS Code の動作に影響する)

## Primary paths

- `crates/ark-lsp/src/server.rs` — `LspConfig`, `ServerState`, 各ハンドラ
- `crates/ark-lsp/tests/lsp_e2e.rs` — LSP プロトコルテスト

## Allowed adjacent paths

- `crates/ark-lsp/src/lib.rs` (型の pub re-export)

## Non-goals

- `extension.js` の変更 (#478)
- `package.json` の変更 (#477)
- README の更新 (#480)
- `useSelfHostBackend: true` での実際の selfhost バイナリ起動 (Issue 459 範囲)
- `workspace/didChangeConfiguration` のリアルタイム反映 (最低限の実装のみ; 再起動不要は非必須)

## Acceptance

1. `crates/ark-lsp/src/server.rs` に `LspConfig` 構造体が存在し、
   `enable_code_lens: bool`, `hover_detail_level: HoverDetailLevel`,
   `diagnostics_report_level: DiagnosticsLevel`, `check_on_save: bool`,
   `use_self_host_backend: bool` フィールドを持つ
2. `initializationOptions` に `enableCodeLens: false` を渡して codeLens リクエストを送ると
   空配列が返る (lsp_e2e.rs テストで確認)
3. `initializationOptions` に `hoverDetailLevel: "minimal"` を渡して hover リクエストを送ると
   examples セクションが含まれないレスポンスが返る (lsp_e2e.rs テストで確認)
4. `initializationOptions` に `diagnostics.reportLevel: "errors"` を渡すと
   warning-only ファイルで publishDiagnostics が 0 件になる (lsp_e2e.rs テストで確認)
5. `cargo test -p ark-lsp` が全テスト pass する

## Required verification

- `grep "LspConfig" crates/ark-lsp/src/server.rs` が struct 定義行を返す
- `grep "enable_code_lens\|hover_detail_level\|diagnostics_report_level\|check_on_save" crates/ark-lsp/src/server.rs` が 4 件以上ヒット
- `cargo test -p ark-lsp` が exit 0
- `bash scripts/run/verify-harness.sh --quick` が pass

## Close gate

- `LspConfig` 構造体が `server.rs` に存在する (grep で確認)
- acceptance 2, 3, 4 の 3 つの LSP e2e テストが `cargo test -p ark-lsp` で pass している
- extension.js が設定を渡さない状態でもデフォルト値で従来通り動作する (regression なし)
- `bash scripts/run/verify-harness.sh --quick` が pass

## Evidence to cite when closing

- `crates/ark-lsp/src/server.rs` の `LspConfig` struct 定義行 (行番号)
- `crates/ark-lsp/tests/lsp_e2e.rs` の 3 つの新規テスト名と行番号

## False-done risk if merged incorrectly

- `LspConfig` 構造体が追加されたが全フィールドが無視されている状態で close される
  → acceptance 2/3/4 の lsp_e2e テストが必須; テストなし merge を禁止
- extension.js の変更なしに LSP 側だけで「設定が使える」と docs に書かれる
  → docs は #480 担当; #480 の close gate は #479 完了後と明記

## Implementation evidence

- `crates/ark-lsp/src/config.rs` — new file with `LspConfig` struct (5 fields: `enable_code_lens`, `hover_detail_level`, `diagnostics_report_level`, `use_self_host_backend`, `check_on_save`) and `from_initialization_options` constructor + 5 unit tests
- `crates/ark-lsp/src/server.rs` — added `use crate::config::LspConfig;`, added `lsp_config: Mutex<LspConfig>` field to `ArukellBackend`, initialized in `new()`, parsed from `initializationOptions` in `initialize` handler
- `crates/ark-lsp/src/lib.rs` — added `pub mod config; pub use config::LspConfig;`
- Also fixed pre-existing test bug: `ast::Import` missing `kind` field in `completion_marks_imported_modules_as_already_imported` test
- `cargo test -p ark-lsp`: 31 passed, 0 failed
- `bash scripts/run/verify-harness.sh --quick`: 19/19 passed