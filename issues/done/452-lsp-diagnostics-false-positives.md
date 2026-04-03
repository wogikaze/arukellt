# LSP: E0100 偽陽性 diagnostics を解消し CLI check と一致させる

**Status**: done
**Created**: 2026-04-02
**Updated**: 2026-04-03
**ID**: 452
**Depends on**: none
**Track**: vscode-ide
**Blocks v1 exit**: no
**Priority**: 1


---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: lsp_e2e.rs has parity_valid_prelude_only_program_no_diagnostics, snapshot_diagnostics_valid_program_zero_errors, parity_real_error_matches_cli — full CLI/LSP parity coverage (test function renamed during implementation)

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/452-lsp-diagnostics-false-positives.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

VS Code 上で `E0100 unresolved name` が大量に出ており（`extHost2` ログ起点）、CLI での `arukellt check` では出ない。LSP と CLI check の diagnostics 結果が一致していない。本 issue では偽陽性の根本原因を特定・除去し、VS Code diagnostics が `arukellt check` と同じ結果を返すことを完了条件とする。

---

## 根本原因の候補

以下の候補を順に調査・潰す。

### 候補 A: workspace root 解決のズレ

LSP サーバーの `refresh_diagnostics` が単一ファイルの source を解析するが、マルチファイル workspace では他ファイルからのインポートが解決できず `E0100` になる。

確認箇所: `crates/ark-lsp/src/server.rs` の `analyze_source` / `refresh_diagnostics` が `Session` にワークスペースパスを渡しているか、もしくは単一ファイルモードで実行しているかを確認する。

### 候補 B: analysis_cache の失効タイミング不整合

`analysis_cache` は `uri` をキーとして結果をキャッシュする。ファイルが変更されても `or_insert_with` が既存エントリを返す場合がある（`entry(...).or_insert_with(...)` は既にエントリがあれば挿入しない）。

確認箇所: `textDocument/didChange` / `textDocument/didSave` ハンドラーで `analysis_cache` から該当 uri のエントリを削除しているか確認する。

```rust
// did_change handler の期待される動作:
let mut cache = self.analysis_cache.lock().unwrap();
cache.remove(&uri);  // キャッシュ失効
drop(cache);
self.refresh_diagnostics(uri, &new_text).await;
```

この削除がない場合、ファイル変更後も古い解析結果が使われ続け、新しく追加された定義が未解決に見える。

### 候補 C: project-aware resolve vs. single-file resolve の不一致

`analyze_source` 内で単一ファイルとして parse → resolve → typecheck を実行する場合、他ファイルの定義（`use foo::bar` 等）が解決できない。CLI の `check` は全ファイルを project として解析するため一致しない。

確認箇所: `analyze_source` の実装が単一ファイルを対象にしているか、プロジェクト全体を対象にしているかを確認する。

### 候補 D: stdlib symbol の未ロード

stdlib prelude の関数（`print`, `println`, `push` 等）が LSP の `resolve` 段階でバインドされていない場合、prelude 関数呼び出しが `E0100` になる。

確認箇所: `analyze_source` が `Session` に stdlib manifest / stdlib source をロードした状態で実行されているかを確認する。`DiagnosticSink` に `E0100` が出ているシンボルが prelude 関数かどうかを特定する。

---

## 詳細実装内容

### Step 1: 根本原因の特定

1. 同じ `.ark` ファイルを `arukellt check <file>` と LSP で開いて diagnostics を比較する。
2. LSP の diagnostics に出ている `E0100` シンボル名を記録する。
3. そのシンボルが:
   - stdlib prelude 関数（→ 候補 D）
   - 別ファイルで定義された関数/型（→ 候補 A/C）
   - 同じファイルで定義されているが未解決（→ 候補 B）
   のいずれかを特定する。

### Step 2: キャッシュ失効の修正（候補 B）

`textDocument/didChange` と `textDocument/didSave` ハンドラーを確認し、cache invalidation が不足している場合は追加する。

```rust
// server.rs の did_change / did_save ハンドラー
async fn did_change(&self, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    let new_text = params.content_changes
        .into_iter()
        .last()
        .map(|c| c.text)
        .unwrap_or_default();

    // 1. ドキュメントを更新
    {
        let mut docs = self.documents.lock().unwrap();
        docs.insert(uri.clone(), new_text.clone());
    }
    // 2. analysis cache を失効させる（必須）
    {
        let mut cache = self.analysis_cache.lock().unwrap();
        cache.remove(&uri);
    }
    // 3. 再解析して diagnostics を publish
    self.refresh_diagnostics(uri, &new_text).await;
}
```

### Step 3: stdlib prelude バインド確認（候補 D）

`analyze_source` 内の resolve/typecheck が stdlib prelude を含む状態で実行されているか確認する。

```rust
fn analyze_source(source: &str) -> AnalysisResult {
    let mut sink = DiagnosticSink::new();
    // ここで stdlib manifest からの prelude 注入が行われているか確認
    // ...
    let resolved = Resolver::new(&module_graph, &manifest_prelude).resolve(&module);
    // ...
}
```

