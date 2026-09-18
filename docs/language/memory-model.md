# メモリモデル

> **Normative**: This document defines the authoritative memory-model contract for Arukellt.
> Language semantics follow ADR-059 / ADR-060 and the backend boundary in ADR-002 / ADR-013. Living implementation gaps are in
> [../current-state.md](../current-state.md).

> **正規仕様との関係**: 値型・参照型の分類は [spec.md §2.4 Value vs Reference Semantics](spec.md#24-value-vs-reference-semantics) を参照してください。

## 言語意味論（backend から独立した正本）

Arukellt v2 の source semantics は Wasm GC の内部表現に依存しない。

- `let` binding の move / `Copy` / `let mut` は ADR-059 に従う。
- `Option` / `Result` / enum は owned tagged ADT である。
- `Box<T>` は非 `Copy` の一意所有値、`Slice<T>` は source collection と bounded range を保持する非 `Copy` の読み取り view である（ADR-060）。
- `as_slice` は source binding を消費しない。closure capture の borrow / move / escape は ADR-061 の規則に従う。
- 公開 stable ABI は WIT / Canonical ABI（ADR-006）。GC layout、linear header、tag、collector は compiler-private である。

## ターゲット別の表現（lowering）

| ターゲット | 役割 | 値の表現 |
|-----------|------|----------|
| `wasm32-gc` | **primary**（ADR-013） | Wasm GC `struct` / `array` / `ref` を優先する lowering（移行途中の箇所あり — current-state） |
| `wasm32` | **supported** 互換 | 同一言語意味論の **linear-memory lowering**。tracing GC fallback は段階導入中 |
| `native-cpp` | scaffold | ADR-049のexecutor設計では同一GC意味論をprocess-lifetime arenaへlower。未実装 |
| `native-llvm` | scaffold | 未決定。ADR-049の対象外 |

「設計上の primary」と「ある時点の実装完成度」は分ける:

- **設計**: primary = `wasm32-gc`（GC lowering を優先する backend）
- **実装**: `wasm32-gc` は GC struct/array 等へ段階移行中。`String` / `Vec` / enum 等は
  まだ linear 表現が残る箇所がある（ADR-035 / plans / current-state）
- **互換**: `wasm32` は AtCoder 等向けに linear lowering を維持する

> 📘 ターゲット一覧: [spec.md Appendix B](spec.md#appendix-b-compilation-targets)

## 所有、コピー、view

値の共有や GC reachability は、source-level の `Copy` と同じ概念ではない。
scalar、構造的に `Copy` な tuple / array / `Option` / `Result` は独立した値としてコピーできる。
user-defined struct / enum は明示 `impl Copy` が必要であり、`Box<T>` と `Slice<T>` は非 `Copy` である。
非 `Copy` 値の ownership boundary は move になり、moved binding の再利用は E0212 で拒否する。

`as_slice` は source collection の owner binding を消費せず、backend object と range を保持する読み取り view を作る。
GC object の共有、linear-memory の pointer、将来の tracing collector はこの view の実装手段であり、view API の observable contract を変えない。

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v1: Vec<i32> = Vec_new_i32()
push(v1, 10)

let v2 = v1
push(v2, 20)
```

この種のコードでは、`v1` と `v2` の move / mutation 可否は binding と `Copy` 契約で決まり、
GC reference だから自動的に alias 可能とはみなさない。共有 view が必要な場合は `as_slice` のような明示 API を使う。

## String / Vec（現行実装メモ）

- layout 方針（compiler-private）: [ADR-035](../adr/ADR-035-wasm-gc-implementation.md)
- 実装フェーズ: [plans/wasm-gc-implementation.md](../plans/wasm-gc-implementation.md)
- 現行の到達点・ギャップ: [current-state.md](../current-state.md)

`wasm32` では linear header + data 表現が正である。
`wasm32-gc` では GC array/struct へ移行中であり、「すべてが fully GC-native」とは限らない。
owned ADT、Box、Slice の source-level 契約は両 target で同じである。

## Closure 表現

closures は MIR lowerer により **named functions** としてコンパイルされる。
通常の非 `Copy` capture は shared borrow、body が source を変更する capture は mutable borrow、
`move |...|` は move capture として型検査する。borrow capture を含む closure の return、aggregate 保存、
callback 引数への escape は E0213、borrow 中の source への再代入・再 move は E0214 とする（ADR-061）。
キャプチャはヒープ環境 struct ではなく関数パラメータ経由で渡す（現行 fixture 集合向け）。

- Class A/B の関数値は typed funcref（`(ref $sig)`）+ `ref.func` / `call_ref`
- Class C / 動的ディスパッチは当面 `call_indirect` + table（ADR-033）
- move/capture-free の escaping closure と mutable managed environment は、GC / linear-memory の両方で
  同じ source semantics を保つ environment lowering（#872 / #874）へ移行する。

## 参照先

- [spec.md §2.4](spec.md#24-value-vs-reference-semantics)
- [ADR-002](../adr/ADR-002-memory-model.md) / [ADR-013](../adr/ADR-013-primary-target.md)
- [../current-state.md](../current-state.md)
- [../platform/abi.md](../platform/abi.md)
