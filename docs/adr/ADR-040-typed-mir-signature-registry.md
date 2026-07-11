# ADR-040: Semantic Type Spine

ステータス: **ACCEPTED** — Semantic Type Spine（SignatureRegistry / MonoInstanceTable）を MIR の正本とする

決定日: 2026-07-03  
改訂日: 2026-07-11 — 詳細設計を RFC-002 へ分離

---

## 文脈

`wasm32-gc` の残存 validate-fail は局所修正の限界に出ている。根本原因は個別の型推論バグではなく、
コンパイルパイプライン各段で意味情報（型・シグネチャ・ABI）が失われ、emitter が
名前やスタックから型を掘り返していることである。

設計原則: **emitter を賢くするのではなく、emitter を馬鹿にできるようにする。**

---

## 決定

1. **FunctionId・TypeId・SignatureRegistry・MonoInstanceTable を semantic spine とする。**
   MIR 以降の正本はこれらであり、emitter は Typed MIR を機械的に Wasm へ変換するだけにする。
2. **型を 3 層に分離する**: `TypeId`（意味型）/ `MirValueType`（内部値型 + nullability）/
   `WasmValueType`（出力型）。nullability は TypeId に混ぜない。
3. **型不明時は fallback せず internal compiler error** とする（考古学的推論を廃止する方向）。
4. **構造体定義・不変条件・詳細 API** は RFC に置き、**移行フェーズ**は plan に置く。

詳細: [`docs/rfcs/002-semantic-type-spine.md`](../rfcs/002-semantic-type-spine.md)  
計画: [`docs/plans/typed-mir-signature-registry.md`](../plans/typed-mir-signature-registry.md)

---

## 関連

- [RFC-002: Semantic Type Spine](../rfcs/002-semantic-type-spine.md)
- [実装計画](../plans/typed-mir-signature-registry.md)
- [ADR-042](ADR-042-intrinsic-layer-separation.md) — intrinsic / semantic 層
- [ADR-006](ADR-006-abi-policy.md) — compiler-private layout
