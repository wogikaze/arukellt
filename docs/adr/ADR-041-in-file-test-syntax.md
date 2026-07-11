# ADR-041: In-file Test Syntax — `test` Declarations

ステータス: **ACCEPTED**

決定日: 2026-07-04

---

## 文脈

Arukellt にはこれまで言語レベルのテスト構文がなく、`arukellt test` コマンドは
`check_only=true` の型チェックラッパに過ぎなかった（`src/compiler/main/project_run.ark`
`cmd_test` 参照）。テスト関数の検出は `test_` / `_test` 命名規約のみ（Issue #458）で、
プロダクションコードとテストの境界は慣習に依存していた。

ファイル内テストを言語機能として導入し、以下の 3 レイヤーを単一の構文体系で
表現したい:

1. **関数単位（白箱）**: 対象関数の直近にテストを書く
2. **テストモジュール単位**: ファイル内でテストをグループ化する
3. **ブラックボックス（結合）**: public API 視点のテスト

比較言語:
- Rust: `#[test]` 属性 + `mod tests`。属性システムが未存在の Arukellt には導入コスト大
- Haskell: プロパティ関数。構文追加不要だがディスカバリが命名規約頼み
- Zig: `test "name" { }`。キーワードベースで Arukellt の宣言モデルと親和性高い

## 決定事項

### D1: `test` キーワードによる 3 形式の宣言

Arukellt は `pub`/`async`/`fn`/`struct`/`resource` とキーワードベースの宣言モデルで
一貫しているため、新たな属性構文（`#[...]`）は導入せず `test` キーワードを
トップレベル item に追加する。

<!-- skip-doc-check -->
```ark
// 形式1: 単独テスト（ファイルスコープ）
test "sanity" {
    test::assert_eq_i32(1 + 1, 2)
}

// 形式2: 関数紐付けテスト（白箱・関数単位）
fn normalize(s: String) -> String { ... }

test normalize "trims_and_lowercases" {
    test::assert_eq_string(normalize(" AbC "), "abc")
}

// 形式3: テストモジュール（グループ・1階層のみ）
test mod "normalize suite" {
    test "idempotent" {
        let s = " AbC "
        test::assert_eq_string(normalize(normalize(s)), normalize(s))
    }
    test "empty" {
        test::assert_eq_string(normalize(""), "")
    }
}
```

### D2: スコープ規則

3 形式とも **ファイルスコープを引き継ぐ**（非 `pub` 項目を含む）。
`test <fn> "name"` は対象関数と同じファイルスコープに加え、対象関数の
非公開項目へアクセス可能（対象関数のローカル変数は捕捉しない）。

レイヤー3（ブラックボックス）は **構文上の特別な修飾子を持たない**。
`tests/` ディレクトリ配下に別ファイルとして配置することで、既存の
モジュール可視性ルールにより public API のみ参照可能となる。

### D3: `test mod` のネストは 1 階層のみ

`test mod` の直下には `test "..." {}` のみ書ける。`test mod` の中に
`test mod` は書けない。スコープ規則を単純に保つため。

### D4: AST ノード種

- `NK_TEST_DECL` (59): `test "name" {}` / `test <fn> "name" {}`
  - `text`: テスト名（文字列リテラル）
  - `int_val`: 対象関数紐付けフラグ（1 = 関数紐付けあり、0 = 単独）
  - `children[0]`: 対象関数名の IDENT ノード（関数紐付け時のみ）
  - `children[last]`: body BLOCK
- `NK_TEST_MOD` (60): `test mod "name" { ... }`
  - `text`: モジュール名
  - `children`: 配下の `NK_TEST_DECL` ノード群

### D5: MIR lowering では test 宣言をスキップ

`decl_kind_to_corehir` が未知の kind に `COREHIR_DECL_UNKNOWN()` を返す既存挙動に
より、test 宣言は自然に MIR lowering でスキップされる。通常コンパイル
（`arukellt compile` / `arukellt run`）では test 宣言は出力バイナリに含まれない。

### D6: `arukellt test` はディスカバリ + check_only

`arukellt test <file>` は:
1. 対象ファイルをパースし `NK_TEST_DECL` / `NK_TEST_MOD` を収集
2. 収集したテスト名を一覧表示
3. `check_only=true` で型チェックを実行（test body の型エラーを捕捉）

将来フェーズでテスト関数の実行モデルを追加する。

### D7: 実行モデル（設計）

将来の実行フェーズでは:
1. test 宣言ごとに `__test_<hash>` 関数を MIR 合成
2. 全 test 関数を順呼びする `__test_main` を合成
3. WASM にコンパイルし wasmtime で実行
4. `--filter <pattern>` / `--layer unit|integration` フラグで選択実行

### D8: カバレッジ採用方針（#715）

in-file テストの採用方針:

- `std/`: 純関数・不変条件（core, collections, text, bytes）を in-file テストでカバー
- `src/compiler/`: 変換パス内の純ヘルパ（lexer, parser, resolver, typechecker, mir, diagnostics）を in-file テストでカバー
- 副作用モジュール（host, component, lsp, dap, wasm emitter 本体）は fixture のみ

in-file テストは **白箱ユニット**（実装近接・非 `pub` 参照可）。既存 fixture ハーネス（run / t3-compile / t3-run）は **結合・副作用** 向けと併用する。

## 影響範囲

- lexer: `TK_TEST` (36) 追加、`keywords_decl` に "test" 登録
- parser: `NK_TEST_DECL` / `NK_TEST_MOD` 追加、`decl_test.ark` 新規、
  `decl_dispatch` / `decl_module` へ分岐追加
- resolver: `kinds` / `ast_decl_predicates` に test 種追加、
  `program.ark` で test body をファイルスコープで解決
- typechecker: `kinds` / `ast_decl_predicates` に test 種追加、
  `entry.ark` で test body を `check_stmt` で型チェック
- CLI: `cmd_test` を test ディスカバリ + 一覧表示に拡張
- corehir: 変更不要（未知 kind は `COREHIR_DECL_UNKNOWN` で自然スキップ）

## 将来の拡張

- `--filter` / `--layer` CLI フラグ
- `std::test::quickcheck(prop_fn, iterations)` によるプロパティテスト
- JSON レポーター
- VS Code CodeLens の `#[test]` 期待から `test` 宣言への移行
