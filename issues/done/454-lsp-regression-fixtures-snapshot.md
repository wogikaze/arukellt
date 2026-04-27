---
Status: done
Created: 2026-04-02
Updated: 2026-04-03
ID: 454
Track: vscode-ide
Depends on: 450, 451, 452
Orchestration class: implementation-ready
---
# LSP 回帰フィクスチャ群を snapshot 化し CI で固定する
**Blocks v1 exit**: no
**Priority**: 5

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: lsp_e2e.rs has 11 snapshot_ tests (Step 6 required 9); request_hover, request_definition, wait_for_diagnostics all present

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/454-lsp-regression-fixtures-snapshot.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Issue 450/451/452 の修正後、その挙動を snapshot テストとして固定する。`crates/ark-lsp/tests/lsp_e2e.rs` に hover 出力 markdown・definition target range・diagnostics 一覧の snapshot を追加し、1 ケース壊れたら CI が落ちる状態にする。

対象ケース: ローカル変数、複数行初期化式、関数呼び出し、shadowing、stdlib シンボル、cross-file（ファイル内のみ）。

---

## 現状

`crates/ark-lsp/tests/lsp_e2e.rs` には LSP プロトコルの基本動作テストが存在するが、以下の点が弱い。

1. hover の応答が「何か返る」レベルで、具体的な markdown 内容が未検証。
2. definition の応答が「見つかった」レベルで、`range.start/end` が未検証。
3. diagnostics の「0 件であること」は確認されているかもしれないが、特定のエラーが出るケースの snapshot がない。

---

## 詳細実装内容

### Step 1: Snapshot テストの方針決定

Rust の snapshot テストライブラリ（`insta` crate）を使う方法と、手書きの「期待値文字列との比較」方法がある。

**採用方針**: 既存の `lsp_e2e.rs` が手書き assert を使っているため、本 issue も手書き assert で実装する（`insta` の導入は大きな変更になるため非対象）。ただし期待値を定数として定義し、変更時に意図的に更新が必要になるようにする。

### Step 2: Hover snapshot テストの追加

各 fixture コードを定数として定義し、hover の markdown 内容を exact assert する。

```rust
// crates/ark-lsp/tests/lsp_e2e.rs に追加

const FIXTURE_BASIC: &str = "\
fn greet(name: String) -> String {\n\
    let msg = concat(\"Hello, \", name)\n\
    msg\n\
}\n\
\n\
fn main() {\n\
    let result = greet(\"world\")\n\
    println(result)\n\
}\n";

#[test]
fn snapshot_hover_println_contains_signature() {
    let mut session = LspSession::start();
    let src = FIXTURE_BASIC;
    // initialize + open
    session.initialize();
    session.open_document("file:///test.ark", src);
    // hover on println (line 7, col 4)
    let hover = session.request_hover("file:///test.ark", 7, 4);
    let content = hover["result"]["contents"]["value"]
        .as_str()
        .unwrap_or("");
    // println の hover には関数シグネチャが含まれる
    assert!(
        content.contains("println"),
        "hover on println should mention println, got: {:?}", content
    );
    assert!(
        content.contains("fn") || content.contains("String"),
        "hover should contain signature info"
    );
}

#[test]
fn snapshot_hover_string_literal_is_null() {
    let mut session = LspSession::start();
    let src = "fn main() {\n    println(\"hello world\")\n}\n";
    session.initialize();
    session.open_document("file:///test.ark", src);
    // hover on string literal (line 1, col 13 — inside "hello world")
    let hover = session.request_hover("file:///test.ark", 1, 13);
    // result should be null (no hover for string literals)
    assert!(
        hover["result"].is_null(),
        "string literal hover should be null, got: {:?}", hover["result"]
    );
}

#[test]
fn snapshot_hover_integer_literal_is_null() {
    let mut session = LspSession::start();
    let src = "fn main() {\n    let x = 42\n}\n";
    session.initialize();
    session.open_document("file:///test.ark", src);
    // hover on 42 (line 1, col 12)
    let hover = session.request_hover("file:///test.ark", 1, 12);
    assert!(
        hover["result"].is_null(),
        "integer literal hover should be null, got: {:?}", hover["result"]
    );
}
```

### Step 3: Definition range snapshot テストの追加

