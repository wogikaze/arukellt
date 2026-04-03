# 拡張機能 CodeLens を Run Main / Debug / Run Test 中心に再設計する

**Status**: done
**Created**: 2026-04-02
**Updated**: 2026-04-03
**ID**: 458
**Depends on**: 453
**Track**: lsp, extension
**Blocks v1 exit**: no
**Priority**: 2


---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: CodeLens commands registered at extension.js:668-711, runMain/debugMain/runTest/debugTest

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/458-codelens-run-debug-redesign.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在の CodeLens は全 `FnDef` に「Open Docs」「Explain Function」を 2 個ずつ出しているだけで、実行・デバッグ用途に使えない。`fn main()` に「▶ Run Main」「⬛ Debug」、test 命名規約に合致する関数に「▶ Run Test」「⬛ Debug Test」を出し、その他の関数には CodeLens を出さない（または明確な価値があるときだけ出す）設計に変更する。

---

## 前提

- `crates/ark-lsp/src/server.rs` line 4681–4725: 現在の CodeLens は `FnDef` ごとに `arukellt.openDocs` + `arukellt.explainCode` を固定で 2 個出す実装。
- `extensions/arukellt-all-in-one/package.json`: `arukellt.openDocs` / `arukellt.explainCode` コマンドは Command として登録済み。
- Issue 453 (VS Code E2E テスト追加) が完了していることで CodeLens の回帰が検知できる。

---

## 詳細実装内容

### Step 1: LSP 側の CodeLens ロジックを書き替える (`crates/ark-lsp/src/server.rs`)

#### 1.1: test 関数の判定ルール

以下のいずれかを満たす関数を「test 関数」とみなす（この issue でのルール確定）。

1. 関数名が `test_` で始まる（例: `test_add`, `test_empty_list`）
2. 関数名が `_test` で終わる（例: `add_test`）
3. 将来的なアトリビュート `#[test]` が AST に付いている（現在のパーサー対応状況を確認し、未対応なら名前規約のみで判定する）

#### 1.2: main 関数の判定ルール

- 関数名が `main` であること（他の名前付け慣習がある場合は `arukellt.mainFunctionName` 設定で上書きできるようにする、ただし本 issue では `main` 固定で可）。

#### 1.3: CodeLens 生成ロジックの置き換え

```rust
fn build_code_lenses(fn_defs: &[FnDef]) -> Vec<CodeLens> {
    let mut lenses = Vec::new();
    for f in fn_defs {
        let name = &f.name;
        let range = span_to_range(f.name_span); // identifier span のみ
        if name == "main" {
            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title: "▶ Run Main".into(),
                    command: "arukellt.runMain".into(),
                    arguments: None,
                }),
                data: None,
            });
            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title: "⬛ Debug".into(),
                    command: "arukellt.debugMain".into(),
                    arguments: None,
                }),
                data: None,
            });
        } else if is_test_function(name) {
            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title: "▶ Run Test".into(),
                    command: "arukellt.runTest".into(),
                    arguments: Some(vec![serde_json::json!(name)]),
                }),
                data: None,
            });
            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title: "⬛ Debug Test".into(),
                    command: "arukellt.debugTest".into(),
                    arguments: Some(vec![serde_json::json!(name)]),
                }),
                data: None,
            });
        }
        // 通常関数: CodeLens なし
    }
    lenses
}
```

#### 1.4: CodeLens の range を identifier span に変更する

現在は `f.span`（関数全体）を使っているが、`f.name_span`（Issue 450 で追加）または関数名トークンの span を使うよう変更する。Issue 450 が未完了の場合は `f.span.start`..`f.span.start + f.name.len()` の近似で代替し、Issue 450 完了後に正式な `name_span` に差し替える。

### Step 2: 拡張側で新コマンドを登録する (`extensions/arukellt-all-in-one/`)

#### 2.1: `package.json` に新コマンドを追加する

```json
{ "command": "arukellt.runMain",   "title": "Arukellt: Run Main" },
{ "command": "arukellt.debugMain", "title": "Arukellt: Debug Main" },
{ "command": "arukellt.runTest",   "title": "Arukellt: Run Test" },
{ "command": "arukellt.debugTest", "title": "Arukellt: Debug Test" }
```

#### 2.2: `extension.js` でコマンドハンドラを実装する

