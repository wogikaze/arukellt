---
Depends on: なし
Priority: P2
Track: main
Orchestration class: implementation-ready
# 168: CoreHIR / MIR 仕様ドキュメント
---
# 168: CoreHIR / MIR 仕様ドキュメント

## 概要

`docs/compiler/ir-spec.md` に CoreHIR と MIR のデータ構造定義、フェーズ間契約、不変条件を記述する。セルフホスト実装の設計仕様書として使用する。

## タスク

1. CoreHIR の全ノード型の定義と意味論
2. MIR の全ステートメント・オペランドの定義
3. HIR → MIR lowering のルール (各 HIR ノードがどの MIR に変換されるか)
4. MIR 最適化パスの仕様 (入力条件、出力保証、適用順序)
5. MIR → Wasm の対応表

## 完了条件

- `docs/compiler/ir-spec.md` が存在する
- Arukellt 版コンパイラの Resolver/TypeChecker/Emitter 実装者がこのドキュメントだけで IR を再実装できる

## 備考

既存の `crates/ark-mir/src/mir.rs` と `crates/ark-mir/src/lower/` のコードから仕様を抽出する。