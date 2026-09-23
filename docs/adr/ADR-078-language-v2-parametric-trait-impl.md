# ADR-078: Language v2 の parametric trait impl

ステータス: **ACCEPTED** — self 型が具象 constructor 適用である generic trait impl を blanket から分離し、bounded parametric impl を合法化する

決定日: 2026-09-23

関連: [ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-066](ADR-066-language-v2-generic-public-api.md)、[ADR-076](ADR-076-language-v2-iterator-consumer-boundary.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#691](../../issues/open/691-iterator-trait-lazy-adapters.md)、[#860](../../issues/open/860-iterator-unification-integration.md)

## 文脈

ADR-062 は「type variable を含む user impl は blanket impl とみなし、初期 v2 では
拒否する」と定めた。この規則は `impl<T> Trait for T` のような真の blanket impl
（self 型が裸の型変数）を防ぎ、coherence と termination を registry 契約で保証する
ための暫定境界である。

一方、`impl<I: Iterator> Iterator for MapIter<I>` は self 型が具象な nominal
constructor `MapIter` の適用であり、候補キーを constructor head で一意に絞れる。
Iterator adapter や container への trait 実装（`Clone for Vec<T>` など）はこの形状を
必要とし、concrete instantiation ごとの impl 列挙では user-defined iterator を上游に
取る adapter を std が提供できない。#691 と #860 の adapter 移行がこの判断を要求する。

## 決定

### 1. parametric impl と blanket impl を分離する

user impl を次の 2 種に分類する。

- **parametric impl**: self 型が `Ctor<args>` の形状で、`Ctor` が宣言済み nominal
  type（struct / enum / builtin container）である impl。引数位置に型変数を含んでよい。
- **blanket impl**: self 型が裸の型変数、または型変数を head とする impl
  （`impl<T> Trait for T`）。

blanket impl は ADR-062 通り拒否を維持する。parametric impl は本 ADR の条件を満たす
場合に合法とする。

### 2. parametric impl の合法条件

- self 型の head は宣言済み nominal constructor でなければならない。
- impl の型パラメータはすべて self 型の引数位置に出現しなければならない
  （unconstrained parameter を持つ impl は拒否する）。
- 型パラメータは trait bound を持ってよい。bound は instantiation 時の obligation
  として再検査する。
- trait 参照の型引数に型変数を含める場合、その変数は self 型に出現するパラメータに
  限る。

### 3. coherence と候補解決

- 同じ `(trait, self constructor head)` に対し、parametric impl は高々 1 つとする。
- parametric impl が登録された `(trait, ctor)` には concrete instantiation impl を
  追加できず、その逆も同様とする。parametric pattern は任意の instantiation に一致
  するため、共存は必ず overlap する。
- concrete self 型の候補解決は、まず concrete impl を constructor head と全引数の構造
  一致で探し、なければ parametric impl を head 一致と引数 unification で試す。
  parametric impl が一致した場合、宣言された bound を concrete 引数へ置き換えた
  obligation を再帰的に検査する。
- obligation の再帰は self 型の真の部分項へ降りるため有限の型で停止する。concrete
  base case を持たない再帰 impl chain は bound error として報告する。

### 4. associated item と dispatch

- parametric impl の associated type 定義（`type Item = I::Item` など）は impl の
  substitution 下で射影する。
- method の monomorphisation は generic function と同じ instantiation 規則を使い、
  callee owner は既存の per-instantiation 命名（`Ctor_arg1_arg2::method`）に従う。
- dispatch は static のみとする。`dyn Trait` の境界は ADR-062 §3 を変更しない。

### 5. compiler-owned blanket との関係

`Ord for Reverse<T>` のような compiler-owned rule は、user が書けない形状を registry
側が保証する仕組みであり維持する。parametric impl は user 構文で書ける合法形であり、
compiler-owned registry へは追加しない。

## 帰結

- `impl<I: Iterator> Iterator for MapIter<I>` を始めとする generic adapter impl が user
  構文で書ける。`std::core::iter` の adapter はこの形へ移行する（#691、#860）。
- `impl<T: Clone> Clone for Vec<T>` のような container impl も同じ規則で合法になる。
- `impl<T> Trait for T`、self に出現しない型パラメータを持つ impl、同一 `(trait, ctor)`
  の parametric + concrete 混在は引き続き拒否する。

## 却下した代替案

### concrete instantiation impl を adapter ごとに列挙する

`impl Iterator for MapIter<VecIter>` は書けるが、user-defined iterator を上游に取る
adapter を std が提供できない。組み合わせ爆発でカバレッジも有限になるため採用しない。

### compiler-owned blanket として adapter impl を登録する

adapter は user-defined type を upstream に取るため、compiler が把握する型集合を越える。
registry rule は「user が書けない形状」のための経路であり、言語構文で表せる impl を
compiler 側へ隠すことは semantic debt になるため採用しない。

### user blanket を全面解禁する

`impl<T> Trait for T` は任意の型へ候補を注入し、coherence の一意性と termination を
registry で検証できない。ADR-062 の禁止を維持する。

## 再検討条件

- self head が nominal constructor でない impl（trait object、関数型など）が必要に
  なった場合。
- parametric impl 間の overlap を bound の排他性で許す必要が出た場合。現行は head
  一致だけで判定し、bound による絞り込みを行わない。
- specialization や負の impl が必要になった場合。別 ADR で採択する。

## 関連文書

- [ADR-062: Language v2 の Trait core と coherence](ADR-062-language-v2-trait-core-coherence.md)
- [ADR-076: Language v2 Iterator の producer / consumer 責務境界](ADR-076-language-v2-iterator-consumer-boundary.md)
- [RFC-011: 言語 v2 の意味論再設計案](../rfcs/011-language-v2-semantics-redesign.md)
- [#691: Iterator trait, lazy adapters, FromIterator / collect](../../issues/open/691-iterator-trait-lazy-adapters.md)
- [#860: Iterator Trait 体系を統合完了する](../../issues/open/860-iterator-unification-integration.md)
