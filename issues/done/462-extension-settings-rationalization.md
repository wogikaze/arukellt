# 拡張機能の設定項目の整理と実装への配線

**Status**: open
**Created**: 2026-04-02
**Updated**: 2026-04-02
**ID**: 462
**Depends on**: none
**Track**: extension
**Blocks v1 exit**: no
**Priority**: 3

## Summary

`extensions/arukellt-all-in-one/package.json` には現在 `arukellt.server.path`、`arukellt.server.args`、`arukellt.target`、`arukellt.emit`、`arukellt.playgroundUrl` の 5 設定しかない。CodeLens 有効化・hover verbosity・selfhost backend 優先など、実装が増えるにつれて必要になる設定を先に設計して追加する。設定変更で挙動が実際に切り替わることを確認し、README に反映する。

---

## 追加する設定項目

### `arukellt.enableCodeLens`

```json
"arukellt.enableCodeLens": {
    "type": "boolean",
    "default": true,
    "description": "Show Run / Debug / Test CodeLens above functions in .ark files.",
    "scope": "resource"
}
```

- `false` の場合、LSP の `textDocument/codeLens` ハンドラが空配列を返す。
- LSP サーバー側: `server.rs` の `handle_code_lens()` 冒頭で `initializationOptions` または設定値を確認する。
- 設定変更時: `workspace/didChangeConfiguration` を受けて設定値を更新する。

### `arukellt.hoverDetailLevel`

```json
"arukellt.hoverDetailLevel": {
    "type": "string",
    "enum": ["minimal", "standard", "verbose"],
    "default": "standard",
    "description": "Controls how much information is shown on hover. 'minimal': type signature only. 'standard': type + doc + availability. 'verbose': all including examples and see_also.",
    "scope": "resource"
}
```

- LSP hover の出力量を制御する。Issue 451 と統合する。
- `minimal`: シグネチャ行のみ。
- `standard`（デフォルト）: シグネチャ + doc + availability（現在の想定挙動）。
- `verbose`: standard + examples + see_also + related spans。

### `arukellt.useSelfHostBackend`

```json
"arukellt.useSelfHostBackend": {
    "type": "boolean",
    "default": false,
    "description": "Use the self-hosted (ark-compiled) compiler backend instead of the Rust backend. Requires Stage 2 fixpoint to be achieved (see Issue 459).",
    "scope": "resource"
}
```

- `true` の場合、LSP / コンパイルコマンドが `arukellt-s1.wasm` 経由で実行する。
- Issue 459 完了前は `true` にしても Rust backend にフォールバックし、`arukellt.output` チャンネルに「selfhost backend not available」を出力する。
- 本 issue では設定を追加して読み取り側を実装するのみ。selfhost backend の実際の起動は Issue 459 範囲。

### `arukellt.diagnostics.reportLevel`

```json
"arukellt.diagnostics.reportLevel": {
    "type": "string",
    "enum": ["errors", "warnings", "all"],
    "default": "all",
    "description": "Controls which diagnostics are surfaced in the editor. 'errors': errors only. 'warnings': errors + warnings. 'all': all including hints.",
    "scope": "resource"
}
```

- LSP の `textDocument/publishDiagnostics` でフィルタリングする。
- 偽陽性が多い間は `errors` に下げやすくする。

### `arukellt.check.onSave` (既存ではない場合)

```json
"arukellt.check.onSave": {
    "type": "boolean",
    "default": true,
    "description": "Run arukellt check automatically when a .ark file is saved.",
    "scope": "resource"
}
```

- `false` の場合、`onDidSaveTextDocument` で triggered な check をスキップする。

---

## 詳細実装内容

### Step 1: `package.json` に設定を追加する

上記 5 設定を `"configuration" > "properties"` に追加する。型・デフォルト・description・scope を全て設定する。

### Step 2: LSP initializationOptions を拡張する

LSP サーバーは `InitializeParams.initializationOptions` で設定値を受け取る。現在どのような構造で渡しているかを `extension.js` で確認し、新設定を渡すよう追加する。

```js
// extension.js の languageClient 初期化部分
const initializationOptions = {
    target: config.get('target'),
    enableCodeLens: config.get('enableCodeLens'),
    hoverDetailLevel: config.get('hoverDetailLevel'),
    useSelfHostBackend: config.get('useSelfHostBackend'),
    diagnosticsReportLevel: config.get('diagnostics.reportLevel'),
};
```

### Step 3: LSP サーバー側で設定値を受け取る (`crates/ark-lsp/src/server.rs`)

`ServerState` 構造体に設定フィールドを追加する。

