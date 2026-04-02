# v0 スコープ定義

## 概要

arukellt v0 は「LLM フレンドリな言語」の最小限の実装。
コア機能に集中し、高度な機能は v1 以降に持ち越す。

> **現在の実装確認は [`docs/current-state.md`](../current-state.md) を参照。**
> 本文書はスコープ定義（何を含めるか）であり、実装完了の宣言ではない。

凡例: ✅ = end-to-end 動作確認済み, ⚠️ = 部分実装, 🔲 = 設計済み・未実装

---

## v0 に含めるもの

### 言語機能

| 機能 | 状態 | 備考 |
|------|------|------|
| プリミティブ型（i32, bool） | ✅ | 完全動作 |
| プリミティブ型（i64, f64） | ✅ | リテラル・演算・変換ヘルパー・print 動作 |
| プリミティブ型（f32, char） | ⚠️ | パース済み。f32 は f64 経由で動作、char は i32 として扱う |
| String | ✅ | リテラル、String_from、eq、concat、split、join、slice、println 動作 |
| struct | ✅ | 定義・初期化・フィールドアクセス・ネスト動作 |
| enum（unit バリアント） | ✅ | 整数タグとして動作、match 対応 |
| enum（payload バリアント） | ✅ | Some(T), Ok(T), Err(E) 等のペイロード動作。match でバインド可能 |
| パターンマッチ（match） | ✅ | int/bool/enum(unit)/enum(payload)/wildcard/変数バインド動作 |
| ジェネリック関数（制限付き） | ✅ | パース・型検査・monomorphization（runtime i32 統一）動作 |
| Option\<T\>, Result\<T, E\> | ✅ | Some/None/Ok/Err ペイロード付き。unwrap/is_some/is_none 動作 |
| if/else, while, loop | ✅ | 文・式両方、break/continue 対応 |
| ? 演算子（エラー伝播） | ✅ | 型検査・lowering・コード生成動作 |
| モジュールシステム | ⚠️ | import 構文パース・基本的なモジュール読込動作。循環検出あり |
| クロージャ | ✅ | lambda lifting + capture injection で動作 |
| 高階関数 | ✅ | map_i32_i32, filter_i32, fold_i32_i32 動作 |
| 基本演算子（算術、比較、論理） | ✅ | i32/i64/f64 で動作。短絡評価・型昇格対応 |
| タプル | ✅ | タプルリテラル・分配束縛動作 |
| Box\<T\> | ✅ | Box_new / unbox でヒープ割当動作 |
| for ループ（限定版） | ✅ | 範囲 `0..n` と Vec 走査 `values(v)` |
| 文字列補間 `f"..."` | ✅ | プリミティブ型のみ。struct/enum は P3 (Display) 待ち |

### 標準ライブラリ

| モジュール | 状態 | 備考 |
|-----------|------|------|
| println / print / eprintln | ✅ | WASI fd_write 経由。i32/i64/f64/bool/String 自動対応 |
| i32_to_string / i64_to_string / f64_to_string / bool_to_string | ✅ | Wasm ヘルパー関数として実装 |
| String_from / eq | ✅ | 文字列生成・比較 |
| concat / slice / split / join | ✅ | 文字列操作。Wasm ヘルパーとして実装 |
| core/option | ✅ | unwrap, unwrap_or, is_some, is_none 動作 |
| core/result | ✅ | unwrap 動作。? 演算子で伝播可能 |
| collections/vec | ✅ | Vec_new_i32, push, pop, get, set, len, sort_i32, contains_i32, reverse_i32, remove_i32 動作 |
| Vec 高階関数 | ✅ | map/filter/fold: i32, i64, f64, String 全型対応 |
| 数学関数 | ✅ | sqrt, abs, min, max 動作 |
| parse_i32 | ✅ | 文字列→数値変換 |
| parse_i64 / parse_f64 | ✅ | 文字列→数値変換（直接返し、エラー時 0/0.0） |
| core/mem | 🔲 | 設計済み・未実装 |
| io/fs | ✅ | fs_read_file, fs_write_file 動作（WASI p1） |
| io/clock | ✅ | clock_now() — WASI clock_time_get 経由で動作 |
| io/random | ✅ | random_i32() — WASI random_get 経由で動作 |

### ツールチェイン

| ツール | 状態 | 備考 |
|--------|------|------|
| arukellt compile | ✅ | .wasm ファイル出力 |
| arukellt run | ✅ | wasmtime 組み込みで実行 |
| arukellt check | ✅ | パース + 名前解決 + 型検査 |
| 複数エラーの一括報告 | ✅ | DiagnosticSink + ariadne |
| Wasm バイナリ出力 | ✅ | WASI Preview 1 互換 |
| WASI p1 サポート | ✅ | fd_write のみ。linear memory ターゲット |
| WASI p2 サポート（Component Model / WIT） | 🔲 | 設計済み・未実装 |

