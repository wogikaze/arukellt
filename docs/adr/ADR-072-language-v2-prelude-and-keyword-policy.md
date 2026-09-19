# ADR-072: Language v2 Prelude と keyword policy

ステータス: **ACCEPTED** — v2 の Prelude、boolean literal、予約語分類を固定する
日付: 2026-09-19
トラック: language-v2
Issue: [#868](../../issues/open/868-prelude-booleans-keywords.md)
関連: [ADR-014](ADR-014-stability-labels.md)、[ADR-018](ADR-018-language-docs-classification.md)、[ADR-036](ADR-036-trait-stdlib-redesign.md)、[ADR-046](ADR-046-free-function-eradication.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)

---

## 文脈

`true` と `false` は boolean の値であり、Prelude から名前解決する値ではない。
一方、既存の resolver は `std::prelude` の公開宣言を広く bare name として登録している。
Prelude の将来契約をこの実装上の移行境界と同一視すると、literal の一意性と API の安定性を保証できない。

また、lexer は実装済みの hard keyword と、文脈によってだけ意味を持つ語を区別していなかった。
未実装の構文語を通常の identifier として許すと、後から機能を追加したときに既存プログラムの意味が変わる。

## 決定

### 1. v2 Prelude を安定集合として列挙する

`std/manifest.toml` の `[prelude_policy]` を v2 の安定集合の正本とする。
安定な型は `Option`、`Result`、`String`、`Vec`、安定な値は `Some`、`None`、`Ok`、`Err` である。
`true` と `false` は `TK_BOOL` の lexical literal であり、Prelude value には含めない。

安定な関数は `panic`、`assert`、`String_from`、`String_new`、`char_to_string` とする。
`len`、`push`、`pop`、`get`、`set`、`get_unchecked` は、内部実装と既存コードのための intrinsic compatibility group として明示する。

`HashMap` と、manifest の他の `prelude = true` entry は、現行移行期間の legacy auto-import である。
`legacy_auto_import = true` は現在の実装状態を記録するもので、v2 の安定集合を拡張する宣言ではない。
legacy boundary の縮小と call site 移行は #878 の責務とし、この ADR で暗黙に削除しない。

### 2. keyword を三分類する

- **hard**: 現在の lexer token を持つ語。`fn`、`struct`、`enum`、`let`、`mut`、制御フロー語、visibility/module 語、trait 語、`const`、`proof`、`resource`、`async`、`Self`、および `true`/`false` を含む。
- **contextual**: lexer では `TK_IDENT` のままにし、parser が特定位置だけで意味を付与する語。`test`、`move`、`dyn`、`type`、`mod` を含む。現行の test declaration、closure capture、`dyn Trait`、associated type、module path の文法を壊さないためである。
- **future-reserved**: lexer が `TK_RESERVED` として扱い、実装が存在しなくても source identifier として拒否する語。現在は `await`、`unsafe`、`extern`、`super` とする。

future-reserved の解除には、対応する構文の採択、parser/formatter/LSP/highlighting の同期、diagnostic fixture、GC/fallback parity の検証を含む別の設計判断が必要である。
「まだ未実装だから」という理由だけで identifier に戻してはならない。

分類の機械可読な一覧は [`docs/data/language-keyword-policy.toml`](../data/language-keyword-policy.toml) に置く。

### 3. target による分類差を作らない

keyword token、boolean literal、Prelude policy は source semantics の契約である。
`wasm32-gc` と `wasm32` は同じ lexer/parser/resolver 契約を使い、backend のメモリ表現によって予約語や literal の意味を変えない。

## 代替案と却下理由

### `true`/`false` を Prelude value として残す

literal と通常の名前解決が競合し、shadowing と target 差の検証を複雑にするため却下する。

### 現在の `prelude = true` 全件を v2 の安定 Prelude とする

既存の広い compatibility surface を新しい安定保証へ昇格させ、trait-first API への移行余地を失うため却下する。

### 未実装語をすべて identifier として解放する

将来の構文追加が既存コードを再解釈するため却下する。既存文法で使われている語は contextual として明示し、それ以外の将来語は予約する。

## 帰結

`std/manifest.toml` から `true`/`false` の value entry を削除し、resolver の literal 処理を唯一の意味源とする。
旧 broad auto-import は #878 が完了するまで残るため、current-state では stable v2 policy と legacy runtime behavior を別々に記述する。
LSP rename、playground tokenizer、formatter/highlighting fixture は同じ分類表に追随する。

## 再検討条件

- #878 の migration が完了し、legacy auto-import boundary を削除できる場合。
- `type`、`mod` などの contextual syntax を hard/future-reserved に昇格する新しい grammar が採択された場合。
- Wasm GC と linear-memory fallback が同じ source token 契約を維持できないことが実証された場合。

## 関連

- [RFC-011: 言語 v2 の意味論再設計案](../rfcs/011-language-v2-semantics-redesign.md)
- [#868](../../issues/open/868-prelude-booleans-keywords.md)
- [#878](../../issues/open/878-language-v2-compatibility-migration.md)
