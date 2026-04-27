---
Status: done
Created: 2026-04-02
Updated: 2026-04-02
ID: 450
Track: vscode-ide
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 1
Stmt: ":Let {"
---

# LSP: ローカル変数の Go to Definition を identifier span ベースに修正する
`textDocument/definition` で変数名の定義に飛ぶと、定義 range が `let` 文全体（または次行まで）を指している。`Cmd/Ctrl+hover` の preview が 1 行以上表示されるのはこの bug が原因。`Stmt: ":Let` の `span` フィールドが `let x = expr` 全体を指すため、`find_let_in_block` が返す span も全体になっている。修正は `Stmt::Let` に `name_span: Span` フィールドを追加し、定義位置として変数名トークンのみを返すようにすること。"
### `crates/ark-parser/src/ast.rs` の `Stmt: ":Let`"
name: String,
ty: Option<TypeExpr>,
init: Expr,
is_mut: bool,
pattern: Option<Pattern>,
span: let_start..=init_end,
ast: ":Item::FnDef(f) if f.name == name => return Some(f.span),"
`Stmt: ":Let` に `name_span: Span` を追加し、parser でその span をセットする。`find_let_in_block` では `*name_span` を返す。"
### Step 1: "`Stmt::Let` に `name_span` フィールドを追加 (`crates/ark-parser/src/ast.rs`)"
name_span: "Span,  // ← 追加: 変数名識別子のみの span"
### Step 2: "Parser で `name_span` をセットする (`crates/ark-parser/src/parser.rs` または let 文パース箇所)"
let 文をパースしている箇所を特定する（grep: "`Stmt::Let {` で発見）。変数名トークンをパースした直後のトークン span を `name_span` としてセットする。"
let is_mut = self.try_consume(Token: ":Mut);"
### Step 3: "`find_let_in_block` の修正 (`crates/ark-lsp/src/server.rs` 行 1448)"
### Step 4: `FnDef.span` の確認と修正
確認方法: `hover_preview` を関数名上で実行して preview が関数全体を含むか確認する。含む場合は同様の修正が必要。
`Param.span` も同様に確認する。パラメータ `p.span` が `name: type` 全体なら `p.name_span` を追加する。
### Step 5: shadowing の確認
shadowing ケース: 同名変数が内側スコープで再定義された場合、`find_let_in_block` は走査順に最初に見つかったものを返す可能性がある。
採用方針: 本 issue では「カーソル位置に最も近い（最内側の）束縛に飛ぶ」を完了条件とする。`find_let_in_block` は現在、block.stmts を先頭から走査するため、**ブロックを内側から探索する**必要がある。
### Step 6: "LSP プロトコルテストの追加 (`crates/ark-lsp/tests/lsp_e2e.rs`)"
// let source = String: ":from("hello")"
// expected: "range points only to `source` (the name token), not full let statement"
let result = lsp_goto_definition(src, line: "2, col: 10);"
- `crates/ark-parser/src/ast.rs`（`Stmt: ":Let` フィールド追加）"
- `Stmt: ":Let` の変更後に `crates/` 内で `Stmt::Let {` を pattern match している全箇所（MIR lowering、type checker 等）が `name_span` を要求する exhaustive match になっていないか確認する。`..` で無視できるようにしておく（フィールド追加なので `..` を使った match は影響を受けない）。"
1. LSP E2E テスト: local let, function arg, function name それぞれで range.start/end を検証
2. shadowing ケースのテスト（最低限: 同名変数が 2 つある時、使用箇所から正しい束縛に飛ぶ）
3. `Stmt: ":Let` の新フィールドに対する exhaustive match が存在する場合のコンパイル確認"
# LSP: ローカル変数の Go to Definition を identifier span ベースに修正する

## Summary

`textDocument/definition` で変数名の定義に飛ぶと、定義 range が `let` 文全体（または次行まで）を指している。`Cmd/Ctrl+hover` の preview が 1 行以上表示されるのはこの bug が原因。`Stmt::Let` の `span` フィールドが `let x = expr` 全体を指すため、`find_let_in_block` が返す span も全体になっている。修正は `Stmt::Let` に `name_span: Span` フィールドを追加し、定義位置として変数名トークンのみを返すようにすること。

---

## 根本原因の特定

### `crates/ark-parser/src/ast.rs` の `Stmt::Let`

```rust
pub enum Stmt {
    Let {
        name: String,
        ty: Option<TypeExpr>,
        init: Expr,
        is_mut: bool,
        pattern: Option<Pattern>,
        span: Span,  // ← let 文全体の span（let キーワードから初期化式末尾まで）
    },
    // ...
}
```

`span` フィールドは `let` キーワードから初期化式末尾までのフル span。変数名識別子のみの span がない。

### `crates/ark-lsp/src/server.rs` の `find_let_in_block`

```rust
ast::Stmt::Let { name: n, span, .. } if n == name => return Some(*span),
```

ここで `*span`（let 文全体）を返している。これが definition range になる。

### 修正方針

`Stmt::Let` に `name_span: Span` を追加し、parser でその span をセットする。`find_let_in_block` では `*name_span` を返す。

---

## 詳細実装内容

### Step 1: `Stmt::Let` に `name_span` フィールドを追加 (`crates/ark-parser/src/ast.rs`)

```rust
pub enum Stmt {
    Let {
        name: String,
        name_span: Span,  // ← 追加: 変数名識別子のみの span
        ty: Option<TypeExpr>,
        init: Expr,
        is_mut: bool,
        pattern: Option<Pattern>,
        span: Span,       // let 文全体（変更なし）
    },
}
```

### Step 2: Parser で `name_span` をセットする (`crates/ark-parser/src/parser.rs` または let 文パース箇所)

let 文をパースしている箇所を特定する（grep: `Stmt::Let {` で発見）。変数名トークンをパースした直後のトークン span を `name_span` としてセットする。

```rust
// パーサー内の let 文パース（概略）
let let_start = self.current_span();         // "let" キーワードの span
// mut の有無を確認
let is_mut = self.try_consume(Token::Mut);
let name_token = self.expect_ident()?;       // 変数名トークン
let name_span = name_token.span;             // ← これが name_span
let name = name_token.text.clone();
// ... ty, init のパース
Stmt::Let {
    name,
    name_span,  // ← セット
    ty,
    init,
    is_mut,
    pattern,
    span: let_start..=init_end,
}
```

### Step 3: `find_let_in_block` の修正 (`crates/ark-lsp/src/server.rs` 行 1448)

```rust
// 変更前:
ast::Stmt::Let { name: n, span, .. } if n == name => return Some(*span),
// 変更後:
ast::Stmt::Let { name: n, name_span, .. } if n == name => return Some(*name_span),
```

### Step 4: `FnDef.span` の確認と修正

`find_definition_span` で `FnDef` の場合 `f.span` を返している。`FnDef.span` が関数本体全体を指すなら、同様に `name_span` が必要になる。

```rust
ast::Item::FnDef(f) if f.name == name => return Some(f.span),
```

`FnDef.span` の実際の範囲を確認する（parser を grep して fn キーワードから body 末尾までか、fn キーワード + 名前だけかを確認）。もし全体 span なら `FnDef` にも `name_span` を追加する。


`Param.span` も同様に確認する。パラメータ `p.span` が `name: type` 全体なら `p.name_span` を追加する。

### Step 5: shadowing の確認

shadowing ケース: 同名変数が内側スコープで再定義された場合、`find_let_in_block` は走査順に最初に見つかったものを返す可能性がある。

```ark
let x = 1
{
    let x = 2  // shadow
    print(x)   // → ここから definition に飛ぶ
}
```

現在の実装が「最初に見つかった let を返す」動作の場合、shadowing で内側を返すためにはスコープを考慮した探索が必要。


現状の実装がスコープを正しく扱っているかを確認し、内側ブロックの束縛を優先しない問題があれば修正する。修正が大きい場合は別 issue として分離する（本 issue では `name_span` 修正と合わせて、shadowing が存在しない単純ケースの正確化を完了条件とする）。

### Step 6: LSP プロトコルテストの追加 (`crates/ark-lsp/tests/lsp_e2e.rs`)

既存のテストパターンに倣い、以下のテストケースを追加する。

```rust
#[test]
fn test_goto_definition_local_variable_has_identifier_span() {
    // let source = String::from("hello")
    // cursor on `source` usage
    // expected: range points only to `source` (the name token), not full let statement
    let src = "fn main() {\n    let source = 42\n    print(source)\n}\n";
    // cursor at usage of `source` in print(source) → line 2, col 10
    let result = lsp_goto_definition(src, line: 2, col: 10);
    // definition should be at line 1, col 8..13 (just "source")
    assert_eq!(result.range.start.line, 1);
    assert_eq!(result.range.start.character, 8);
    assert_eq!(result.range.end.character, 14); // len("source") = 6
}
```

テストケースの最小セット:
- ローカル変数（`let x = ...`）で definition range が識別子のみを指す
- 関数引数で definition range が引数名のみを指す
- 関数名で definition range が関数名のみを指す（`FnDef` 修正後）
- shadowing で内側の束縛に飛ぶ（スコープ修正後）

---

## 依存関係

- Issue 453（VS Code API E2E テスト）は本 issue の完了後にこの挙動を E2E で確認するテストを追加できる。
- Issue 454（LSP regression fixture）は本 issue の修正後の挙動を snapshot 化する。
- `ast.rs` の変更は parser が `name_span` をセットしないとコンパイルエラーになるため、parser の修正と LSP の修正を同じ commit/PR に含める。

---

## 影響範囲

- `crates/ark-parser/src/ast.rs`（`Stmt::Let` フィールド追加）
- `crates/ark-parser/src/` のパーサー実装（`name_span` セット）
- `crates/ark-lsp/src/server.rs`（`find_let_in_block` / `find_definition_span` 修正）
- `crates/ark-lsp/tests/lsp_e2e.rs`（テスト追加）
- `Stmt::Let` を exhaustive match しているコード全体（compiler のパターンマッチ）

---

## 後方互換性・移行影響

- `Stmt::Let` に `name_span` フィールドを追加する。フィールド追加は AST の struct-like enum variant の変更であり、`Stmt::Let { .. }` ワイルドカードで受け取っているすべての match 式でコンパイルエラーは出ない（`..` があれば）。明示的フィールドリストで match している箇所は修正が必要。
- LSP の definition range が変わることは **意図的な修正**。外部ツールが definition range に依存している場合は変わるが、正しい範囲への修正である。

---

## 今回の範囲外（明確な非対象）

- `let (a, b) = ...` のタプル分解パターンでの definition（`pattern` フィールドを使うケース）
- multi-file cross definition（Issue 453/454 スコープ）
- rename provider の range 修正（別の issue として追跡）

---

## 完了条件

- [x] `let source = 42` の定義に飛んだ時に range が `source` トークンのみを指す（let 文全体にならない）
- [x] 関数引数の定義に飛んだ時に range が引数名のみを指す
- [x] LSP E2E テスト `test_goto_definition_local_variable_has_identifier_span` が pass
- [x] `cargo test --workspace` で全テスト pass
- [x] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

1. LSP E2E テスト: local let, function arg, function name それぞれで range.start/end を検証
2. shadowing ケースのテスト（最低限: 同名変数が 2 つある時、使用箇所から正しい束縛に飛ぶ）
3. `Stmt::Let` の新フィールドに対する exhaustive match が存在する場合のコンパイル確認

---

## 実装時の注意点

- パーサーで `name_span` をセットする際、`let mut x` の場合は `mut` キーワードの次のトークンが変数名になる。`let x` の場合は `let` の次。どちらの場合も `Ident` トークンの span を取得する。
- `Stmt::Let` の変更後に `crates/` 内で `Stmt::Let {` を pattern match している全箇所（MIR lowering、type checker 等）が `name_span` を要求する exhaustive match になっていないか確認する。`..` で無視できるようにしておく（フィールド追加なので `..` を使った match は影響を受けない）。
- definition range の `end` は `start + name.len()` として計算できるが、UTF-8 文字を含む識別子の場合は byte offset と character offset が異なる。`span_to_range` 関数が既に UTF-8 を正しく扱っているか確認する。