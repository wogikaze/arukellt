# v0 Canonical Surface - Freeze Ready

**Date**: 2026-03-24  
**Status**: Ready for freeze

## Summary

arukellt v0 の canonical surface は LLM 向けに十分整理され、freeze 可能な状態に達しました。

## Design Principles (from search.md)

v0 は以下の原則に基づいて設計:

1. **表記の正規形を少なくする**
   - 同じ操作に複数の書き方を持たない
   - API スタイルを統一（全て裸関数、メソッド構文なし）

2. **局所推論 + 境界明示**
   - 型は局所的に推論可能
   - v0/v1 境界を明確に分離

3. **少数の核 API に集中**
   - Prelude: Option/Result/Vec/String + 基本操作
   - stdlib: 拡張操作（import 必要）
   - capability: I/O（main 引数経由）

4. **生成→型検査→実行→自己修正ループの安定化**
   - 診断は今後設計（構造化、fix-it hint）

## v0 Core Features

### 構文
- ✅ brace 構文（`{ }` ベース）
- ✅ trait なし
- ✅ for なし
- ✅ メソッド構文なし（v1）
- ✅ 演算子オーバーロードなし
- ✅ `Result<T, E>` 中心のエラー処理
- ✅ `?` は v0 では型一致時のみ（自動変換なし）

### 型システム
- ✅ Wasm GC 前提
- ✅ 参照型: struct/enum/String/Vec/[T] は GC heap 上の参照
- ✅ 代入・引数渡し = 参照コピー（オブジェクト共有）
- ✅ mutation: Vec のみ in-place 変更可能（struct は immutable）
- ✅ generic: `<T>` 記法、ネスト禁止

### API
- ✅ Prelude: Option/Result/String/Vec, Some/None/Ok/Err, len/clone/unwrap/panic
- ✅ Vec 操作: vec_new/vec_push/vec_pop/vec_get（全て裸関数）
- ✅ String: concat/string_append_char（不変、新値を返す）
- ✅ I/O: io.Caps 経由のみ（capability-based）

### Documentation
- ✅ v1 機能を syntax-v1-preview.md に分離
- ✅ 文書間の矛盾を解消
- ✅ コードサンプルは全て型検査可能

## Completed Canonicalization

### Phase 1: 構文レベルの矛盾解消
- impl/メソッド構文を v0 から削除
- generics 記法を `<T>` に統一
- 文字列リテラル型を String に統一
- v1 機能を別ファイルに分離

### Phase 2: API/セマンティクス境界の固定化
- clone を v0 正式採用（shallow clone）
- [] を固定長配列専用に固定
- unwrap を Prelude に追加
- API スタイルを裸関数に統一
- mutation boundary を明文化（Vec のみ）
- generic ネスト禁止を強調

### Phase 3: 最終磨き（freeze 前）
- clone 説明を 1 文に統一
- [] 露出を削減（vec_new() 推奨）
- typo 修正

## What's NOT in v0 (v1 Preview)

- `impl` ブロック・メソッド構文
- trait・演算子オーバーロード
- `for` ループ・イテレータ
- match の guard/or-pattern/struct pattern
- `?` のエラー型自動変換
- struct field update

## Next Steps (Post-Freeze)

1. **診断設計**
   - 構造化診断メッセージ
   - expected/actual 表示
   - fix-it hint
   - 生成→型検査→自己修正ループの実装

2. **stdlib 完全性**
   - v0 で必要な Vec 操作の全列挙
   - String 操作の正規形確定

3. **実装同期**
   - コンパイラで v0 制限を強制
   - 禁止構文に対する明確なエラーメッセージ

## Conclusion

v0 は「Python 風」ではなく「LLM が安定生成・修正しやすい canonical surface を持つ
Wasm GC 前提言語」として定義される。表面構文は少数の正規形に絞り、型は局所推論
+ 境界明示、標準 API は核機能に集中。LLM との相性は構文単体ではなく、型制約・診断・
整形・実行・自己修正ループまで含めて最適化する。

**Status**: Freeze ready. 大きな構文変更は不要。
