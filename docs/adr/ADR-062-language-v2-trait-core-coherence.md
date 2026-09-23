# ADR-062: Language v2 の Trait core と coherence

ステータス: **ACCEPTED** — `Self`、associated item、coherence、blanket、dispatch の意味論を固定する

決定日: 2026-09-19

関連: [ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-058](ADR-058-language-v2-semantics-redesign.md)、[ADR-078](ADR-078-language-v2-parametric-trait-impl.md)、[RFC-004](../rfcs/004-trait-expressiveness.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#856](../../issues/open/856-trait-core-coherence-generics.md)

> 補足: §2 の「type variable を含む user impl は blanket impl とみなし拒否する」句は、
> self 型が具象 constructor 適用である parametric impl について [ADR-078](ADR-078-language-v2-parametric-trait-impl.md) が限定的に改訂する。

## 文脈

Arukellt の parser と typechecker は `trait`、`impl Trait for T`、メソッド呼び出し、
型パラメータの bound を受け付ける。
一方、impl の登録は trait 名と receiver の名前だけを比較しており、trait の型引数、
型引数を含む receiver、associated item、generic impl の重なりを意味論として保持していなかった。
そのため、宣言順に依存した候補選択や、generic arity の実装上限を言語仕様と誤認する余地がある。

既存の #602 は、同じ `(Trait, SelfType)` の重複、曖昧な bound、解決できない bound を拒否する
基礎を導入した。
本 ADR はその上に、Language v2 の trait core が持つ識別単位と境界を定める。

## 決定

### 1. Trait の意味型と `Self`

Trait の候補は、表示用の名前ではなく、次の構造を持つ宣言として扱う。

```text
TraitRef = (trait identity, ordered type arguments)
ImplKey  = (TraitRef, implementing type)
```

同じ名前でも型引数の個数と順序が異なる trait は別の参照である。
trait 宣言中の `Self` は実装対象型のプレースホルダ、impl 中の `Self` はその impl の対象型を表す。
`Self` は receiver の有無に関係なく、引数・戻り値・associated item の型で使用できる。

trait の associated function は receiver 引数を持たない trait method である。
呼び出しは `Type::function(...)` または候補が一意でない場合の完全修飾形を使い、
通常の公開 free function の代替経路は作らない。

associated type は trait 内の `type Item` 宣言と impl 内の `type Item = Concrete` 定義で導入する。
`Self::Item` は impl 対象に対する射影として解決し、未解決の射影を別の候補へ暗黙に変更しない。

### 2. Coherence と候補解決

- concrete な impl の重複は、trait 型引数と implementing type を含む構造比較で拒否する。
- type argument の個数が異なる impl は同じ候補キーにならないが、宣言された trait arity と一致しない
  impl は拒否する。
- type variable を含む user impl は blanket impl とみなし、初期 v2 では拒否する。
- compiler-owned blanket は、明示的な semantic rule として登録されたものだけを許可する。
  その rule は入力の型がすでに解決している場合に一度だけ適用し、同じ obligation を再投入しない。
  現行の唯一の rule は stdlib の `Ord for Reverse<T>` であり、同じ形状以外の user impl はこの経路へ入らない。
- candidate が 0 個なら bound error、2 個以上なら coherence/ambiguity error とする。
  collection の順序や最初に見つかった impl を選択規則にしない。
- `Into` / `TryInto` の導出 blanket など、stdlib 由来の規則はこの registry の明示的な compiler-owned
  rule として後続の変換 issue が利用する。user impl より暗黙に優先させない。

これにより、coherence のキー、blanket の適用条件、solver の termination 条件が同じ registry 契約を共有する。

### 3. Dispatch の境界

v2 の trait dispatch は monomorphisation による static dispatch を主経路とする。
`dyn Trait` は trait object や vtable を暗黙に生成せず、型位置で認識したうえで明示的な unsupported
diagnostic とする。
動的 dispatch を導入する場合は object safety、vtable ABI、所有権、Wasm GC/linear fallback の表現を
別 ADR で採択する。

### 4. Generic arity

型パラメータは parser、typechecker、trait registry、monomorphisation の各段階で順序付き列として扱う。
言語意味論として固定した小さい個数上限は設けない。
backend や toolchain の制約が見つかった場合は、言語仕様を狭めず、該当する実装ギャップを issue として記録する。

### 5. Target parity

Trait の識別、bound、coherence、associated item の解決は target 非依存の source/typechecker 契約である。
`wasm32-gc` と `wasm32` fallback は同じ解決結果と observable source semantics を使い、GC representation の
違いを trait の合法性や候補選択に反映しない。

## 帰結

- trait impl registry は表示名だけでなく、ordered type arguments、generic pattern、associated type 定義を保持する。
- parser は `type` associated item と `dyn Trait` の型位置を contextual syntax として扱える。
- `dyn` の実行時 ABI、user blanket、specialization、default method の追加はこの ADR の範囲外であり、別の採択が必要である。
- RFC-004 の「associated type を初期段階で先送りする」提案は、Language v2 core について本 ADR で置き換える。
  RFC の他の制約、特に user blanket の禁止と static dispatch の既定は維持する。

## 却下した代替案

### trait 名と receiver 名だけで照合する

型引数を失い、`Trait<A>` と `Trait<B>` の候補を区別できない。
coherence と functional dependency を後段で再構築する必要があるため採用しない。

### user blanket を宣言順で選ぶ

候補の意味が module graph の読み込み順に依存し、termination と再現性を保証できないため採用しない。

### v2 core で `dyn Trait` を暗黙に受け付ける

static dispatch と異なる ABI、object safety、managed value の保持期間を未決のままコードに固定するため採用しない。

## 再検討条件

- compiler-owned blanket の追加で、明示的な入力 obligation と termination proof を registry だけで検証できない場合。
- `dyn Trait` の object safety と Wasm GC/linear fallback の同値 ABI が別 ADR で確定した場合。
- associated type projection の higher-kinded/recursive 型が必要になった場合。
