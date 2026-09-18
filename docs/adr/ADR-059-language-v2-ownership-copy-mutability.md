# ADR-059: 言語 v2 の所有権・Copy・可変性

ステータス: **ACCEPTED** — binding の move 状態、明示的な `Copy`、可変性を独立した型検査契約として採択する

決定日: 2026-09-19

関連: [ADR-002](ADR-002-memory-model.md)、[ADR-013](ADR-013-primary-target.md)、[ADR-029](ADR-029-selfhost-native-verification-contract.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-058](ADR-058-language-v2-semantics-redesign.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#853](../../issues/open/853-ownership-move-copy-mutability.md)

## 文脈

Arukellt v2 は、所有権と値の意味論を Wasm GC の内部表現に依存させない。
現行の型検査器は binding に型だけを保持し、`let mut` の再代入検査、move-after-use、
ユーザー定義型の Copy opt-in を一貫して表現できない。
この状態では、Wasm GC と linear-memory fallback が別の所有権規則へ分岐し、
stdlib と移行対象の診断も安定しない。

## 決定

1. `let` で導入する binding は既定で不変とする。
   `let mut` だけが、その binding 自体への再代入と、mutable place を通した mutation を許可する。
2. 可変性と Copy 性を別の性質として扱う。
   mutable な binding でも値を move でき、immutable な binding でも Copy 値は複製できる。
3. scalar の Copy 性は言語組み込みとする。
   tuple、`Option`、`Result` などの値型は要素がすべて Copy の場合だけ構造的に Copy とする。
   user-defined struct / enum は、`impl Copy for T {}` による明示的 opt-in がない限り Copy ではない。
4. 非 Copy 値を所有権境界で使うと binding を moved とする。
   moved 後の再利用は E0212 として拒否する。Copy 値の利用は元 binding を moved にしない。
5. assignment は target の binding または place が mutable でなければならず、失敗は E0207 とする。
   assignment の可否は RHS の Copy 性や Wasm 表現から推測しない。
6. `Copy` は marker trait として公開し、GC object、linear-memory object のいずれでも同じ型検査結果と
   観測可能な値の意味を持たせる。GC reference の layout や linear-memory の header / tracing は
   backend の責務であり、言語の Copy 判定を変更してはならない。
7. trait、借用、clone、関数引数・closure capture の各境界は、同じ binding state を参照する後続 issue で
   拡張する。実装上まだ移行していない legacy call boundary は current-state と issue に記録し、
   互換のために言語契約を別物へ分岐させない。

## 帰結

- 型検査器は型だけでなく、mutable / Copy / moved の binding metadata を保持する必要がある。
- `Copy` opt-in のない nominal value は、内部表現が参照であっても暗黙に複製されない。
- Wasm GC primary と linear-memory fallback は同じ positive / negative fixture と診断契約を共有できる。
- stdlib と既存 fixture の移行では、暗黙の alias に依存するコードを明示的な clone または設計済みの
  borrow / mutation API へ置き換える必要がある。

## 却下した代替案

- **全ての GC reference を自動 Copy にする。** alias と mutation の意図を失い、linear-memory fallback
  とも異なるため採用しない。
- **`mut` を Copy の代用にする。** 再代入権限と値の複製可能性を混同するため採用しない。
- **Wasm GC と linear memory に別の型検査規則を持たせる。** target-dependent な observable semantics を
  作り、移行と parity 検証を壊すため採用しない。
- **moved 値を runtime trap まで遅延する。** 誤りを compile-time contract から外し、診断位置と移行性を
  不安定にするため採用しない。

## 再検討条件

言語の公開 ABI が Copy の表現を明示的に要求するようになった場合、または ownership boundary の
borrow / clone / closure capture が採択済み契約と衝突した場合に再検討する。
再検討時も backend ごとの別意味論ではなく、共通の型検査契約を先に更新する。