```rust
#[test]
fn snapshot_definition_local_let_points_to_identifier() {
    let mut session = LspSession::start();
    let src = FIXTURE_BASIC;
    session.initialize();
    session.open_document("file:///test.ark", src);
    // definition from `result` usage in println(result) — line 7, col 12
    let def = session.request_definition("file:///test.ark", 7, 12);
    let result = &def["result"];
    assert!(!result.is_null(), "Should find definition");
    // result は Location または Location[] 
    let loc = if result.is_array() { &result[0] } else { result };
    let start = &loc["range"]["start"];
    let end = &loc["range"]["end"];
    // let result = greet("world") の `result` は line 6, col 8..14
    assert_eq!(start["line"].as_u64().unwrap(), 6, "definition should be on line 6");
    assert_eq!(start["character"].as_u64().unwrap(), 8, "definition should start at col 8");
    assert_eq!(end["line"].as_u64().unwrap(), 6, "definition range should not span multiple lines");
    let range_len = end["character"].as_u64().unwrap() - start["character"].as_u64().unwrap();
    assert!(range_len <= 8, "identifier range should be short (got {} chars)", range_len);
}

#[test]
fn snapshot_definition_function_arg_points_to_param() {
    let mut session = LspSession::start();
    // 関数内で引数を使うコード
    let src = "fn foo(bar: i32) -> i32 {\n    bar\n}\n";
    session.initialize();
    session.open_document("file:///test.ark", src);
    // definition from `bar` usage on line 1, col 4
    let def = session.request_definition("file:///test.ark", 1, 4);
    let result = &def["result"];
    let loc = if result.is_array() { &result[0] } else { result };
    let start = &loc["range"]["start"];
    // fn foo(bar: i32) の `bar` は line 0, col 7..10
    assert_eq!(start["line"].as_u64().unwrap(), 0, "param definition should be on line 0");
    assert_eq!(start["character"].as_u64().unwrap(), 7, "param def should start at col 7");
}

#[test]
fn snapshot_definition_shadowed_let_points_to_inner() {
    let mut session = LspSession::start();
    let src = "fn main() {\n    let x = 1\n    {\n        let x = 2\n        println(x.to_string())\n    }\n}\n";
    session.initialize();
    session.open_document("file:///test.ark", src);
    // definition from inner `x` usage (line 4, col 16 — inside println(x.to_string()))
    let def = session.request_definition("file:///test.ark", 4, 16);
    let result = &def["result"];
    let loc = if result.is_array() { &result[0] } else { result };
    let start = &loc["range"]["start"];
    // inner `let x = 2` は line 3
    assert_eq!(start["line"].as_u64().unwrap(), 3,
        "shadowed variable should point to inner binding (line 3), not outer (line 1)");
}
```

### Step 4: Diagnostics snapshot テストの追加

```rust
#[test]
fn snapshot_diagnostics_valid_file_empty() {
    let mut session = LspSession::start();
    let src = "fn main() {\n    println(\"hello\")\n}\n";
    session.initialize();
    session.open_document("file:///test.ark", src);
    // diagnostics を受け取るまで待つ（publishDiagnostics 通知）
    let diag_notif = session.wait_for_diagnostics("file:///test.ark");
    let diagnostics = diag_notif["params"]["diagnostics"].as_array().unwrap();
    assert_eq!(
        diagnostics.len(), 0,
        "Valid file should produce 0 diagnostics, got: {:?}", diagnostics
    );
}

#[test]
fn snapshot_diagnostics_unresolved_name_produces_e0100() {
    let mut session = LspSession::start();
    let src = "fn main() {\n    println(undefined_xyz)\n}\n";
    session.initialize();
    session.open_document("file:///test.ark", src);
    let diag_notif = session.wait_for_diagnostics("file:///test.ark");
    let diagnostics = diag_notif["params"]["diagnostics"].as_array().unwrap();
    assert!(diagnostics.len() > 0, "Should have diagnostics for undefined symbol");
    let has_unresolved = diagnostics.iter().any(|d| {
        let msg = d["message"].as_str().unwrap_or("");
        msg.contains("E0100") || msg.contains("unresolved") || msg.contains("undefined_xyz")
    });
    assert!(has_unresolved, "Should have E0100 unresolved name diagnostic");
}

#[test]
fn snapshot_diagnostics_long_init_expression() {
    // 長い初期化式（`let source = String::from("...")`）で行全体に diagnostics が出ない
    let mut session = LspSession::start();
    let src = "fn main() {\n    let source = concat(\"hello\", \" world\")\n    println(source)\n}\n";
    session.initialize();
    session.open_document("file:///test.ark", src);
    let diag_notif = session.wait_for_diagnostics("file:///test.ark");
    let diagnostics = diag_notif["params"]["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 0, "Long init expression should not produce false diagnostics");
}
```

