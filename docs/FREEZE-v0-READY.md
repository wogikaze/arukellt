# v0 Canonical Surface - Freeze Ready

**Date**: 2026-03-24  
**Status**: ⚠️ Not ready — 設計未完了

## Summary

arukellt v0 の canonical surface は構文整理フェーズを通過したが、**設計はまだ完了していない**。

未決定・未設計の要件が残っており、比較分析や段取り（設計手順）が確立されていない項目がある。
「freeze」は全要件の設計が完了し、各要件について選択肢比較と根拠が揃った段階で宣言する。

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

## What's NOT in v0 (v1 Preview — 優先度順)

**注**: `break` / `continue` は v0 に含まれている。v1 項目ではない。

| 優先度 | 機能 | 設計コスト |
|--------|------|-----------|
| **P1** | `for` ループ（限定版: `0..n`, `values(v)`） | 低（trait 不要） |
| **P2** | 文字列補間 `f"..."` | 低 |
| **P3** | trait / iterator / 高階走査 | 高 |
| **P4** | `impl` / メソッド構文 | 中（trait 後） |
| **P5** | 演算子オーバーロード | 中（trait 後） |
| — | match guard / or-pattern / struct pattern | 低 |
| — | `?` エラー型自動変換（`From` trait） | trait 後 |
| — | struct field update | 低 |

**段階的導入**: P1・P2 は trait なしで導入可能。P3 導入前に組み込み反復プロトコルで橋渡し。

## Next Steps（freeze 前に必要な作業）

以下が完了して初めて freeze を宣言できる:

1. **要件の洗い出し完了**
   - 未設計の要件を列挙し、各要件に対して選択肢比較と根拠を揃える
   - 例: wasm32 コンパイルターゲット（2026-03-25 追加）の設計詳細

2. **各要件の比較・段取り設計**
   - 選択肢の列挙と比較（benchmark / 分析）
   - 決定基準と根拠の文書化（ADR または設計ノート）

3. **診断設計**
   - 構造化診断メッセージ
   - expected/actual 表示
   - fix-it hint

4. **stdlib 完全性**
   - v0 で必要な Vec 操作の全列挙
   - String 操作の正規形確定

5. **実装同期**
   - コンパイラで v0 制限を強制
   - 禁止構文に対する明確なエラーメッセージ

## Conclusion

v0 は「Python 風」ではなく「LLM が安定生成・修正しやすい canonical surface を持つ
Wasm GC 前提言語」として定義される。表面構文は少数の正規形に絞り、型は局所推論
+ 境界明示、標準 API は核機能に集中。LLM との相性は構文単体ではなく、型制約・診断・
整形・実行・自己修正ループまで含めて最適化する。

**Status**: 構文整理は進んでいるが、まだ freeze できる状態ではない。