```rust
pub struct LspConfig {
    pub enable_code_lens: bool,
    pub hover_detail_level: HoverDetailLevel,
    pub use_self_host_backend: bool,
    pub diagnostics_report_level: DiagnosticsLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum HoverDetailLevel { Minimal, Standard, Verbose }

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticsLevel { ErrorsOnly, Warnings, All }
```

`handle_initialize` で `initializationOptions` から `LspConfig` をパースする。`workspace/didChangeConfiguration` で更新する。

### Step 4: 設定を実際の挙動に反映する

#### `enableCodeLens = false`

```rust
fn handle_code_lens(&self, params: ...) -> Vec<CodeLens> {
    if !self.config.enable_code_lens { return vec![]; }
    // ... 既存の CodeLens 生成
}
```

#### `hoverDetailLevel`

`stdlib_hover_info()` に `level: HoverDetailLevel` を渡し、level に応じて出力を制限する。

#### `diagnosticsReportLevel`

`publish_diagnostics()` でフィルタリングする。

```rust
fn filter_diagnostics(diags: Vec<Diagnostic>, level: DiagnosticsLevel) -> Vec<Diagnostic> {
    match level {
        DiagnosticsLevel::ErrorsOnly => diags.into_iter().filter(|d| d.is_error()).collect(),
        DiagnosticsLevel::Warnings => diags.into_iter().filter(|d| d.severity() != Severity::Help).collect(),
        DiagnosticsLevel::All => diags,
    }
}
```

### Step 5: README に設定一覧を追加する

`extensions/arukellt-all-in-one/README.md` に「Extension Settings」セクションを追加する。

```markdown
## Extension Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `arukellt.server.path` | string | `"arukellt"` | Path to the arukellt CLI |
| `arukellt.target` | string | `"wasm32-wasi-p1"` | Default compilation target |
| `arukellt.enableCodeLens` | boolean | `true` | Show Run/Debug/Test CodeLens |
| `arukellt.hoverDetailLevel` | string | `"standard"` | Hover information verbosity |
| `arukellt.useSelfHostBackend` | boolean | `false` | Use selfhost compiler backend |
| `arukellt.diagnostics.reportLevel` | string | `"all"` | Diagnostic severity filter |
| `arukellt.check.onSave` | boolean | `true` | Run check on save |
```

---

## 依存関係

- 依存なし（独立して着手可能）
- Issue 458（CodeLens 再設計）: `enableCodeLens` を読む側は Issue 458 で実装する
- Issue 457（availability 統一）: `hoverDetailLevel` の verbose モードで availability を表示
- Issue 459（selfhost）: `useSelfHostBackend` の実動作は Issue 459 完了後

---

## 影響範囲

- `extensions/arukellt-all-in-one/package.json`（設定定義追加）
- `extensions/arukellt-all-in-one/src/extension.js`（initializationOptions 拡張）
- `crates/ark-lsp/src/server.rs`（`LspConfig` struct, 各ハンドラへの反映）
- `extensions/arukellt-all-in-one/README.md`（設定一覧）

---

## 後方互換性

- 全設定にデフォルト値があるため既存の動作は変わらない。
- `initializationOptions` に新フィールドを追加しても、LSP サーバー側が旧バージョンであれば無視される（additive 変更）。

---

## 今回の範囲外

- workspace 単位設定（`.vscode/settings.json` で上書き）は VS Code が自動で処理するため追加実装不要
- 設定変更の動的反映（再起動なし）: `didChangeConfiguration` のハンドリングは最低限のみ実装
- GUI 設定パネルの追加

---

## 完了条件

- [x] `package.json` に 5 設定が追加されている（型・デフォルト・description・scope 全て設定）
- [x] `enableCodeLens: false` で CodeLens が消える
- [x] `hoverDetailLevel: "minimal"` でシグネチャのみ表示になる
- [x] `diagnostics.reportLevel: "errors"` で警告が LSP から届かなくなる
- [x] `extension.js` が新設定を initializationOptions に含めて LSP サーバーに渡す
- [x] README に設定一覧テーブルが存在する
- [x] `bash scripts/run/verify-harness.sh` 通過

---

## 必要なテスト

1. LSP プロトコルテスト: `enableCodeLens: false` を initializationOptions に入れて codeLens リクエスト → 空配列
2. LSP プロトコルテスト: `hoverDetailLevel: "minimal"` で hover → examples セクションなし
3. LSP プロトコルテスト: `diagnosticsReportLevel: "errors"` で警告のみのファイル → 0 diagnostics

---

## 実装時の注意点

- `workspace/didChangeConfiguration` は VS Code が設定変更時に自動送信する。これを受けた LSP サーバーが `ServerState.config` を更新し、次回リクエストから新しい設定が使われるようにする。
- `useSelfHostBackend: true` の場合に selfhost が未完了なら silent fallback（エラーにしない）とし、Output チャンネルにログを出す。
