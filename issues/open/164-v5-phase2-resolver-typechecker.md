# 164: Phase 2 — Resolver + TypeChecker の Arukellt 実装

**Version**: v5 Phase 2
**Priority**: P1
**Depends on**: #163 (Driver + CLI)

## 概要

Arukellt で書かれた名前解決 (Resolver) と型検査 (TypeChecker) を実装する。Phase 2 完了後、Arukellt 版コンパイラで型付き HIR を生成できる。

## タスク

1. `src/compiler/resolver.ark`: 名前解決
   - スコープスタック: `Vec<HashMap<String, Symbol>>`
   - シンボル解決: ローカル変数、関数、型、import
   - エラー: 未定義変数、重複定義の検出
2. `src/compiler/typechecker.ark`: 型検査 + 型推論
   - Union-Find (配列ベース parent 管理)
   - 型ユニフィケーション
   - ジェネリクスのモノモーフィゼーション (最大2型パラメータ)
   - 型エラーのスパン情報付き報告
3. `src/compiler/hir.ark`: HIR データ構造
   - AST からの変換: 名前解決済み + 型付き
4. `src/compiler/mir.ark`: MIR データ構造 + HIR → MIR lowering
   - 制御フローの平坦化
   - MIR 最適化パスの Arukellt 実装 (定数畳み込み、デッドコード除去)

## 完了条件

- Phase 2 のすべてのソースが `arukellt compile` で成功する
- 10 個以上の fixture で Rust 版と同一の型付き HIR を生成する
- 型エラーのあるファイルで適切にエラーを報告する

## 注意事項

- TypeChecker はコンパイラで最も複雑なコンポーネント。arena-style アロケーション (大きな Vec を事前確保 + index 参照) で GC pause を抑制する
- Union-Find は `Vec<i32>` で parent を管理。path compression と union by rank を実装する
