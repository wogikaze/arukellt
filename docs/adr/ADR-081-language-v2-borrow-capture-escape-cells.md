# ADR-081: 言語 v2 の mutable-borrow capture escape — promoted shared cell

ステータス: **ACCEPTED** — rebind capture は source place を共有 heap cell に昇格し、env が live alias を運ぶことで escape を許す

決定日: 2026-09-28

関連: [ADR-061](ADR-061-language-v2-closure-capture-lifetime.md)、[ADR-075](ADR-075-language-v2-linear-memory-tracing-runtime.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#891](../../issues/open/891-borrow-capture-escape-env-cells.md)、[#872](../../issues/open/872-language-v2-closure-pair-env.md)

## 文脈

ADR-061 §2 は borrow capture を含む closure の escape を一律 E0213 で拒否した。
一方 §3 は「environment が source mutation を保持できるなら mutable borrow も escape できる」ことを設計意図として残し、再検討条件に「両 backend で同一の観測可能意味論を fixture と receipt で示すこと」を要求していた。

#872 / #874 で managed env record と rebind capture の env cell / write-back は実装済みだったが、non-move closure の env field は source の snapshot であり、escape した closure の write が source に届かなかった。
ADR-061 の fallback はその誠実な制限として維持されていた。

## 決定

### 1. promoted shared cell

- non-move closure が capture source の place を rebind する場合、capture point でその place を共有 heap cell に昇格する。
  wasm32-gc では 1 フィールドの GC struct、wasm32 では同じ slot 幅の linear-memory cell として下ろす。
- 昇格した place への以後の read/write は cell field 0 への access とする。
  closure env は cell ref を capture field として保持するため、env は snapshot ではなく **live alias** を運ぶ。
- direct call と env trampoline(indirect call)のどちらの経路でも同じ cell ref が cap argument として渡り、closure body の write は parent frame と同じ heap place を変更する(write-through)。
- source binding がすでに cell-backed の場合(synth の capture shadow を nested closure が再捕捉する場合)、新しい cell を作らず既存の cell ref を capture supply として再利用する。
  write-through 連鎖は同一 heap cell を共有する。

### 2. ADR-061 §2/§3 の絞り込み

- mutable-borrow(rebind)capture を含む closure は escape できる。
  escape 境界(return の返却値、callback 引数、aggregate 保存)の規則自体は変更しない。
- E0213 は「env が source mutation を保持できない経路」に限り発火する。
  shared borrow capture が残る唯一の発火経路であり、引き続き拒否する。
- mutable borrow の排他性(shared borrow との共存禁止、同一 source への別 mutable borrow の禁止)、borrow 中の source への代入禁止(E0214)、move-after-capture(E0212)は変更しない。
  parent が capture 後に source を書き換える経路は従来どおりの診断である。

### 3. 検証契約

write-through の観測可能意味論は両 backend で同一であり、closure_capture fixture(escape 経路別: callback 引数・return・aggregate 保存)と cross-backend parity gate がその receipt である。

## 帰結

- non-move rebind capture の write-back copy 経路は promoted cell へ統合され、直接呼び出し・escape を問わず単一の write-through 意味論になる。
- shared borrow capture の escape(read-only alias を env が運ぶ拡張)は別の設計判断であり、本 ADR では扱わない。
- 関数引数など let 以外の capture source も同じ機構で昇格できるが、immutable binding は rebind capture を構成しないため実際の発火経路は let mut と同型である。

## 却下した代替案

- **cap param を cell 型のまま body を下ろす。** MIR の演算 dispatch・返り値解決が local の宣言型を値型として参照するため、cell 型が + の string concat 判定や closure 返り値型へ漏れて破綻する。payload 型 shadow + emit 段 rewrite を採用した。
- **function-level 事前解析で source binding を最初から cell として下ろす。** capture point で現在値を包む昇格で十分であり、capture を持たない経路の local 表現を変えない利点があるため採用しない。
- **non-move rebind capture の env-cell + write-back copy を維持する。** env field が snapshot のままでは escape した closure の write が source に届かず、live alias が本質的に必要なため採用しない。
- **mutable borrow capture も move と同じ所有権移動とみなす。** source binding が生存したまま共有される意味論が mutable borrow の定義であり、消費すると borrow conflict 規則と矛盾するため採用しない。

## 再検討条件

- shared borrow capture を env が read-only alias として運べるようになった場合、E0213 の残存経路を再評価する。
- borrow の有効範囲を enclosing function より精密に短縮する liveness 解析を導入する場合、昇格 cell の寿命契約を再検討する。
- 外側の shared borrow 経由で内側 closure が mutable borrow を構成する nested reborrow を診断へ変える場合、昇格点の typechecker 契約を再検討する(現行は snapshot cell として観測可能に振る舞う)。

## 参照

- [RFC-011 §4.1](../rfcs/011-language-v2-semantics-redesign.md)
- [docs/language/spec.md](../language/spec.md)
- [docs/language/memory-model.md](../language/memory-model.md)
