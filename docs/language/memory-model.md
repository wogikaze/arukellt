# メモリモデル

> **Normative**: This document defines the authoritative memory-model contract for Arukellt.
> Language semantics follow ADR-002 / ADR-013. Living implementation gaps are in
> [../current-state.md](../current-state.md).

> **正規仕様との関係**: 値型・参照型の分類は [spec.md §2.4 Value vs Reference Semantics](spec.md#24-value-vs-reference-semantics) を参照してください。

## 言語意味論（設計上の正）

Arukellt の言語意味論は **Wasm GC 前提**である（ADR-002）。

- 所有権 / borrow checker を言語に持ち込まない
- `struct` / `enum` / `String` / `Vec` / closure は GC 管理される参照意味論
- 公開 stable ABI は WIT / Canonical ABI（ADR-006）。GC layout は compiler-private

## ターゲット別の表現（lowering）

| ターゲット | 役割 | 値の表現 |
|-----------|------|----------|
| `wasm32-gc` | **primary**（ADR-013） | Wasm GC references（移行途中の箇所あり — current-state） |
| `wasm32` | **supported** 互換 | 同一言語意味論の **linear-memory lowering** |
| `native-*` | scaffold | 未決定（ADR-045） |

「設計上の primary」と「ある時点の実装完成度」は分ける:

- **設計**: primary = `wasm32-gc`（GC 意味論）
- **実装**: `wasm32-gc` は GC struct/array 等へ段階移行中。`String` / `Vec` / enum 等は
  まだ linear 表現が残る箇所がある（ADR-035 / plans / current-state）
- **互換**: `wasm32` は AtCoder 等向けに linear lowering を維持する

> 📘 ターゲット一覧: [spec.md Appendix B](spec.md#appendix-b-compilation-targets)

## 共有とコピー

値型はコピーされ、参照型は共有される（[spec.md §2.4](spec.md#24-value-vs-reference-semantics)）。
利用者視点では、参照型は共有される前提で考えるのが安全である。

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v1: Vec<i32> = Vec_new_i32()
push(v1, 10)

let v2 = v1
push(v2, 20)
```

この種のコードでは、`v1` と `v2` が同じ実体を共有すると考えるのが現実に近い。

## String / Vec（現行実装メモ）

- layout 方針（compiler-private）: [ADR-035](../adr/ADR-035-wasm-gc-implementation.md)
- 実装フェーズ: [plans/wasm-gc-implementation.md](../plans/wasm-gc-implementation.md)
- 現行の到達点・ギャップ: [current-state.md](../current-state.md)

`wasm32` では linear header + data 表現が正である。
`wasm32-gc` では GC array/struct へ移行中であり、「すべてが fully GC-native」とは限らない。

## Closure 表現

closures は MIR lowerer により **named functions** としてコンパイルされる。
キャプチャはヒープ環境 struct ではなく関数パラメータ経由で渡す（現行 fixture 集合向け）。

- 関数値は `funcref` テーブル index（i32）
- `call_indirect` でディスパッチ
- エスケープするクロージャが必要になった場合は別途環境 struct を検討する

## 参照先

- [spec.md §2.4](spec.md#24-value-vs-reference-semantics)
- [ADR-002](../adr/ADR-002-memory-model.md) / [ADR-013](../adr/ADR-013-primary-target.md)
- [../current-state.md](../current-state.md)
- [../platform/abi.md](../platform/abi.md)
