# ADR-089: 言語 v2 の impl 型パラメータは明示 binder でだけ導入する

ステータス: **ACCEPTED** — impl head の型パラメータは `impl<T>` の binder でだけ導入し、名前の形による暗黙の型パラメータ推定を廃止する

決定日: 2026-10-27

関連: [ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-078](ADR-078-language-v2-parametric-trait-impl.md)、[ADR-079](ADR-079-language-v2-method-level-generics.md)、[ADR-014](ADR-014-stability-labels.md)、[#946](../../issues/open/946-single-uppercase-implicit-impl-param.md)

## 文脈

impl head は `impl<T: Bound> Trait for Ctor<T>` の明示 binder を持てる。
一方、binder のない `impl Trait for Ctor<T>` では、head に現れる大文字 1 文字の名前を暗黙の型パラメータとみなす慣習が実装に残っている。
この慣習はどの ADR にも採択されていない。

名前の形で型か型変数かを決めるため、`struct S` に対する `impl Copy for S {}` は `S` が型変数と解釈され、
user blanket impl として E0216 で拒否される（#946）。
大文字 1 文字の名前を持つ型は impl の対象にできず、同じ綴りが文脈によって型にも型変数にもなる。

## 決定

1. impl の型パラメータは、trait impl・inherent impl を問わず `impl<T, U: Bound>` の binder でだけ導入する。
2. impl head と impl 本体に現れる型名は、binder で導入した名前を除き、通常の型名解決に従う。
   解決できない名前は未定義の型として診断し、binder の追加を案内する。
3. 名前の長さや大文字・小文字の形を、型か型パラメータかの判定に使ってはならない。
4. 関数・struct・enum・trait の型パラメータは既に明示宣言であり、この ADR はその規則を変更しない。
5. 暗黙の型パラメータ推定は廃止する。移行は ADR-014 に従い、移行用の診断（binder を付けた形を提示する警告）を経てエラー化する。

## 帰結

- `impl Copy for S {}` は宣言済みの `struct S` を対象とする impl として扱われる。
- 暗黙の形で書かれた既存の parametric impl は binder 付きの形へ移行する必要がある。移行の進捗は #946 と current-state に置く。
- ADR-078 の parametric / blanket の分類は、binder で導入した型パラメータに対して従来どおり適用する。

## 実装メモ

2026-10-29 の残差解消で、return・tuple・parameter・mono の符号化は、宣言済み binder、宣言済み struct/enum 名、登録済み集約レイアウトを参照する。
parameter の初期型付けは宣言済み型パラメータと宣言済み nominal 型を先に区別し、宣言済み enum は `enum:` 綴りを保つ。
mono の型変数置換と ordinal 収集も、登録済み struct/enum を型変数として扱わない。
名前の形による型変数判定は、宣言も登録もない綴りに対する最後のフォールバックに限る。
検証と残差は [#946](../../issues/open/946-single-uppercase-implicit-impl-param.md) と `docs/current-state.md` に記録する。

## 却下した代替案

- **宣言済みの型への解決を優先し、解決できない大文字 1 文字だけを暗黙の型パラメータにする。**
  移行は不要だが、名前の形に依存する規則が残り、同じ綴りの型を後から宣言すると impl の意味が変わるため採用しない。
- **現行の慣習を仕様として文書化する。**
  大文字 1 文字の型を impl の対象にできない制限を言語に固定するため採用しない。

## 再検討条件

- 型パラメータの省略記法を、名前の形に依存しない規則（例: 専用の記号）で導入する必要が生じた場合。

## 関連文書

- [ADR-062: Language v2 の Trait core と coherence](ADR-062-language-v2-trait-core-coherence.md)
- [ADR-078: Language v2 の parametric trait impl](ADR-078-language-v2-parametric-trait-impl.md)
- [#946: single-uppercase type names in impl heads are treated as implicit type parameters](../../issues/open/946-single-uppercase-implicit-impl-param.md)
