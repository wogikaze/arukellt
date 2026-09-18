# ADR-061: 言語 v2 のクロージャ捕捉・借用・escape

ステータス: **ACCEPTED** — クロージャの shared borrow、mutable borrow、move capture と escape 検査を共通意味論として採択する

決定日: 2026-09-19

関連: [ADR-002](ADR-002-memory-model.md)、[ADR-033](ADR-033-call-ref-hof-migration.md)、[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、[ADR-060](ADR-060-language-v2-owned-adts-box-slice.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#855](../../issues/open/855-closure-capture-lifetime.md)

## 文脈

ADR-059 と ADR-060 は binding の move / `Copy`、owned value、`Box<T>`、`Slice<T>` の値境界を固定した。
しかし、クロージャが外側の binding を読むときに、その値を借用するのか所有移動するのか、またクロージャを返却・保存・callback に渡せるのかは未定義だった。

現在の lambda lifting は capture を合成関数の追加引数として表現する。
この実装表現だけから「GC reference は共有される」「capture は自動的に Copy である」と推論すると、`wasm32-gc` と `wasm32` で異なる所有権意味論を作ってしまう。

## 決定

### 1. Capture mode

- 通常の `|params| body` は、`Copy` binding を shared-copy capture として捕捉する。
  元の binding は消費しない。
- 通常の `|params| body` は、非 `Copy` binding を shared borrow capture として捕捉する。
  借用中の binding は move または mutable borrow の対象にできない。
- `move |params| body` は、非 `Copy` binding を move capture として捕捉する。
  作成時に元の binding を moved とし、capture の所有者をクロージャへ移す。
  `Copy` binding は `move` が付いていても ADR-059 の Copy 規則に従い消費しない。
- closure body が capture source の place を変更する場合、その capture は mutable borrow とする。
  mutable borrow は shared borrow と同時に存在できず、同じ source に対する別の mutable borrow も許可しない。
- closure parameter や closure body 内で新たに導入した local は capture source ではない。
  nested closure は内側の closure 自身が capture を検査し、外側の closure の capture 集合へ暗黙に昇格しない。

### 2. Lifetime と escape

- borrow capture の有効範囲は、その capture を作った enclosing function 内である。
  borrow は source binding より長く生存してはならない。
- `return` の返却値、関数呼び出しの callback 引数、aggregate の要素・field への保存は、closure value の escape とみなす。
- borrow capture を含む closure が escape 境界へ到達した場合、capture identifier の span を指す E0213 で拒否する。
- move capture または capture を持たない closure は、通常の function value と同じく escape できる。
- closure value を同じ enclosing function の local binding に束縛すること自体は escape ではない。
  drop/liveness による borrow の短縮は、後続の lifetime 精密化で追加する。

### 3. Backend independence

- capture mode、move 状態、borrow conflict、escape 診断は target-independent な typechecker 契約とする。
- `wasm32-gc` の managed reference、`wasm32` の linear-memory value、lambda lifting の追加引数は backend 表現であり、capture mode を変更してはならない。
- mutable borrow を保持する managed closure environment と、その mutation の GC / linear-memory lowering は #872 / #874 の責務である。
  environment が source mutation を保持できない経路では、mutable borrow を escape させず compile-time error とする。

## 帰結

- `move` は closure prefix の contextual syntax として予約し、既存の `move` という identifier の使用は closure prefix 以外で維持する。
- typechecker の binding state は moved に加えて shared borrow / mutable borrow を保持する。
- closure binding を callback として渡す場合、callee が保持する可能性を排除できないため、borrow capture は conservative に拒否する。
- GC の object representation を理由に非 `Copy` capture を複製してはならない。

## 却下した代替案

- **すべての capture を move にする。** source の読み取りだけをした closure まで binding を消費し、shared borrow の意味を表現できないため採用しない。
- **すべての GC reference を Copy capture にする。** linear-memory fallback と異なる所有権を作り、ADR-059 の owned value 契約を壊すため採用しない。
- **`&x` / `&mut x` の capture list を先に導入する。** 現行 closure grammar に capture list の構文・型・desugar 境界がなく、#855 の lifetime 契約を必要以上に拡張するため採用しない。
- **callback は常に同期呼び出しと仮定する。** callee が closure を保存する可能性を型から判定できず、短命 borrow の escape を見逃すため採用しない。

## 再検討条件

次のいずれかが発生した場合に再検討する。

- closure environment の ownership と mutation が GC / linear-memory の両 backend で同一の観測可能意味論を持つことを fixture と検証 receipt で示せる。
- lifetime liveness により borrow の有効範囲を enclosing function より精密に短縮できる。
- capture list、`&T`、`&mut T` を公開構文として導入する必要が生じる。

## 参照

- [RFC-011 §4.1](../rfcs/011-language-v2-semantics-redesign.md)
- [docs/language/spec.md](../language/spec.md)
- [docs/language/memory-model.md](../language/memory-model.md)
