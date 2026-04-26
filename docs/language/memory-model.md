# メモリモデル

> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> Behavior described here is verified by the fixture harness. Changes require spec review.
> For current verified state, see [../current-state.md](../current-state.md).

このページは、現行実装のメモリモデルを先に示し、そのあとに設計意図を短く残します。

> **正規仕様との関係**: 値型・参照型の分類は [spec.md §2.4 Value vs Reference Semantics](spec.md#24-value-vs-reference-semantics) を参照してください。
> このページでは §2.4 の分類に基づく実装上の振る舞いを補足しています。

## 現行実装

Arukellt の現在の production path は **linear memory + bump allocator** ベースです。

- `struct`
- `enum`
- `String`
- `Vec`
- `closure`

は、現行実装では linear memory 上の表現を使います。

### 現在の前提

- T1 (`wasm32-wasi-p1`) が production path
- 実装基盤は linear memory
- no GC runtime in production
- allocator は bump allocator
- 一部の wrapper / intrinsic がランタイム表現を隠している

> 📘 コンパイルターゲットの一覧は [spec.md Appendix B](spec.md#appendix-b-compilation-targets) を参照。

## 共有とコピー

値型はコピーされ、参照型は共有されます ([spec.md §2.4](spec.md#24-value-vs-reference-semantics))。

利用者視点では、参照型は共有される前提で考えるのが安全です。

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v1: Vec<i32> = Vec_new_i32()
push(v1, 10)

let v2 = v1
push(v2, 20)
```

この種のコードでは、`v1` と `v2` が同じ実体を共有すると考えるのが現実に近いです。

## String / Vec の実装 reality

- `String` は linear memory 上の文字列表現
- `Vec` は header + data region の線形メモリ表現
- 古い Wasm GC struct/array の説明は **設計資料** として読むべきです

## 将来設計

ADR や過去の設計文書では、Wasm GC ベースの表現を採っていました。
それらは「なぜその方向を検討していたか」を知るには有用ですが、現行コードの source of truth ではありません。

## 参照先

- [spec.md §2.4](spec.md#24-value-vs-reference-semantics) — 値型・参照型の正規分類
- 現在の実装: [../current-state.md](../current-state.md)
- stdlib 現況: [../stdlib/README.md](../stdlib/README.md)
- ABI 方針: [../platform/abi.md](../platform/abi.md)

## T3 closure representation

In both T1 and T3, closures are compiled as **named functions** by the MIR lowerer.
Captured variables are passed through function parameters rather than a heap-allocated
environment struct. This means:

- No runtime closure allocation or GC pressure
- Captured values follow the same copy/share semantics as regular assignments
- Function values are represented as `funcref` table indices (i32)
- `call_indirect` dispatches through the function table

This design is sufficient for the current fixture set. A heap-allocated environment
struct would be needed if closures could escape their defining scope (e.g., returned
from a function), but this is not yet required.
