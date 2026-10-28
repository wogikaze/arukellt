# メモリモデル

> **Normative**: This document defines the authoritative memory-model contract for Arukellt.
> Language semantics follow ADR-059 / ADR-060 and the backend boundary in ADR-002 / ADR-013. Living implementation gaps are in
> [../current-state.md](../current-state.md).

> **正規仕様との関係**: 値型・参照型の分類は [spec.md §2.4 Value vs Reference Semantics](spec.md#24-value-vs-reference-semantics) を参照してください。

## 言語意味論（backend から独立した正本）

Arukellt v2 の source semantics は Wasm GC の内部表現に依存しない。

- `let` binding の move / `Copy` / `let mut` は ADR-059 に従う。
- 関数呼び出しは非 `Copy` 引数を move せず、呼び出しの間だけ借用する。呼び出しをまたぐ書き換えは `mut` 仮引数と可変 place の組でだけ許可する（ADR-088）。
- `Option` / `Result` / enum は owned tagged ADT である。
- `Box<T>` は非 `Copy` の一意所有値、`Slice<T>` は source collection と bounded range を保持する非 `Copy` の読み取り view である（ADR-060）。
- `as_slice` は source binding を消費しない。closure capture の borrow / move / escape は ADR-061 の規則に従う。
- 公開 stable ABI は WIT / Canonical ABI（ADR-006）。GC layout、linear header、tag、collector は compiler-private である。

## ターゲット別の表現（lowering）

| ターゲット | 役割 | 値の表現 |
|-----------|------|----------|
| `wasm32-gc` | **primary**（ADR-013） | Wasm GC `struct` / `array` / `ref` を優先する lowering（移行途中の箇所あり — current-state） |
| `wasm32` | **supported** 互換 | 同一言語意味論の **linear-memory lowering**。tracing GC fallback は段階導入中 |
| `native-cpp` | scaffold | 同一 GC 意味論を process-lifetime arena へ lower する（ADR-049）。experimental な `run --target native-cpp` を提供（ADR-050）。実装は partial |
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

<!-- fixture: spec_examples/memory_model_move.ark -->
```ark
let mut v1: Vec<i32> = Vec::new<i32>()
v1.push(10)

let mut v2 = v1
v2.push(20)
```

この種のコードでは、`v1` と `v2` の move / mutation 可否は binding と `Copy` 契約で決まり、
GC reference だから自動的に alias 可能とはみなさない。`let mut v2 = v1` で `v1` は move され、以後は使えない。
共有 view が必要な場合は `as_slice` のような明示 API を使う。

## 関数呼び出し境界（ADR-088）

関数呼び出しは所有権の境界ではない。非 `Copy` の実引数は呼び出しの間だけ借用され、caller の binding は move されない。

```text
fn total(v: Vec<i32>) -> i32 { … }      // 共有仮引数: v を通した書き換えは不可
fn fill(mut v: Vec<i32>) { v.push(1) }  // mut 仮引数: 書き換えは caller から見える

let xs = Vec::new<i32>()
total(xs)              // OK: 借用のみ。xs は引き続き使える
let mut ys = Vec::new<i32>()
fill(ys)               // OK: ys は可変 place
fill(xs)               // error: xs は不変 binding
```

- 共有仮引数 `p: T` と receiver `self: T` からは書き換えられない。書き換えるには `mut p: T` / `mut self: T` を宣言する。
- `mut` 仮引数へ渡せるのは、`let mut` binding、`mut` 仮引数、それらを根とする field / element place、一時値だけである。
- 同じ place を一つの呼び出しで `mut` 仮引数とほかの仮引数へ同時に渡すことはできない。
- callee が借用した値を返却・保存すると、その値は複製されず GC reference として共有される。
  言語が保証するのは「不変 binding と共有仮引数を根とする経路からは書き換えが起きない」ことであり、
  別名から `mut` 経路で書き換えた結果は元の binding からも見える。この観測結果は全 target で同一である。
- `Box<T>` は一意所有のため共有 escape の対象外で、借用した `Box<T>` 仮引数は返却・保存・`unbox` できない。

**実装状態**: コンパイラは `mut` 仮引数、`mut self`、`|mut v|`、関数型の
`mut` 位置（`fn(mut T) -> U`）を解析し、`mut` 仮引数へ渡す実引数が可変 place で
あることを呼び出し境界で検査する（E0223）。同じ呼び出しで同一根の place を
`mut` 仮引数と他の仮引数へ渡すことは E0224 とする。callee 側では共有仮引数と
共有 `self` を不変 binding として扱い、そこからの代入を E0226 で拒否する。
mode は `TY_FN` 型に含まれ、関数値経由の呼び出しにも同じ規則が適用される。
共有仮引数の関数値を `mut` 位置へ代入できる一方、逆方向は注釈付き `let`
binding、関数引数、return 位置で E0225 として拒否する。module 修飾された
関数 path も同じ検査の対象である。借用した `Box<T>` 仮引数の返却・集約値への
格納・`unbox` は E0227 として拒否する。`std/`、`src/compiler/`、fixture は
`mut` 仮引数と `let mut` place へ移行済みである。移動済み binding を実引数に
渡す E0212 の一部は [#944](../../issues/open/944-move-after-use-not-diagnosed.md)
で追跡する。規則の詳細は
[spec.md §3.5.1](spec.md#351-argument-passing-and-parameter-modes)、実装は
[#951](../../issues/open/951-mut-parameter-mode-and-call-borrow-checks.md) で追跡する。

## String / Vec（現行実装メモ）

- layout 方針（compiler-private）: [ADR-035](../adr/ADR-035-wasm-gc-implementation.md)
- 実装フェーズ: [plans/wasm-gc-implementation.md](../plans/wasm-gc-implementation.md)
- 現行の到達点・ギャップ: [current-state.md](../current-state.md)

`wasm32` では linear header + data 表現が正である。
`wasm32-gc` では GC array/struct へ移行中であり、「すべてが fully GC-native」とは限らない。
owned ADT、Box、Slice の source-level 契約は両 target で同じである。

## Closure 表現

通常の非 `Copy` capture は shared borrow、body が source を変更する capture は mutable borrow、
`move |...|` は move capture として型検査する。shared-borrow capture を含む closure の return、aggregate 保存、
callback 引数への escape は E0213、borrow 中の source への再代入・再 move は E0214 とする（ADR-061）。
mutable-borrow(rebind)capture は source place を共有 heap cell に昇格し、env が live alias を運ぶため
escape できる（ADR-081）。nested closure は中間 frame を transport slot として経由して外側の名前を捕捉する（ADR-083）。

表現（compiler-private）:

- escape しない closure は MIR lowerer が **named function** へ持ち上げ、capture を追加引数として渡す。
- move / `Copy` capture を持つ escaping closure と mutable-borrow capture は、managed な `{fnptr, env}` pair で
  表現し、env が capture ごとの field または昇格した共有 cell を保持する（#872 / #891）。
- 関数値は typed funcref（`(ref $sig)`）+ `ref.func` / `call_ref` を使い、動的ディスパッチは `call_indirect` + table（ADR-033）。
- 観測可能な capture の意味は `wasm32-gc` と `wasm32` で同一である。

## 参照先

- [spec.md §2.4](spec.md#24-value-vs-reference-semantics)
- [ADR-002](../adr/ADR-002-memory-model.md) / [ADR-013](../adr/ADR-013-primary-target.md)
- [../current-state.md](../current-state.md)
- [../platform/abi.md](../platform/abi.md)