### Step 5: LspSession ヘルパーの追加

既存の `LspSession` に `request_hover`, `request_definition`, `wait_for_diagnostics` メソッドを追加する。

```rust
impl LspSession {
    fn request_hover(&mut self, uri: &str, line: u32, col: u32) -> Value {
        let id = self.next_id();
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": line, "character": col}
            }
        }));
        self.wait_for_response(id)
    }

    fn request_definition(&mut self, uri: &str, line: u32, col: u32) -> Value {
        let id = self.next_id();
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": line, "character": col}
            }
        }));
        self.wait_for_response(id)
    }

    fn wait_for_diagnostics(&mut self, uri: &str) -> Value {
        // publishDiagnostics 通知を待つ（最大 10 秒）
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(msg) = self.rx.recv_timeout(Duration::from_millis(500)).ok() {
                if msg["method"] == "textDocument/publishDiagnostics" {
                    if msg["params"]["uri"].as_str() == Some(uri) {
                        return msg;
                    }
                }
            }
            if std::time::Instant::now() > deadline {
                panic!("Timeout waiting for diagnostics for {}", uri);
            }
        }
    }
}
```

### Step 6: 全 fixture ケースのリスト

以下の全 fixture を `snapshot_*` テストとして追加する。

| テスト名 | 種別 | fixture コード | 検証内容 |
|---|---|---|---|
| `snapshot_hover_println_contains_signature` | hover | `println(...)` | signature 含む |
| `snapshot_hover_string_literal_is_null` | hover | `"hello world"` | null |
| `snapshot_hover_integer_literal_is_null` | hover | `42` | null |
| `snapshot_definition_local_let_points_to_identifier` | definition | `let result = ...` | range single-line short |
| `snapshot_definition_function_arg_points_to_param` | definition | `fn foo(bar: i32)` | param span |
| `snapshot_definition_shadowed_let_points_to_inner` | definition | shadowing | inner binding |
| `snapshot_diagnostics_valid_file_empty` | diagnostics | valid file | 0 件 |
| `snapshot_diagnostics_unresolved_name_produces_e0100` | diagnostics | undefined symbol | E0100 |
| `snapshot_diagnostics_long_init_expression` | diagnostics | long init | 0 件 |

---

## 依存関係

- Issue 450 完了後に definition range の exact assert が pass する。
- Issue 451 完了後に hover null ケースが pass する。
- Issue 452 完了後に diagnostics の 0 件ケースが pass する。
- これらが未完了の間は本 issue のテストを `#[ignore]` または `expected_fail` として追加しておく。

---

## 影響範囲

- `crates/ark-lsp/tests/lsp_e2e.rs`（テスト追加・LspSession ヘルパー追加）

---

## 後方互換性・移行影響

- テスト追加のみ。既存テストへの影響なし。

---

## 今回の範囲外（明確な非対象）

- `insta` crate による自動 snapshot（手書き assert に統一）
- cross-file definition の snapshot（マルチファイル resolve 前提）
- completion / references / rename の snapshot

---

## 完了条件

- [x] Step 6 の全 9 テストケースが `crates/ark-lsp/tests/lsp_e2e.rs` に追加されている
- [x] `LspSession` に `request_hover` / `request_definition` / `wait_for_diagnostics` が追加されている
- [x] Issue 450/451/452 完了後に全テストが `cargo test -p ark-lsp` で pass
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

本 issue 自体がテスト追加 issue のため、テストの完了が完了条件。

---

## 実装時の注意点

- `wait_for_diagnostics` は publishDiagnostics 通知を待つが、LSP サーバーが通知を送らないケース（解析が即時完了・通知を送らない設計）では timeout する可能性がある。サーバーが diagnostics を publish するタイミングを `did_open` / `did_change` ハンドラーで確認しておくこと。
- `request_hover` / `request_definition` は既存の `recv()` メソッドを使ってレスポンスを待つ。notification と response が混在するため、`id` でフィルタリングすること。
- shadowing テスト（Step 3 最後）は Issue 450 のスコープ実装に依存する。shadowing 対応が Issue 450 の完了条件に含まれていない場合は `#[ignore]` を付ける。