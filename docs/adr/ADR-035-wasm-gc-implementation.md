# ADR-035: Wasm GC 内部レイアウト方針

ステータス: **PROPOSED** — `wasm32-gc` 上の compiler-private GC 表現方針を提案

提案日: 2026-06-17  
改訂日: 2026-07-18 — 型 identity、enum layout、host 境界の判断候補を明確化

---

## 文脈

ADR-002 / ADR-007 / ADR-013 は次を既に決定している:

- 言語意味論は Wasm GC 前提
- primary ターゲットは `wasm32-gc`
- `wasm32` は同一意味論の linear lowering（supported）
- GC 型 layout は **compiler-private**（ADR-006）であり stable ABI ではない

本 ADR が新たに固定するのは、**emitter が選ぶ内部 GC 表現の方針**だけである。
実装フェーズ・fixture 完了条件・移行手順は
[`docs/plans/wasm-gc-implementation.md`](../plans/wasm-gc-implementation.md) を正本とする。

---

## 提案する決定

`wasm32-gc` の compiler-private layout は次を既定とする（いずれも stable ABI ではない）:

| 言語型 | 既定の Wasm GC 表現 | 却下した案（要約） |
|--------|---------------------|-------------------|
| `String`（UTF-8） | `(ref null (array (mut i8)))` | UTF-16 array（WASI/JS 境界コスト増）; rope を言語必須にしない |
| `Vec[T]` | GC struct（len/cap + `(ref null (array (mut T')))`） | 単一 flat array のみ（grow 時の再配置契約が曖昧） |
| enum / `Option` / `Result` | 共通 base と variant subtype からなる GC struct（tag + 型付き payload fields） | すべてを linear tagged union に戻す（GC 意味論と乖離）; payload を untyped array に格納する（型検査を emitter へ先送りする） |

補足:

- nullable は Wasm の `ref null` を用いる
- 同じ具象 `TypeId` は 1 core module 内でただ 1 つの GC type family に対応させる。
  nullability は利用箇所の `MirValueType` の属性であり、別の GC type identity を作らない
- enum variant は共通 base の明示的 subtype とし、payload は
  `GcLayoutTable` が決めた exact `WasmValueType` で保持する。linear-memory address や
  整数への pack を GC ref の代用にしない
- 具体的な type index・field 順序・inline 最適化は emitter 私有で変更してよい
- 公開相互運用は WIT / Canonical ABI（ADR-006）経由
- Memory64 と host/component 側 canonical memory のアドレス幅変換は、型付きの
  canonical ABI adapter が所有する。通常の MIR call site へ無検査の truncate を散在させない

詳細な不変条件と境界設計は
[`RFC-007`](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md) に置く。

## スコープ外（他 ADR / plan）

- primary / supported ターゲット選定（ADR-013 / ADR-007）
- `Weak<T>` / finalizer（未採択、ADR-043）
- native ABI（ADR-045）
- 実装フェーズ・検証ゲート・現行通過数（plan / current-state）

## 代替案

### call site ごとの `i32.wrap_i64`

却下する。validation error は局所的に消せるが、範囲外アドレスを黙って truncate し、
WASI P2 の interface / resource semantics を pseudo core import に固定してしまう。
幅変換が必要な場合は canonical ABI adapter で検査して行う。

### 名前や固定 offset による GC type index 復元

却下する。同じ意味型が別 type index へ分裂し、関数 signature、local、constructor の
identity が一致しない。`TypeId` から構築した module-wide layout plan を唯一の owner とする。

### enum payload の linear-memory 併用を恒久化

却下する。payload が ref か scalar かを call site で再推論する必要が生じ、ADR-040 の
Semantic Type Spine と衝突する。移行中の linear 表現は現行実装ギャップとしてのみ扱う。

## ブラウザ経路（jco）との関係

製品経路は ADR-007 / ADR-017。jco の検証状態は
[`docs/research/target-runtime-verification.md`](../research/target-runtime-verification.md)
（Node E2E 済み、Chrome jco component E2E は未検証）。本 ADR は layout のみを扱う。

## 関連

- [ADR-002](ADR-002-memory-model.md) — GC 意味論
- [ADR-006](ADR-006-abi-policy.md) — layout は compiler-private
- [ADR-007](ADR-007-targets.md) / [ADR-013](ADR-013-primary-target.md)
- [ADR-043](ADR-043-wasm-gc-post-mvp.md)
- [ADR-040](ADR-040-typed-mir-signature-registry.md) — Semantic Type Spine
- [RFC-007](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md) — 詳細設計
- [`docs/plans/wasm-gc-implementation.md`](../plans/wasm-gc-implementation.md)
