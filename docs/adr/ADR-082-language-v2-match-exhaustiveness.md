# ADR-082: 言語 v2 の match 網羅性 — 全 arm 非マッチは compile 時診断

ステータス: **ACCEPTED** — 証明可能な網羅性のない match を型検査で拒否し、非マッチ時の runtime 挙動を言語から排除する

決定日: 2026-09-28

関連: [ADR-058](ADR-058-language-v2-semantics-redesign.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#899](../../issues/open/899-non-catchall-last-arm-unmatched-divergence.md)、[#892](../../issues/done/892-linear-enum-pattern-dispatch-divergence.md)、[#898](../../issues/done/898-or-pattern-last-guarded-arm-compiler-trap.md)、[#852](../../issues/open/852-language-v2-semantics-full-implementation.md)

## 文脈

`match` の全 arm が非マッチのときの挙動はこれまで未規定であり、
backend で発散していた。

- wasm32 linear では last arm が条件なしで実行されるため、非マッチ時に
  最後の arm の body が評価される。
- wasm32-gc では条件付き arm が skip され、match 結果が null ref の
  まま消費されて trap する。

同時に、coverage 記録は variant 名レベルで意味を持たない穴があった。

1. i32 / string など有限でない scrutinee には exhaustiveness 検査自体が
   存在せず、catchall の無い match が受理されていた
   (`match x { 1 => a, 2 => b }`)。
2. guard 付き arm も coverage に寄与していたため、
   `match o { Some(v) if g => a, None => b }` が網羅済みと
   判定され、guard 失敗時に同じ発散へ到達していた。
3. `Some(0)` のような refutable な payload を持つ variant arm が
   variant を「網羅した」と記録されていた
   (`match o { Some(0) => a, None => b }` が `Some(5)` で発散)。
4. or-pattern arm は coverage に何も記録せず、finite 型の
   or-pattern match は事実上常に non-exhaustive 診断になっていた。
5. named 型の scrutinee は enum かどうかに関わらず enum 検査へ
   送られ、struct scrutinee が `unknown enum` で誤って拒否されていた。

## 決定

### 1. 証明可能な網羅性を要求する

`match` 式は次のいずれかを満たさなければ compile 時診断とする。

- **catchall arm を持つ**: guard の無い `_ =>` または binding
  ident arm(`x =>`)が存在する。
- **有限 domain を完全に網羅する**: scrutinee が bool / Option /
  Result / 既知 enum で、要求される全 variant が coverage 済み。
- **全域 irrefutable arm を持つ**: tuple / struct pattern で
  全 sub-pattern が irrefutable(`_` または binding ident)の
  unguarded arm が存在する。

有限でない scrutinee(i32 / i64 / f64 / f32 / char / string / tuple /
Vec / HashMap / fn / resource 等)は literal coverage を証明できない
ため、catchall または全域 irrefutable arm が必須である。

### 2. coverage の精密化

- guard 付き arm は coverage に寄与しない(guard は runtime に
  失敗し得る)。
- variant pattern がその variant を cover するのは、全
  sub-pattern が irrefutable のときに限る。
  `Some(v)` は `Some` を cover するが、`Some(0)` は cover
  しない。
- or-pattern は各 alternative が独立に coverage へ寄与する。
- scrutinee が `TY_UNKNOWN`(推論失敗)の場合は検査を skip し、
  既存の診断へ委ねる(cascade 回避)。
- enum 検査は scrutinee が enum と解決した場合に限る。named 型で
  enum def が見つからないケースは `unknown enum` ではなく
  「catchall が必要」の診断へ落とす。

### 3. 診断

既存の `non-exhaustive match:` 診断ファミリ(`unify::type_error`)
を継続し、「catchall arm が必要」旨のメッセージを追加する。
新しい診断コードは割り当てない。

## 帰結

- `match x { 1 => a, 2 => b }`(i32 scrutinee)は compile 時診断。
- `match o { Some(0) => a, None => b }` は `Some(0)` が
  `Some` を cover しないため compile 時診断。
- `match o { Some(v) if g => a, None => b }` は guard 付き arm が
  coverage に寄与しないため compile 時診断。
- `match p { Point { x, y } => a }`(struct scrutinee)は全域
  irrefutable arm として受理され、従来の `unknown enum` 誤診断を
  解消する。
- 非マッチの runtime 挙動は到達不能になり、backend 間の発散は
  意味論として存在しなくなる(match lowering の差異は残り得るが
  観測不能)。
- multi-clause `fn` は match への desugar として同一規則を
  継承する。

## 却下した代替案

- **非マッチを runtime trap に統一する。** linear 側の last arm を
  条件付きに直し、双方を trap へ揃える案。emit 変更が大きく、
  到達不能なはずの経路へ codegen コストを払うより compile 時に
  排除する方が単純であり、ユーザーにも早期に報告できるため採用
  しない。
- **defined default(暗黙の既定値)を返す。** 根拠のある既定値を
  言語が選べず、バグを隠すため採用しない。
- **full pattern-matrix exhaustiveness を実装する。** ネストした
  tuple / enum / literal の積 coverage を完全に解く案。現行の
  coverage 機構を超える大規模拡張であり、shallow 規則で診断を
  要求する方が KISS/YAGNI に合うため採用しない。必要になれば
  本 ADR の枠組みを拡張して再検討する。

## 再検討条件

- 深い nested pattern の coverage を証明したい需要が出た場合、
  coverage 記録を pattern matrix へ拡張するか再検討する。
- 既存コードで catchall 追加を強いられる量が実害になった場合、
  warning 先行の移行期間を検討する。

## 参照

- [RFC-011](../rfcs/011-language-v2-semantics-redesign.md)
- [docs/language/spec.md](../language/spec.md) §3.11 / §5