```js
context.subscriptions.push(
    vscode.commands.registerCommand('arukellt.runMain', async () => {
        const file = vscode.window.activeTextEditor?.document.fileName;
        if (!file) return;
        const target = vscode.workspace.getConfiguration('arukellt').get('target', 'wasm32-wasi-p1');
        const terminal = vscode.window.createTerminal('Arukellt Run');
        terminal.show();
        terminal.sendText(`arukellt run --target ${target} "${file}"`);
    }),
    vscode.commands.registerCommand('arukellt.debugMain', async () => {
        const file = vscode.window.activeTextEditor?.document.fileName;
        if (!file) return;
        vscode.debug.startDebugging(undefined, {
            type: 'arukellt',
            request: 'launch',
            name: 'Debug Arukellt Program',
            program: file,
        });
    }),
    vscode.commands.registerCommand('arukellt.runTest', async (testName: string) => {
        const file = vscode.window.activeTextEditor?.document.fileName;
        if (!file) return;
        const terminal = vscode.window.createTerminal('Arukellt Test');
        terminal.show();
        terminal.sendText(`arukellt test --filter ${testName} "${file}"`);
    }),
    vscode.commands.registerCommand('arukellt.debugTest', async (testName: string) => {
        const file = vscode.window.activeTextEditor?.document.fileName;
        if (!file) return;
        vscode.debug.startDebugging(undefined, {
            type: 'arukellt',
            request: 'launch',
            name: `Debug Test: ${testName}`,
            program: file,
        });
    }),
);
```

#### 2.3: 既存の `openDocs` / `explainCode` を Command Palette に移す

- CodeLens からは除去する（`build_code_lenses` に含めない）。
- `package.json` のコマンド定義・タイトルは残す（Command Palette からは使える）。
- `arukellt.openDocs` / `arukellt.explainCode` のハンドラは extension.js に残す。
- hover Markdown にこれらへのリンクを追加することは、Issue 451（semantic hover）で対応する。

#### 2.4: `enableCodeLens` 設定のサポート（Issue 462 との整合）

Issue 462 で追加予定の `arukellt.enableCodeLens` 設定を読んで、`false` の場合は全 CodeLens を空配列で返す。本 issue でその設定を追加しても良いが、Issue 462 と重複しないよう、設定追加は Issue 462 側で行い、本 issue では設定値を読む側だけ実装する。

### Step 3: `arukellt test --filter` コマンドの確認

`arukellt.runTest` コマンドが `arukellt test --filter <test_name>` を発行する。これが CLI に存在しない場合（`arukellt test` の filter オプション）は、本 issue のスコープとして `--filter` 引数を `Commands::Test` に追加する。

現状確認: `crates/arukellt/src/main.rs` の `Commands::Test` に `--filter` 引数があるか確認し、なければ追加する。

---

## 依存関係

- Issue 450（`FnDef.name_span` 追加）: 完了している場合は `name_span` を使う。未完了なら近似で代替。
- Issue 453（VS Code E2E テスト）: 完了後に CodeLens E2E テストを追加する。
- Issue 462（設定整理）: `enableCodeLens` 設定読み取り。

---

## 影響範囲

- `crates/ark-lsp/src/server.rs`（CodeLens ハンドラ）
- `extensions/arukellt-all-in-one/package.json`（コマンド追加）
- `extensions/arukellt-all-in-one/src/extension.js`（コマンドハンドラ追加）
- `crates/arukellt/src/main.rs`（`--filter` 引数、未実装の場合）

---

## 後方互換性

- `arukellt.openDocs` / `arukellt.explainCode` は削除せず Command Palette から使えるまま残す。
- CodeLens の表示件数が 2 → 0 になる関数があるが、機能削減ではなく改善として扱う。

---

## 今回の範囲外

- 通常関数への「Run Function」CodeLens（本 issue では追加しない）
- LSP 側からの動的テスト検出（現在の AST 走査で十分）
- `#[test]` アトリビュートのパーサー対応（未実装なら名前規約のみで判定）

---

## 完了条件

- [x] `fn main()` の上に「▶ Run Main」「⬛ Debug」の 2 lens が出る
- [x] `fn test_foo()` の上に「▶ Run Test」「⬛ Debug Test」の 2 lens が出る
- [x] 通常関数（main でも test でもない）に CodeLens が出ない
- [x] `arukellt.runMain` コマンドが terminal で `arukellt run` を実行する
- [x] `arukellt.runTest <name>` コマンドが terminal で `arukellt test --filter <name>` を実行する
- [x] VS Code E2E テストで CodeLens のタイトル・コマンド・range を assertion する
- [x] `bash scripts/run/verify-harness.sh` 通過

---

## 必要なテスト

1. LSP プロトコルテスト: `textDocument/codeLens` で main を含むファイル → 2 lenses (Run Main, Debug)
2. LSP プロトコルテスト: test 関数を含むファイル → 2 lenses per test function
3. LSP プロトコルテスト: 通常関数のみのファイル → 0 lenses
4. VS Code E2E テスト（Issue 453 フレームワーク使用）: `vscode.executeCodeLensProvider` で titles と commands を assert

---

## 実装時の注意点

- `is_test_function` の判定は `test_` prefix のみでも十分だが、`_test` suffix も含める（後退しない範囲で）。
- `arukellt.runMain` が現在のファイルを対象とするため、CodeLens を発行したファイルの URI を command arguments に渡すか、拡張側で active editor の URI を使う。LSP 側から URI を arguments に含める方が確実。
- CodeLens の `range` は identifier span を使うことで、関数が長くても lens が先頭行に出る。