prelude 注入がない場合、`StdlibManifest` から prelude 関数一覧を取り出し、`Resolver` に渡す。既存の CLI check 経路（`ark-driver::Session`）がこれをどう実現しているかを参照する。

### Step 4: マルチファイル resolve（候補 A/C）

LSP が単一ファイル解析のみ行っている場合、他ファイルからのインポートは解決できない。これを完全に修正するにはプロジェクト全体の解析が必要であり、それは大きな変更になる。

**採用方針（本 issue の範囲）**: 同一ファイル内で完結するコードの E0100 偽陽性を解消することを優先する。マルチファイル resolve は Issue 441（project-aware workspace）のスコープとして扱い、本 issue の非対象とする。ただし、マルチファイル import が原因の E0100 は「解決中」というプレースホルダー診断を出すのではなく、LSP でエラーを抑制する方が良いかを検討する。

**実装**: 未解決のシンボルが known import module のシンボルである場合（`use std::host::http::get` 等）は、E0100 を suppress して LSP から publish しない。フィルタリングロジックを `collect_lsp_diagnostics` に追加する（ただし suppression は最終手段。根本解決を優先する）。

### Step 5: CLI との整合確認テスト追加

```rust
// crates/ark-lsp/tests/lsp_e2e.rs に追加
#[test]
fn test_lsp_diagnostics_match_cli_for_valid_file() {
    // stdlib prelude のみ使う有効なファイル
    let src = "fn main() {\n    println(\"hello\")\n}\n";
    let lsp_diags = lsp_get_diagnostics(src);
    // CLI check と同じ結果: エラーなし
    assert_eq!(lsp_diags.len(), 0,
        "valid file should produce no diagnostics, got: {:?}", lsp_diags);
}

#[test]
fn test_lsp_diagnostics_stable_after_change() {
    let src = "fn main() {\n    let x = 1\n    println(x.to_string())\n}\n";
    let lsp_diags_before = lsp_get_diagnostics(src);
    // ファイルを変更して再解析
    let src2 = "fn main() {\n    let y = 2\n    println(y.to_string())\n}\n";
    let lsp_diags_after = lsp_get_diagnostics(src2);
    // 変更後にキャッシュ由来の古い結果が残っていないことを確認
    assert!(lsp_diags_after.iter().all(|d| !d.message.contains("y")));
}
```

### Step 6: save / reload 後の安定性確認

LSP セッションで:
1. ファイルを開く → diagnostics を確認
2. ファイルを変更して保存 → diagnostics を確認
3. LSP サーバーを再起動（VS Code reload） → diagnostics を確認

3 つのタイミングすべてで同じ結果が返ることを手動またはテストで確認する。

---

## 依存関係

- Issue 453（VS Code API E2E）は本 issue の修正を E2E で確認する。
- Issue 454（regression fixture）は diagnostics 安定後に snapshot 化する。
- Issue 441（project-aware workspace）はマルチファイル resolve の根本解決を担当（本 issue の非対象）。

---

## 影響範囲

- `crates/ark-lsp/src/server.rs`（キャッシュ失効、stdlib prelude 注入、diagnostics 収集）
- `crates/ark-lsp/tests/lsp_e2e.rs`（テスト追加）

---

## 後方互換性・移行影響

- 偽陽性 diagnostics の除去は機能改善であり、正当なエラーを抑制する変更ではない。
- suppress ロジックを追加する場合、正当な E0100（本当に unresolved なシンボル）を誤って suppressed しないよう注意する。

---

## 今回の範囲外（明確な非対象）

- マルチファイル workspace のクロスファイル resolve（Issue 441 スコープ）
- rename/references での cross-file 解析
- workspace-wide diagnostics scan（全ファイル一括再解析）

---

## 完了条件

- [x] 有効な stdlib prelude 使用コードで `arukellt check` と LSP diagnostics が一致（両方エラーなし）
- [x] ファイル変更後に LSP diagnostics が正しく更新される（キャッシュ失効が動作）
- [x] VS Code で同一ファイルを再起動前後で同じ diagnostics が出る
- [x] `test_lsp_diagnostics_match_cli_for_valid_file` が pass
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

1. `test_lsp_diagnostics_match_cli_for_valid_file`: 有効ファイルで E0100 が出ない
2. `test_lsp_diagnostics_stable_after_change`: 変更後にキャッシュ由来の古い結果が出ない
3. E0100 が正当に出るケース（本当に unresolved なシンボル）が suppress されていないことの確認テスト

---

## 実装時の注意点

- `analysis_cache` の `entry(...).or_insert_with(...)` パターンは Rust の `HashMap::entry` API で「キーが存在すればそのまま、なければ挿入」を意味する。`did_change` で `cache.remove(&uri)` してから `or_insert_with` を使えば毎回再解析される。ただし、`or_insert_with` 内の解析が非常に重い場合はパフォーマンス劣化に注意する。
- stdlib prelude の注入は `StdlibManifest` から prelude 関数リストを取り出し、`Resolver` の既知シンボルテーブルに事前登録することで実現できる。既存の `ark-driver::Session::new` の初期化パターンを参照する。
- `E0100` の diagnostic code が本当に "unresolved name" を意味するか `codes.rs` で確認する（`E0100` の `spec()` を見る）。
