---
Status: done
Created: 2026-04-02
Updated: 2026-04-10
ID: 451
Track: vscode-ide
Depends on: none
Orchestration class: implementation-ready
---

# LSP: Hover をセマンティックな identifier に限定し literal / keyword の無意味表示を除去する
Blocks v1 exit: no
Priority: 2
TokenKind: ":StringLit(_) => "string literal".to_string(),  // ← ノイズ"
if let Some(type_info) = Self: ":type_hover_info(...) {"
} else if let Some(stdlib_info) = Self: ":stdlib_hover_info(...) {"
} else if let Some(mod_info) = Self: ":stdlib_module_hover(text, m) {"
format!("identifier `{}`", text)  // ← ノイズ: null を返すべき
修正: "non-identifier および semantic 情報のない identifier は `return Ok(None)` にする。"
### Step 1: "hover handler の分岐修正 (`crates/ark-lsp/src/server.rs`)"
if let TokenKind: ":Ident(_) = &tok.kind {"
let info = if let Some(type_info) = Self: ":type_hover_info(...) {"
if let Some(stdlib_info) = Self: ":stdlib_hover_info(text, m) {"
return Ok(None);  // ← semantic info なし: hover なし
return Ok(None);  // ← manifest なし + type info なし: hover なし
contents: "HoverContents::Markup(MarkupContent {"
kind: "MarkupKind::Markdown,"
value: info,
range: "Some(Self::span_to_range(&source, tok.span)),"
注意: "`return Ok(None)` は「このトークンに hover なし」だが、ループ外で `Ok(None)` を返す現状の実装でも問題ない。ただし非 identifier のトークンに当たった時点で即 `Ok(None)` を返すことで、隣接する identifier トークンを「誤爆」しないようにする。"
### Step 2: `identifier \`x\`` フォールバックの除去
### Step 3: 境界ケース（identifier の末尾・直後の空白）
採用方針: `target_offset < end`（strictly less）にする（識別子 `source` の最後の文字 `e` の offset が `end - 1`、その直後の offset は `end` で次のスペースや記号の先頭）。ただし、LSP の標準的な実装では「カーソルが identifier 上にある」を `start <= offset < end` で判定することが多い。完了条件に「識別子の末尾の直後で隣接トークンを誤爆しない」を含める。
### Step 4: `keyword` hover の除去確認
Ark のキーワード（`let`, `fn`, `if`, `while` 等）が `TokenKind: ":Ident` ではなく別の variant で入っている場合、Step 1 の `if let TokenKind::Ident(_)` で自動的に除外される。キーワードが `TokenKind::Ident` と同じ variant として入っている場合は、キーワードリストと照合して hover を出さないようにする必要がある。"
確認方法: `ark-lexer/src/token.rs` の `TokenKind` 定義を確認し、`fn`, `let`, `if` 等のキーワードが `Ident` variant か否かを特定する。`Ident` variant と別 variant の場合は Step 1 で十分。
### Step 5: "LSP プロトコルテストの追加 (`crates/ark-lsp/tests/lsp_e2e.rs`)"
let result = lsp_hover(src, line: "0, col: 12);"
- Step 1 の変更で「`TokenKind: ":Ident` にヒットして semantic info がない場合に即 `Ok(None)` を返す」実装にすると、同じ offset に複数のトークンが重なっている（通常は起こらないが念のため）場合にも最初のヒットで終了する。問題なし。"
# LSP: Hover をセマンティックな identifier に限定し literal / keyword の無意味表示を除去する

## Summary

現在の hover handler（`crates/ark-lsp/src/server.rs`）はカーソル位置のトークンを種類を問わず hover に変換する。`"hello"` 上でホバーすると `"string literal"` が、整数リテラルでは `"integer literal \`42\`"`が、キーワードや記号では `` "`if`" `` のような意味のない文字列が表示される。これらはすべて除去する。

加えて、identifier ではあるが semantic 情報が取れない場合（型情報なし・stdlib なし）に表示される `"identifier \`x\`"` というフォールバック文字列も除去する（ユーザーには `null` hover が返るべき）。

---

## 根本原因の特定

`crates/ark-lsp/src/server.rs` の hover handler（行 3587–3630 付近）:

```rust
let info = match &tok.kind {
    TokenKind::Ident(_) => {
        if let Some(type_info) = Self::type_hover_info(...) {
            type_info
        } else if let Some(stdlib_info) = Self::stdlib_hover_info(...) {
            stdlib_info
        } else if let Some(mod_info) = Self::stdlib_module_hover(...) {
            mod_info
        } else {
            format!("identifier `{}`", text)  // ← ノイズ: null を返すべき
        }
    }
    TokenKind::IntLit(_) => {
        format!("integer literal `{}`", text)  // ← ノイズ
    }
    TokenKind::FloatLit(_) => {
        format!("float literal `{}`", text)    // ← ノイズ
    }
    TokenKind::StringLit(_) => "string literal".to_string(),  // ← ノイズ
    _ => format!("`{}`", text),                                // ← ノイズ
};
return Ok(Some(Hover { ... }));  // ← ここでノイズ hover を返している
```

修正: non-identifier および semantic 情報のない identifier は `return Ok(None)` にする。

---

## 詳細実装内容

### Step 1: hover handler の分岐修正 (`crates/ark-lsp/src/server.rs`)

```rust
for tok in &analysis.tokens {
    let start = tok.span.start as usize;
    let end = tok.span.end as usize;
    if start <= target_offset && target_offset <= end && end <= source.len() {
        let text = &source[start..end];

        // Identifier のみ hover を試みる
        if let TokenKind::Ident(_) = &tok.kind {
            let info = if let Some(type_info) = Self::type_hover_info(...) {
                type_info
            } else if let Some(ref m) = *manifest {
                if let Some(stdlib_info) = Self::stdlib_hover_info(text, m) {
                    stdlib_info
                } else if let Some(mod_info) = Self::stdlib_module_hover(text, m) {
                    mod_info
                } else {
                    return Ok(None);  // ← semantic info なし: hover なし
                }
            } else {
                return Ok(None);  // ← manifest なし + type info なし: hover なし
            };

            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: info,
                }),
                range: Some(Self::span_to_range(&source, tok.span)),
            }));
        }
        // Identifier 以外（リテラル、キーワード、記号）は hover なし
        // → ループを break して None を返す
        return Ok(None);
    }
}
```


### Step 2: `identifier \`x\`` フォールバックの除去

上記 Step 1 で `return Ok(None)` に置き換える。この変更により、ローカル変数で型情報が取れない場合（型チェッカーが未解析の場合など）も hover が出なくなる。これは現状の `"identifier \`x\`"` というノイズ表示よりも良い UX である。

型情報が取れないケースを将来的に改善するには Issue 453/454 の semantic hover 強化が必要。本 issue はノイズ除去のみを担当する。

### Step 3: 境界ケース（identifier の末尾・直後の空白）

LSP の `textDocument/hover` はカーソル位置（character offset）を受け取る。トークン境界の扱いを確認する。

現状のコード:

```rust
if start <= target_offset && target_offset <= end && end <= source.len() {
```

`target_offset == end`（識別子の直後）のケースは現状 hover が出る。これは「識別子の末尾の直後でも hover が出る」という挙動になっている。


修正後の条件:

```rust
if start <= target_offset && target_offset < end
```

### Step 4: `keyword` hover の除去確認

Ark のキーワード（`let`, `fn`, `if`, `while` 等）が `TokenKind::Ident` ではなく別の variant で入っている場合、Step 1 の `if let TokenKind::Ident(_)` で自動的に除外される。キーワードが `TokenKind::Ident` と同じ variant として入っている場合は、キーワードリストと照合して hover を出さないようにする必要がある。

確認方法: `ark-lexer/src/token.rs` の `TokenKind` 定義を確認し、`fn`, `let`, `if` 等のキーワードが `Ident` variant か否かを特定する。`Ident` variant と別 variant の場合は Step 1 で十分。

### Step 5: LSP プロトコルテストの追加 (`crates/ark-lsp/tests/lsp_e2e.rs`)

以下のテストケースを追加する。

```rust
#[test]
fn test_hover_string_literal_returns_none() {
    let src = "fn main() { println(\"hello world\") }\n";
    // cursor over the string literal "hello world"
    let result = lsp_hover(src, line: 0, col: 21);
    assert!(result.is_none(), "string literal should return no hover");
}

#[test]
fn test_hover_integer_literal_returns_none() {
    let src = "fn main() { let x = 42 }\n";
    let result = lsp_hover(src, line: 0, col: 21);
    assert!(result.is_none(), "integer literal should return no hover");
}

#[test]
fn test_hover_unknown_identifier_returns_none() {
    // 型情報なし・stdlib なし の識別子
    let src = "fn main() { let xyz_unknown = 1 }\n";
    let result = lsp_hover(src, line: 0, col: 16);
    // No meaningful semantic info → no hover
    assert!(result.is_none(), "unknown identifier with no semantic info should return no hover");
}

#[test]
fn test_hover_known_function_returns_content() {
    let src = "fn main() { println(\"hi\") }\n";
    let result = lsp_hover(src, line: 0, col: 12);
    assert!(result.is_some(), "known stdlib function should return hover");
    let content = result.unwrap();
    assert!(content.contains("println"), "hover should mention function name");
}
```

---

## 依存関係

- Issue 450（definition span 修正）とは独立して進行可能（同じファイルの変更だが非重複領域）。
- Issue 453（VS Code API E2E）は本 issue の完了後に hover ノイズが出ないことを E2E で確認する。
- Issue 454（regression fixture snapshot）は本 issue 完了後に hover null ケースを snapshot 化する。

---

## 影響範囲

- `crates/ark-lsp/src/server.rs`（hover handler の修正）
- `crates/ark-lsp/tests/lsp_e2e.rs`（テスト追加）

---

## 後方互換性・移行影響

- `"string literal"` / `"integer literal \`42\`"` の hover 文字列が消える。これらに依存したテストや UI があれば修正が必要。現状のテストは `lsp_e2e.rs` で確認した限り "何か返る" レベルのアサーションであり、具体的な文字列を期待していなければ影響なし。
- VS Code 上での hover UX が変わる（リテラルで何も出なくなる）。これは意図的な改善。

---

## 今回の範囲外（明確な非対象）

- 型情報が取れないローカル変数に型推論結果を hover 表示する（Issue 453 以降の semantic 強化）
- ドキュメントコメント付き関数の hover 強化
- hover の range 精度向上（すでに Issue 450 で扱う）

---

## 完了条件

- [x] 文字列リテラル上で hover が `null`（VS Code 上で何も表示されない）
- [x] 整数・浮動小数点リテラル上で hover が `null`
- [x] semantic info のない identifier 上で hover が `null`
- [x] `println` 等 stdlib 関数上で hover が有意な内容を返す
- [x] 識別子の末尾直後（空白文字位置）で隣接 token の hover が出ない
- [x] `test_hover_string_literal_returns_none` 等 LSP テストが pass
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

1. 各リテラル種別（string, int, float）で hover が null になることの LSP E2E テスト
2. unknown identifier で hover が null になることの LSP E2E テスト
3. known stdlib 関数で hover が有意な内容を返すことの LSP E2E テスト（regression 防止）
4. 境界位置（identifier 末尾の直後）で誤爆しないことのテスト

---

## 実装時の注意点

- Step 1 の変更で「`TokenKind::Ident` にヒットして semantic info がない場合に即 `Ok(None)` を返す」実装にすると、同じ offset に複数のトークンが重なっている（通常は起こらないが念のため）場合にも最初のヒットで終了する。問題なし。
- `target_offset < end` に変更する際、既存のテストが `target_offset == end` を入力として使っているケースがないか確認する。
- `TokenKind` の variant 構成は `ark-lexer/src/token.rs` を確認すること。`Keyword` variant が存在する場合は Step 4 は不要になる。