# ADR-035: Wasm GC 段階移行方針

ステータス: **PROPOSED** — `wasm32-gc` 向け Wasm GC 実装の段階的移行方針を提案

決定日: 2026-06-17  
改訂日: 2026-07-11 — タイトル整理、現行テスト数を current-state へ委譲

---

## 文脈

ADR-002 (Memory Model, 2026-03-25) は **選択肢 A: Wasm GC 前提** を採用した。
歴史的な Rust プロトタイプは GC 命令を出力していた。selfhost 移行以降は
線形メモリ + bump アロケータを使用していたが、GC target (`wasm32-gc`) では
GC 命令基盤、GC struct/array、文字列/Vec の GC 表現を段階的に実装する。
`wasm32` は線形メモリを維持する。現行の fixture / テスト通過数は
`docs/current-state.md` を正本とする。

ADR-007 (Targets) は以下のメモリモデルを定義している：

| ターゲット | メモリモデル |
|------------|-------------|
| `wasm32` | Linear memory |
| `wasm32-gc` | **Linear memory + Wasm GC** |
| `native-cpp` / `native-llvm` | LLVM/C++ 依存 |

selfhost エミッタには GC 命令基盤と struct/array 発行の target dispatch を追加する。
GC ターゲットは reference local/type encoding と `struct.*` / `array.*` 命令を出力し、
`wasm-tools validate --features gc` を通すことを目標とする。MIR/CoreHIR には
aggregate reference 用の `VT_GC_REF` tag を追加し、GC 型は function signatures より前に
emitted する。

## 提案する決定

1. **`wasm32-gc` を GC-native ターゲットとする。** 値表現を `i32-as-pointer` から
   GC reference type へ段階的に移行する。
2. **`wasm32` は線形メモリパスを維持する。** ADR-002 で「両対応」は拒否されたが、
   AtCoder 等の既存ターゲット維持のため linear memory 実装は残す。
3. **文字列は `(ref null (array (mut i8)))`、Vec/Enum は GC struct/array 表現とする**
   （ADR-002 合意済みの表現方針に従う）。
4. **完了基準は `wasm32-gc` で既存フィクスチャスイートが全通過すること。**
5. **実装フェーズ・検証手順は [`docs/plans/wasm-gc-implementation.md`](../plans/wasm-gc-implementation.md) に置く。**

## スコープ外

- Post-MVP GC features（ADR-043）: static fields、weak references、generics。
  `Weak<T>` / finalizer は言語未採択（ADR-002 / ADR-043）
- LLVM backend (`native-llvm`): native target は別トラック（ADR-045）
- WASI P3 async-first: 仕様未確定のため defer

## ブラウザ経路（jco）との関係

`wasm32-gc` → in-tree component → `jco transpile` → browser/Node は ADR-007 / ADR-017 の
製品経路である。jco の Wasm GC 対応は調査時点で確認済み
（[`docs/research/target-runtime-verification.md`](../research/target-runtime-verification.md)、
旧 #037 は解消済み）。本 ADR は「jco GC 待ち」をブロッカーとしない。

継続的な conformance（GC struct/array を含む component の jco + Chrome/Node）は
`docs/current-state.md` / CI で追跡する。

## リスク

1. **wasmtime GC perf**: fixture parity と benchmark で監視する。
2. **Migration cost**: MIR lowering の広範な影響。段階的移行が難しい場合、
   「flag day」アプローチも検討。
3. **`wasm32` / `wasm32-gc` の二重 lowering コスト**: 意味論は単一、表現だけが分岐する
   （ADR-002）。コードベース複雑さは残る。

## 関連 ADR

- [ADR-002: Memory Model](ADR-002-memory-model.md) — GC-native 決定の根拠
- [ADR-007: Targets](ADR-007-targets.md) — ターゲット定義
- [ADR-043: Wasm GC Post-MVP](ADR-043-wasm-gc-post-mvp.md) — Phase 0 拡張を言語前提にしない
- [ADR-013: Primary Target](ADR-013-primary-target.md) — `wasm32-gc` primary 根拠
- [ADR-040: Semantic Type Spine](ADR-040-typed-mir-signature-registry.md) — GcLayoutTable 基盤
- [実装計画: Wasm GC](../plans/wasm-gc-implementation.md)
