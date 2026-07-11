# ADR-035: Wasm GC 内部レイアウト方針

ステータス: **PROPOSED** — `wasm32-gc` 上の compiler-private GC 表現方針を提案

決定日: 2026-06-17  
改訂日: 2026-07-11 — 段階移行計画を plans へ委譲し、layout 方針のみ残す

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
| enum / `Option` / `Result` | GC struct（tag + payload fields）または payload 用 array | すべてを linear tagged union に戻す（GC 意味論と乖離） |

補足:

- nullable は Wasm の `ref null` を用いる
- 具体的な type index・field 順序・inline 最適化は emitter 私有で変更してよい
- 公開相互運用は WIT / Canonical ABI（ADR-006）経由

## スコープ外（他 ADR / plan）

- primary / supported ターゲット選定（ADR-013 / ADR-007）
- `Weak<T>` / finalizer（未採択、ADR-043）
- native ABI（ADR-045）
- 実装フェーズ・検証ゲート・現行通過数（plan / current-state）

## ブラウザ経路（jco）との関係

製品経路は ADR-007 / ADR-017。jco の検証状態は
[`docs/research/target-runtime-verification.md`](../research/target-runtime-verification.md)
（Node E2E 済み、Chrome jco component E2E は未検証）。本 ADR は layout のみを扱う。

## 関連

- [ADR-002](ADR-002-memory-model.md) — GC 意味論
- [ADR-006](ADR-006-abi-policy.md) — layout は compiler-private
- [ADR-007](ADR-007-targets.md) / [ADR-013](ADR-013-primary-target.md)
- [ADR-043](ADR-043-wasm-gc-post-mvp.md)
- [`docs/plans/wasm-gc-implementation.md`](../plans/wasm-gc-implementation.md)
