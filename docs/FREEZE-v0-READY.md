# v0 Canonical Surface - Freeze Ready

**Date**: 2026-03-25  
**Status**: ✅ Ready — v0 全機能実装完了（io/fs 含む）。設計文書の整合済み。

## Summary

arukellt v0 の canonical surface は構文整理フェーズを通過し、**コンパイラ実装が大きく進展した**。

主要な v0 機能はすべて実装済み。io/fs (fs_read_file, fs_write_file) を含め、142/147 fixture pass。
freeze は全要件の設計が完了し、各要件について選択肢比較と根拠が揃った段階で宣言する。

> **実装状況**: コンパイラは v0 機能の大部分を実装。
> 142/147 fixture テスト pass（5 skip はモジュールヘルパーファイル）。i32/i64/f64/bool/String/struct/enum(payload)/
> Option/Result/?演算子/クロージャ/高階関数/match(payload binding)/タプル/Box/for ループ/文字列補間 が
> end-to-end 動作。
> 詳細は [`docs/process/v0-scope.md`](process/v0-scope.md) 参照。

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
- ✅ for あり（限定版: `0..n`, `values(v)`）
- ✅ メソッド構文なし（v1）
- ✅ 演算子オーバーロードなし
- ✅ `Result<T, E>` 中心のエラー処理
- ✅ `?` は v0 では型一致時のみ（自動変換なし）

### 型システム

- ✅ 現行実装は linear memory ベース（Wasm GC 型は将来対応）
- ✅ 参照型: struct/enum/String/Vec は linear memory 上のポインタ
- ✅ 代入・引数渡し = ポインタコピー（オブジェクト共有）
- ✅ mutation: Vec のみ in-place 変更可能（struct は immutable）
- ✅ generic: `<T>` 記法、ネスト禁止、runtime は i32 統一

### API

- ✅ Prelude: Option/Result/String/Vec, Some/None/Ok/Err, len/clone/unwrap/panic
- ✅ Vec 操作: Vec_new_i32/push/pop/get/set/sort_i32/map_i32_i32/filter_i32/fold_i32_i32（全て裸関数）
- ✅ String: concat/split/join/slice（不変、新値を返す）
- ✅ Option: unwrap/unwrap_or/is_some/is_none
- 🔲 I/O: io.Caps 経由のみ（capability-based）— 未実装

### Documentation

- ✅ v1 機能を syntax-v1-preview.md に分離
- ⚠️ 文書間の矛盾を解消中（Wasm GC vs linear memory の記述整合）
- ✅ コードサンプルは型検査可能（io/fs 含む）

## Completed Canonicalization

### Phase 1: 構文レベルの矛盾解消

- impl/メソッド構文を v0 から削除
- generics 記法を `<T>` に統一
- 文字列リテラル型を String に統一
- v1 機能を別ファイルに分離

### Phase 2: API/セマンティクス境界の固定化

- clone を v0 正式採用（deep clone — 全参照型を再帰的に複製）
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
| **P1** | `for` ループ（限定版） | ✅ v0 実装済み |
| **P2** | 文字列補間 `f"..."` | ✅ v0 実装済み |
| **P3** | trait / iterator / 高階走査 | 高 |
| **P4** | `impl` / メソッド構文 | 中（trait 後） |
| **P5** | 演算子オーバーロード | 中（trait 後） |
| — | match guard / or-pattern / struct pattern | 低 |
| — | `?` エラー型自動変換（`From` trait） | trait 後 |
| — | struct field update | 低 |

**段階的導入**: ~~P1・P2 は trait なしで導入可能~~ → ✅ v0 実装済み。P3 導入前に組み込み反復プロトコルで橋渡し。

## v0 / v1 境界表

「v0 では不可、v1 では可」を 1 箇所に集約する。

| 機能 | v0 | v1 | 備考 |
|------|:--:|:--:|------|
| `for` ループ（限定版） | ✅ | — | `0..n`, `values(v)` のみ |
| 文字列補間 `f"..."` | ✅ | — | プリミティブ型のみ。struct/enum は Display 待ち |
| `break` / `continue` | ✅ | — | while, for 両方で動作 |
| `while` ループ | ✅ | — | |
| match（payload binding） | ✅ | — | |
| `?` 演算子 | ✅ | — | 同一エラー型のみ |
| クロージャ（キャプチャなし） | ✅ | — | |
| 固定長配列 `[T; N]` | ✅ | — | |
| モジュール（import / pub） | ✅ | — | |
| trait / interface | ❌ | P3 | coherence/orphan rule と同時に導入 |
| `impl` / メソッド構文 | ❌ | P4 | trait 後に導入 |
| 演算子オーバーロード | ❌ | P5 | trait ベースでのみ |
| 一般 Iterator（`for x in expr`） | ❌ | P3 | Iterator trait が必要 |
| `?` エラー型自動変換 | ❌ | P3 | From trait が必要 |
| ユーザー定義 generic struct | ❌ | v1+ | |
| ネスト generics（`Vec<Vec<T>>`） | ❌ | v1+ | |
| trait bounds（`T: Eq`） | ❌ | v1+ | trait 後 |
| `extern "C"` / FFI | ❌ | v1+ | |
| Wasm GC バックエンド | ❌ | v1+ | 設計は GC 前提、実装は linear memory |

### v0 の non-goals（明示的禁止）

- **trait** を中途半端に入れない（coherence/orphan rule 未定義で導入禁止）
- **operator overload** を trait 前に先行しない
- **borrow checker / ownership** を言語仕様に逆輸入しない
- **native 専用機能** を追加しない
- T1 (linear memory) の実装詳細を T3 (Wasm GC) の意味論に昇格させない
- `mem.__alloc` / `mem.__free` をユーザーに露出しない

## Next Steps（freeze 前に必要な作業）

以下が完了して初めて freeze を宣言できる:

1. ~~**io/fs 実装**~~ ✅ 完了
   - ~~capability-based I/O（fs_read_file, fs_write_file）~~ ✅
   - WASI p1 上の簡易実装（preopened dir fd 3 使用） ✅

2. ~~**設計文書の最終整合**~~ ✅ 完了
   - ~~Wasm GC vs linear memory の記述統一~~ ✅
   - ~~v0-scope.md と本文書の status 一致~~ ✅
   - ~~quickstart.md のサンプル検証完了~~ ✅

3. ~~**診断設計の最終化**~~ ✅ 完了
   - ~~fix-it hint の実装範囲確定~~ ✅ v1 で実装予定と明記
   - ~~LLM 向け診断パターンの検証~~ ✅ 設計済み・v1 で実装予定

## Conclusion

v0 は「LLM が安定生成・修正しやすい canonical surface を持つ
Wasm-first 言語」として定義される。表面構文は少数の正規形に絞り、型は局所推論
- 境界明示、標準 API は核機能に集中。

**Status**: コンパイラ実装は 142/147 fixture pass（5 skip はモジュールヘルパーファイル）。v0 freeze ready。