---

## v0 に含めないもの

### 言語機能

**注**: `break` / `continue` は v0 に含まれている。
**注**: for ループと文字列補間は当初 v1 予定だったが、v0 で実装済み。

| 機能 | 理由 | 予定 | v1 優先度 |
|------|------|------|----------|
| ~~for 構文（限定版）~~ | ~~trait 不要。範囲 `0..n` + Vec 走査 `values(v)`~~ | ~~v1~~ | ~~**P1**~~ ✅ v0 実装済み |
| ~~文字列補間 `f"..."`~~ | ~~concat ネスト解消。プリミティブ型のみ~~ | ~~v1~~ | ~~**P2**~~ ✅ v0 実装済み |
| trait / interface | ~~LLM が壊しやすい解決規則。組み込み反復で先に橋渡し~~ | ~~v1~~ | ~~P3~~ ✅ v1 実装済み（静的ディスパッチ） |
| impl / メソッド構文 | ~~trait 後に導入~~ | ~~v1~~ | ~~P4~~ ✅ v1 実装済み |
| 演算子オーバーロード | ~~trait が必要~~ | ~~v1~~ | ~~P5~~ ✅ v1 実装済み（impl メソッド経由） |
| ネストしたジェネリクス | コード爆発を防ぐ | v1 | 未定 |
| ユーザー定義 generic struct | v0 の複雑さを抑える | v1 | 未定 |
| async/await | 非同期ランタイム設計が必要 | v2 | — |
| マクロ | 設計が必要 | v2 | — |

### 標準ライブラリ

| モジュール | 理由 | 予定 |
|-----------|------|------|
| iter | Iterator trait が必要 | v1 |
| collections/hashmap | Eq, Hash trait が必要 | v1 |
| fmt | Display trait が必要 | v1 |
| net | async 設計前には入れない | v2 |

### ツールチェイン

| ツール | 理由 | 予定 |
|--------|------|------|
| LLVM バックエンド | Wasm 優先 | v1 |
| LSP サーバー | v0 後に追加 | v1 |
| インクリメンタルコンパイル | v0 後に追加 | v1 |

---

## 制限事項

### ジェネリクスの制限

- 型パラメータは 2 個まで
- ネスト禁止: ~~`Vec[Vec[T]]` は使用不可~~ v1 M8 で対応済み
- generic struct はライブラリ提供のみ

### クロージャの制限

- キャプチャは値コピーのみ
- mutable capture は `ref` 経由

### エラー処理の制限

- ? 演算子はエラー型が一致する場合のみ
- From trait による自動変換は v1 以降

---

## 成功基準

v0 完成の条件:

1. `scripts/run/verify-harness.sh` が成功
2. 以下のサンプルがコンパイル・実行可能:
   - Hello World ✅
   - 数値計算（フィボナッチ、素数判定） ✅
   - 構造体・enum の基本操作 ✅
   - エラー処理（Result の連鎖） ✅
   - ファイル読み書き — **io/fs 実装済み**（fs_read_file, fs_write_file）
3. Wasm バイナリサイズが許容範囲内
   - Hello World: 5KB 以下
4. 複数エラーの一括報告が機能 ✅

> **現在の到達度**: 182 fixture テスト pass。
> Hello World、数値計算、構造体、enum payload、Option/Result、クロージャ、
> 高階関数（i32/i64/f64/String）、パターンマッチ、? 演算子、for ループ、文字列補間がすべて end-to-end 動作。
> clock_now, random_i32, parse_i64, parse_f64, contains_i32, reverse_i32, remove_i32 も動作確認済み。

---

## 関連

- ADR-002: メモリモデル（Wasm GC）
- ADR-003: ジェネリクス戦略
- ADR-004: trait 戦略（v0 では trait なし）
- `docs/stdlib/README.md`: 標準ライブラリ追加順序

---

## v0 凍結・v1 実装済み機能

> **v0 は凍結済み。** 以下の v1 機能が実装されている:

| 機能 | マイルストーン | 備考 |
|------|--------------|------|
| `any_i32`, `find_i32` HOFs | M3 | `Vec<i32>` 向け高階関数 |
| trait 定義 (`trait Name { ... }`) | M4 | 静的ディスパッチのみ |
| trait 実装 (`impl Trait for Type { ... }`) | M4 | 名前マングリングで脱糖 |
| inherent impl (`impl Type { ... }`) | M4/M5 | trait なしのメソッド定義 |
| メソッド構文 (`obj.method(args)`) | M4 | `Type__method(obj, args)` に脱糖 |
| 演算子オーバーロード (`+`, `-`, `*`, `/`, `==`, `<` 等) | M6 | `Type__add(a, b)` 等に変換 |
